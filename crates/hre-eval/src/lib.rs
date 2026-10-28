pub mod ablation;
pub mod baselines;
pub mod generator_b;
pub mod metrics;
pub mod runner;

pub use ablation::{AblationConfig, AblationEvaluator, AblationResult};
pub use baselines::{KeywordBaseline, KeywordBaselineComparison};
pub use generator_b::{GeneratedCase, GeneratorB};
pub use metrics::{
    compute_cer, compute_cohen_kappa, compute_percentiles, compute_wer, ClassificationStats,
};
pub use runner::{EvalRunner, FullEvaluationReport};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_and_cer_wer() {
        assert_eq!(metrics::levenshtein_distance("kitten", "sitting"), 3);
        assert_eq!(metrics::compute_cer("abc", "abc"), Some(0.0));
        assert_eq!(
            metrics::compute_wer("hello world", "hello world"),
            Some(0.0)
        );
        assert_eq!(
            metrics::compute_wer("hello world", "hello there world"),
            Some(0.5)
        );
    }

    #[test]
    fn test_cohen_kappa() {
        let r1 = vec![true, true, false, false];
        let r2 = vec![true, true, false, false];
        assert_eq!(metrics::compute_cohen_kappa(&r1, &r2), Some(1.0));
    }

    #[test]
    fn test_generator_b_coverage() {
        let gen = GeneratorB::new();
        let cases = gen.generate_all_cases();
        assert_eq!(cases.len(), 16);
        assert!(cases.iter().any(|c| c.scenario_class == "polite_stalker"));
        assert!(cases.iter().any(|c| c.scenario_class == "heavy_benign"));
        assert!(cases.iter().any(|c| c.scenario_class == "night_caller"));
    }
}
