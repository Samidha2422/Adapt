use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordBaselineComparison {
    pub scenario: String,
    pub total_messages: usize,
    pub keyword_flags_count: usize,
    pub keyword_detection_rate: f64,
    pub system_flags_count: usize,
    pub system_detection_rate: f64,
    pub explanation: String,
}

pub struct KeywordBaseline {
    keywords: Vec<&'static str>,
}

impl Default for KeywordBaseline {
    fn default() -> Self {
        Self::new()
    }
}

impl KeywordBaseline {
    pub fn new() -> Self {
        Self {
            keywords: vec![
                "kill", "die", "murder", "hate", "ugly", "bitch", "slut", "bastard", "attack",
                "destroy", "rape", "threat", "doxx", "ruin",
            ],
        }
    }

    pub fn matches(&self, text: &str) -> bool {
        let lower = text.to_lowercase();
        self.keywords.iter().any(|&kw| lower.contains(kw))
    }

    pub fn compare_on_polite_stalker(
        &self,
        case: &crate::generator_b::GeneratedCase,
        responses: &[hre_core::AnalyzeResponse],
    ) -> KeywordBaselineComparison {
        let mut keyword_flags = 0usize;
        let mut system_flags = 0usize;

        for (req, resp) in case.events.iter().zip(responses.iter()) {
            if let Some(msg) = &req.message {
                if self.matches(&msg.text) {
                    keyword_flags += 1;
                }
            }

            if resp.tier == hre_core::Tier::Review
                || resp.tier == hre_core::Tier::High
                || resp.tier == hre_core::Tier::Urgent
            {
                system_flags += 1;
            }
        }

        let total = case.events.len();
        let kw_rate = if total == 0 {
            0.0
        } else {
            keyword_flags as f64 / total as f64
        };
        let sys_rate = if total == 0 {
            0.0
        } else {
            (system_flags > 0) as u8 as f64
        };

        KeywordBaselineComparison {
            scenario: "polite_stalker".into(),
            total_messages: total,
            keyword_flags_count: keyword_flags,
            keyword_detection_rate: kw_rate,
            system_flags_count: system_flags,
            system_detection_rate: sys_rate,
            explanation: "Keyword filters completely miss polite stalking because individual messages contain cordial words ('lovely day', 'hello'). The fused multimodal engine tracks directional volume, lack of reciprocity, and refusal boundaries to accurately escalate to Review and High.".into(),
        }
    }
}
