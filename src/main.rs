use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::process::ExitCode;

const CSV_HEADER: &str = "timestamp,value";
const USAGE: &str = "Usage: sensor-stats <file>";

#[derive(Debug)]
struct Stats {
    count: usize,
    min: f64,
    max: f64,
    sum: f64,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("sensor-stats: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args_os().skip(1);
    let (Some(path), None) = (args.next(), args.next()) else {
        return Err(USAGE.to_owned());
    };

    let path = Path::new(&path);
    let file =
        File::open(path).map_err(|error| format!("cannot open {}: {error}", path.display()))?;
    let stats =
        read_stats(BufReader::new(file)).map_err(|error| format!("{}: {error}", path.display()))?;
    let mean = stats.sum / stats.count as f64;
    let mut output = io::stdout().lock();

    writeln!(
        output,
        "Count: {}\nMin:   {:.2}\nMax:   {:.2}\nMean:  {mean:.2}",
        stats.count, stats.min, stats.max,
    )
    .and_then(|()| output.flush())
    .map_err(|error| format!("cannot write output: {error}"))
}

fn read_stats(reader: impl BufRead) -> Result<Stats, String> {
    let mut stats = Stats {
        count: 0,
        min: f64::INFINITY,
        max: f64::NEG_INFINITY,
        sum: 0.0,
    };
    let mut has_csv_header = None;

    for (index, line) in reader.lines().enumerate() {
        let number = index + 1;
        let line = line.map_err(|error| format!("line {number}: cannot read input: {error}"))?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if has_csv_header.is_none() {
            let is_header = line == CSV_HEADER;
            has_csv_header = Some(is_header);
            if is_header {
                continue;
            }
        }

        let field = if has_csv_header == Some(true) {
            let (_, value) = line
                .split_once(',')
                .filter(|(timestamp, value)| !timestamp.trim().is_empty() && !value.contains(','))
                .ok_or_else(|| format!("line {number}: expected timestamp,value"))?;
            value.trim()
        } else {
            line
        };
        let value = field
            .parse::<f64>()
            .map_err(|_| format!("line {number}: invalid sensor reading {field:?}"))?;
        if !value.is_finite() {
            return Err(format!("line {number}: sensor reading must be finite"));
        }

        stats.count += 1;
        stats.min = stats.min.min(value);
        stats.max = stats.max.max(value);
        // ponytail: a running f64 sum can overflow. Use a range-safe mean if larger inputs are needed.
        stats.sum += value;
        if !stats.sum.is_finite() {
            return Err(format!("line {number}: running sum is out of range"));
        }
    }

    if stats.count == 0 {
        return Err("no sensor readings".to_owned());
    }
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::read_stats;

    #[test]
    fn summarizes_plain_text_with_negative_values() {
        let stats = read_stats(&b"-5.5\r\n\r\n -3 \r\n-2\r\n"[..]).unwrap();
        assert_eq!(
            (stats.count, stats.min, stats.max, stats.sum),
            (3, -5.5, -2.0, -10.5),
        );
    }

    #[test]
    fn reads_csv_after_blank_lines() {
        let stats = read_stats(&b"\ntimestamp,value\n0,23.5\n1,24\n2,25.5\n"[..]).unwrap();
        assert_eq!(
            (stats.count, stats.min, stats.max, stats.sum),
            (3, 23.5, 25.5, 73.0),
        );
    }

    #[test]
    fn reports_the_physical_line_of_a_bad_reading() {
        let error = read_stats(&b"timestamp,value\n0,23.5\n\n1,broken\n"[..]).unwrap_err();
        assert_eq!(error, "line 4: invalid sensor reading \"broken\"");
    }

    #[test]
    fn rejects_invalid_csv_rows() {
        for row in ["23.5", ",23.5", "0,23.5,24"] {
            let input = format!("timestamp,value\n{row}\n");
            assert_eq!(
                read_stats(input.as_bytes()).unwrap_err(),
                "line 2: expected timestamp,value",
            );
        }
    }

    #[test]
    fn rejects_input_without_samples() {
        for input in ["", "  \n", "timestamp,value\n\n"] {
            assert_eq!(
                read_stats(input.as_bytes()).unwrap_err(),
                "no sensor readings",
            );
        }
    }

    #[test]
    fn rejects_nonfinite_readings_and_sum_overflow() {
        for input in ["NaN", "inf", "1e999"] {
            assert_eq!(
                read_stats(input.as_bytes()).unwrap_err(),
                "line 1: sensor reading must be finite",
            );
        }
        assert_eq!(
            read_stats(&b"1e308\n1e308\n"[..]).unwrap_err(),
            "line 2: running sum is out of range",
        );
    }
}
