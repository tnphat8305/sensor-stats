# sensor-stats

`sensor-stats` reports the count, minimum, maximum, and arithmetic mean of
sensor readings. It uses Rust's standard library.

## Run the demo

From the repository root, with Rust and Cargo installed:

```sh
cargo run --quiet -- sample.csv
```

```text
Count: 3
Min:   23.50
Max:   25.50
Mean:  24.33
```

The three readings sum to 73. Dividing by 3 gives 24.33 at the displayed precision.

## Input format

Files use UTF-8. Supply one number per line or start the file with a
`timestamp,value` header. CSV rows use exactly two unquoted fields. The
timestamp is a non-empty label for the sample. The CLI calculates stats from
the value column. Blank lines are skipped in either format.

```text
timestamp,value
2026-11-23T08:00:00Z,23.5
2026-11-23T08:00:01Z,24.0
```

A bad row reports the path and the physical line number on stderr. Empty files,
header-only CSV, nonfinite readings, and running sums outside the finite
`f64` range also produce a nonzero exit status. The calculation uses
floating-point arithmetic and displays two decimal places.

## How it works

`BufReader` reads the file one line at a time. For each reading, the program
updates four values in memory. It keeps the count, minimum, maximum, and sum,
then divides the sum by the count at the end. The scan takes one pass and
stores only a line of input and the running totals.

## Run the tests

```sh
cargo test
```
