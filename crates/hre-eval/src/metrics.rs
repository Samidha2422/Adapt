use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClassificationStats {
    pub tp: usize,
    pub fp: usize,
    pub tn: usize,
    pub fn_count: usize,
    pub support: usize,
}

impl ClassificationStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, is_positive_ground_truth: bool, is_positive_prediction: bool) {
        self.support += 1;
        match (is_positive_ground_truth, is_positive_prediction) {
            (true, true) => self.tp += 1,
            (false, true) => self.fp += 1,
            (false, false) => self.tn += 1,
            (true, false) => self.fn_count += 1,
        }
    }

    pub fn precision(&self) -> Option<f64> {
        if self.tp + self.fp == 0 {
            None
        } else {
            Some(self.tp as f64 / (self.tp + self.fp) as f64)
        }
    }

    pub fn recall(&self) -> Option<f64> {
        if self.tp + self.fn_count == 0 {
            None
        } else {
            Some(self.tp as f64 / (self.tp + self.fn_count) as f64)
        }
    }

    pub fn f1(&self) -> Option<f64> {
        let p = self.precision()?;
        let r = self.recall()?;
        if p + r == 0.0 {
            Some(0.0)
        } else {
            Some(2.0 * p * r / (p + r))
        }
    }

    pub fn fpr(&self) -> Option<f64> {
        if self.fp + self.tn == 0 {
            None
        } else {
            Some(self.fp as f64 / (self.fp + self.tn) as f64)
        }
    }

    pub fn accuracy(&self) -> Option<f64> {
        let total = self.tp + self.fp + self.tn + self.fn_count;
        if total == 0 {
            None
        } else {
            Some((self.tp + self.tn) as f64 / total as f64)
        }
    }
}

/// Computes standard Levenshtein distance between two character sequences.
pub fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    let mut dp = vec![vec![0usize; n + 1]; m + 1];

    for (i, row) in dp.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, item) in dp[0].iter_mut().enumerate() {
        *item = j;
    }

    for i in 1..=m {
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }

    dp[m][n]
}

/// Computes Character Error Rate (CER): Levenshtein(ref, hyp) / len(ref)
pub fn compute_cer(reference: &str, hypothesis: &str) -> Option<f64> {
    let ref_len = reference.chars().count();
    if ref_len == 0 {
        if hypothesis.is_empty() {
            Some(0.0)
        } else {
            None // Undefined / division by zero on empty reference
        }
    } else {
        let dist = levenshtein_distance(reference, hypothesis);
        Some(dist as f64 / ref_len as f64)
    }
}

/// Computes Word Error Rate (WER) using word tokens.
pub fn compute_wer(reference: &str, hypothesis: &str) -> Option<f64> {
    let ref_words: Vec<&str> = reference.split_whitespace().collect();
    let hyp_words: Vec<&str> = hypothesis.split_whitespace().collect();
    let m = ref_words.len();
    let n = hyp_words.len();

    if m == 0 {
        if n == 0 {
            return Some(0.0);
        } else {
            return None;
        }
    }

    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for (i, row) in dp.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, item) in dp[0].iter_mut().enumerate() {
        *item = j;
    }

    for i in 1..=m {
        for j in 1..=n {
            let cost = if ref_words[i - 1] == hyp_words[j - 1] {
                0
            } else {
                1
            };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }

    Some(dp[m][n] as f64 / m as f64)
}

/// Computes Cohen's Kappa inter-annotator agreement between two boolean rating series.
/// Contingency table:
///              Rater2 True | Rater2 False
/// Rater1 True       a      |      b
/// Rater1 False      c      |      d
pub fn compute_cohen_kappa(rater1: &[bool], rater2: &[bool]) -> Option<f64> {
    if rater1.len() != rater2.len() || rater1.is_empty() {
        return None;
    }

    let n = rater1.len() as f64;
    let mut a = 0.0;
    let mut b = 0.0;
    let mut c = 0.0;
    let mut d = 0.0;

    for (&r1, &r2) in rater1.iter().zip(rater2.iter()) {
        match (r1, r2) {
            (true, true) => a += 1.0,
            (true, false) => b += 1.0,
            (false, true) => c += 1.0,
            (false, false) => d += 1.0,
        }
    }

    let po = (a + d) / n;
    let pe = (((a + b) * (a + c)) + ((c + d) * (b + d))) / (n * n);

    if (1.0 - pe).abs() < f64::EPSILON {
        Some(1.0) // Perfect agreement
    } else {
        Some((po - pe) / (1.0 - pe))
    }
}

/// Computes percentiles (p50, p95, p99) from a slice of durations/latencies in milliseconds.
pub fn compute_percentiles(mut values: Vec<f64>) -> (f64, f64, f64) {
    if values.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let len = values.len();
    let p50_idx = ((len as f64 * 0.50).round() as usize).min(len - 1);
    let p95_idx = ((len as f64 * 0.95).round() as usize).min(len - 1);
    let p99_idx = ((len as f64 * 0.99).round() as usize).min(len - 1);
    (values[p50_idx], values[p95_idx], values[p99_idx])
}
