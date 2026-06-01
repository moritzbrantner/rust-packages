# numbers-core

Deterministic scalar numeric summaries, quantiles, ranges, and histograms for `moritzbrantner-video-analysis`.

## Highlights

- Running scalar stats with finite/non-finite accounting
- Weighted descriptive summaries for reusable analytics building blocks
- Deterministic quantiles and quartiles over finite values
- Fixed-width histograms with optional explicit ranges
- Normalization helpers for numeric ranges

## Example

```rust,no_run
use numbers_core::{histogram, quartiles, summarize_numbers, HistogramConfig, RunningStats};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut stats = RunningStats::new();
    stats.push(1.0);
    stats.push_weighted(3.0, 2.0)?;
    stats.push(f64::NAN);

    let summary = stats.summary();
    assert_eq!(summary.count, 3);
    assert_eq!(summary.finite_count, 2);
    assert_eq!(summary.mean, Some(7.0 / 3.0));

    let quartiles = quartiles(&[1.0, 2.0, 3.0, 4.0])?;
    assert_eq!(quartiles.median, 2.5);

    let histogram = histogram(&[1.0, 2.0, 3.0, 4.0], HistogramConfig::new(2)?)?;
    assert_eq!(histogram.count, 4);
    Ok(())
}
```

## Numeric behavior

Running summaries count every input, including `NaN` and infinite values, but
only finite values contribute to min, max, sum, mean, variance, and standard
deviation. Weighted summaries require finite positive weights and report
weighted population variance.

Quantiles are computed over finite values using linear interpolation between
sorted ranks. Histograms use fixed-width bins over either an explicit range or
the derived finite-value range. Degenerate ranges are valid; all matching values
land in the last bin.

## Related crates

- `dense-data`
- `video-analysis-data`
- `video-analysis-core`
