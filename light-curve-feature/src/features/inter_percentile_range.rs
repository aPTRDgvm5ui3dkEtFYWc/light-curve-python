use crate::evaluator::*;
use crate::statistics::Statistics;

/// Inter-percentile range
///
/// $$
/// \mathrm{inter-percetile range} \equiv Q(1 - p) - Q(p),
/// $$
/// where $Q(p)$ is the $p$th quantile of the magnitude distribution.
///
/// Special cases are [the interquartile range](https://en.wikipedia.org/wiki/Interquartile_range)
/// which is inter-percentile range for $p = 0.25$ and
/// [the interdecile range](https://en.wikipedia.org/wiki/Interdecile_range) which is
/// inter-percentile range for $p = 0.1$.
///
/// - Depends on: **magnitude**
/// - Minimum number of observations: **1**
/// - Number of features: **1**
#[derive(Clone)]
pub struct InterPercentileRange {
    quantile: f32,
    name: String,
}

lazy_info!(
    INTER_PERCENTILE_RANGE_INFO,
    size: 1,
    min_ts_length: 1,
    t_required: false,
    m_required: true,
    w_required: false,
    sorting_required: false,
);

impl InterPercentileRange {
    pub fn new(quantile: f32) -> Self {
        assert!(
            (quantile > 0.0) && (quantile < 0.5),
            "Quanitle should be in range (0.0, 0.5)"
        );
        Self {
            quantile,
            name: format!("inter_percentile_range_{:.0}", 100.0 * quantile),
        }
    }
}

impl Default for InterPercentileRange {
    fn default() -> Self {
        Self::new(0.25)
    }
}

impl<T> FeatureEvaluator<T> for InterPercentileRange
where
    T: Float,
{
    fn eval(&self, ts: &mut TimeSeries<T>) -> Result<Vec<T>, EvaluatorError> {
        self.check_ts_length(ts)?;
        let q = [self.quantile, 1.0 - self.quantile];
        let ppf = ts.m.get_sorted().ppf_many_from_sorted(&q[..]);
        let value = ppf[1] - ppf[0];
        Ok(vec![value])
    }

    fn get_info(&self) -> &EvaluatorInfo {
        &INTER_PERCENTILE_RANGE_INFO
    }

    fn get_names(&self) -> Vec<&str> {
        vec![self.name.as_str()]
    }
}

#[cfg(test)]
#[allow(clippy::unreadable_literal)]
#[allow(clippy::excessive_precision)]
mod tests {
    use super::*;
    use crate::tests::*;

    eval_info_test!(inter_percentile_range_info, InterPercentileRange::default());

    feature_test!(
        inter_percentile_range,
        [
            Box::new(InterPercentileRange::default()),
            Box::new(InterPercentileRange::new(0.25)), // should be the same
            Box::new(InterPercentileRange::new(0.1)),
        ],
        [50.0, 50.0, 80.0],
        linspace(0.0, 99.0, 100),
    );
}
