use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Confusion {
    pub tp: usize,
    pub fp: usize,
    pub tn: usize,
    #[serde(rename = "fn")]
    pub fn_: usize,
}

pub fn ratio(n: usize, d: usize) -> Option<f64> {
    (d != 0).then(|| n as f64 / d as f64)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Metrics {
    pub n: usize,
    pub confusion: Confusion,
    pub precision: Option<f64>,
    pub recall: Option<f64>,
    pub f1: Option<f64>,
    pub fpr: Option<f64>,
}

impl Metrics {
    pub fn calculate(rows: &[(bool, f64)], threshold: f64) -> Self {
        let mut c = Confusion::default();
        for &(positive, score) in rows {
            match (positive, score >= threshold) {
                (true, true) => c.tp += 1,
                (false, true) => c.fp += 1,
                (false, false) => c.tn += 1,
                (true, false) => c.fn_ += 1,
            }
        }
        Self {
            n: rows.len(),
            confusion: c,
            precision: ratio(c.tp, c.tp + c.fp),
            recall: ratio(c.tp, c.tp + c.fn_),
            f1: ratio(2 * c.tp, 2 * c.tp + c.fp + c.fn_),
            fpr: ratio(c.fp, c.fp + c.tn),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PrPoint {
    pub threshold: f64,
    pub precision: Option<f64>,
    pub recall: Option<f64>,
}

pub fn pr_curve(rows: &[(bool, f64)]) -> Vec<PrPoint> {
    let mut thresholds: Vec<_> = rows.iter().map(|r| r.1).collect();
    thresholds.extend([0.0, 1.000_001]);
    thresholds.sort_by(|a, b| b.total_cmp(a));
    thresholds.dedup();
    thresholds
        .into_iter()
        .map(|threshold| {
            let m = Metrics::calculate(rows, threshold);
            PrPoint {
                threshold,
                precision: m.precision,
                recall: m.recall,
            }
        })
        .collect()
}

pub fn percentile(values: &[f64], percentile: usize) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let rank = (sorted.len() * percentile).div_ceil(100).max(1);
    sorted.get(rank - 1).copied()
}

/// Binary Cohen's kappa. Constant equal labels have undefined chance correction.
pub fn kappa(a: &[bool], b: &[bool]) -> Result<Option<f64>, crate::EvalError> {
    if a.is_empty() || a.len() != b.len() {
        return Err(crate::EvalError::InvalidLabels);
    }
    let n = a.len() as f64;
    let pa = a.iter().filter(|&&x| x).count() as f64 / n;
    let pb = b.iter().filter(|&&x| x).count() as f64 / n;
    let expected = pa * pb + (1.0 - pa) * (1.0 - pb);
    let agreement = a.iter().zip(b).filter(|(x, y)| x == y).count() as f64 / n;
    Ok((expected < 1.0).then(|| (agreement - expected) / (1.0 - expected)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confusion_and_rates_use_actual_counts() {
        let m = Metrics::calculate(&[(true, 0.9), (true, 0.2), (false, 0.7), (false, 0.1)], 0.5);
        assert_eq!(
            m.confusion,
            Confusion {
                tp: 1,
                fp: 1,
                tn: 1,
                fn_: 1
            }
        );
        assert_eq!(
            (m.precision, m.recall, m.f1, m.fpr),
            (Some(0.5), Some(0.5), Some(0.5), Some(0.5))
        );
    }

    #[test]
    fn empty_and_missing_denominators_are_null() {
        let m = Metrics::calculate(&[], 0.5);
        assert_eq!(
            (m.precision, m.recall, m.f1, m.fpr),
            (None, None, None, None)
        );
        assert_eq!(Metrics::calculate(&[(true, 0.0)], 0.5).precision, None);
    }

    #[test]
    fn pr_curve_keeps_ties_together_and_includes_no_predictions() {
        let curve = pr_curve(&[(true, 0.5), (false, 0.5)]);
        assert_eq!(curve.len(), 3);
        assert_eq!(curve[0].precision, None);
        assert_eq!(
            (curve[1].precision, curve[1].recall),
            (Some(0.5), Some(1.0))
        );
    }

    #[test]
    fn agreement_handles_undefined_and_invalid_inputs() -> Result<(), crate::EvalError> {
        assert_eq!(kappa(&[true, false], &[true, false])?, Some(1.0));
        assert_eq!(kappa(&[true, true], &[true, true])?, None);
        assert!(kappa(&[], &[]).is_err());
        assert!(kappa(&[true], &[true, false]).is_err());
        Ok(())
    }

    #[test]
    fn nearest_rank_latency_is_defined_for_one_sample() {
        assert_eq!(percentile(&[8.0], 95), Some(8.0));
        assert_eq!(percentile(&[], 95), None);
    }
}
