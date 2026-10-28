//! Evaluation-only phase schedules; no training imports, fixtures, labels or generated inputs.
use hre_core::Pseudonymizer;
use hre_store::{EventKind, StoredEvent};

pub struct Event {
    pub stored: StoredEvent,
    pub text: Option<&'static str>,
}

pub struct Conversation {
    pub id: String,
    pub class: &'static str,
    pub positive: bool,
    pub heavy_benign: bool,
    pub calls: bool,
    pub events: Vec<Event>,
}

const CLASSES: &[&str] = &[
    "normal",
    "heavy_benign_couple",
    "heavy_benign_friends",
    "heavy_benign_fandom",
    "heavy_benign_family",
    "polite_stalker",
    "escalating_stalker",
    "post_block_contacter",
    "pile_on",
    "bully",
    "sextortion",
    "silent_caller",
    "night_caller",
    "post_block_caller",
    "new_number_caller",
    "heavy_benign_calls",
    "noisy_ambiguous",
];

pub fn generate(seed: u64, variants: usize) -> Result<Vec<Conversation>, super::EvalError> {
    let ids = Pseudonymizer::new(b"hre-eval-public-synthetic-key-v1-000000")?;
    let mut conversations = Vec::new();
    for (class_index, &class) in CLASSES.iter().enumerate() {
        for variant in 0..variants {
            // Phase lengths and interleaved replies are independent of the engine's decision thresholds.
            let phase = (seed.wrapping_add((variant * 19 + class_index * 7) as u64) % 29) as usize;
            let heavy = class.starts_with("heavy_benign");
            let calls = class.contains("caller") || class == "heavy_benign_calls";
            let positive = !heavy && !matches!(class, "normal" | "noisy_ambiguous");
            let count = match class {
                "normal" => 5 + phase % 7,
                "pile_on" => 11 + phase % 5,
                "noisy_ambiguous" => 13 + phase % 11,
                "bully" | "sextortion" => 7 + phase % 9,
                _ => 83 + phase * 2,
            };
            let id = format!("b-{class}-{variant}");
            let sender = ids.subject("eval", &format!("{id}-sender"))?.0;
            let recipient = ids.subject("eval", &format!("{id}-recipient"))?.0;
            let mut events = Vec::new();
            let mut time = 20_833 * 86_400_000_i64
                + if class == "night_caller" {
                    23 * 3_600_000
                } else {
                    12 * 3_600_000
                };
            for i in 0..count {
                let blocked = class.starts_with("post_block") && i >= count / 3;
                let gap = if class == "pile_on" {
                    2700
                } else if class == "night_caller" {
                    91_000
                } else {
                    [13_000, 71_000, 193_000, 29_000, 307_000][(i + phase) % 5]
                };
                time += gap;
                if blocked && i == count / 3 {
                    events.push(Event {
                        text: None,
                        stored: StoredEvent {
                            event_id: format!("{id}-refusal"),
                            sender: sender.clone(),
                            recipient: recipient.clone(),
                            timestamp_ms: time - 1,
                            kind: EventKind::Refusal,
                            channel: "block".into(),
                            simulated: true,
                            ..StoredEvent::default()
                        },
                    });
                }
                let current_sender = if class == "pile_on" || class == "new_number_caller" {
                    ids.subject(
                        "eval",
                        &format!(
                            "{id}-account-{}",
                            if class == "pile_on" { i } else { i / 3 }
                        ),
                    )?
                    .0
                } else {
                    sender.clone()
                };
                let text = if calls {
                    None
                } else {
                    Some(match class {
                        "bully" => "Your work is worthless and everyone should mock you.",
                        "sextortion" => "Pay before Friday or I will leak your private photos.",
                        "escalating_stalker" if i > count * 2 / 3 => {
                            "Answer now; I will hurt you when I arrive."
                        }
                        "noisy_ambiguous" if i % 4 == 0 => {
                            "You killed it on stage, that was amazing!"
                        }
                        "heavy_benign_family" => "Dinner is ready; let us know when you get home.",
                        "heavy_benign_fandom" => "Another match update for our watch party!",
                        _ => [
                            "Hope your afternoon is going well.",
                            "Could we chat when you have a moment?",
                            "Just checking whether you saw my note.",
                        ][i % 3],
                    })
                };
                let stored = StoredEvent {
                    event_id: format!("{id}-{i:04}"),
                    sender: current_sender,
                    recipient: recipient.clone(),
                    timestamp_ms: time,
                    kind: if calls {
                        EventKind::Call
                    } else {
                        EventKind::Message
                    },
                    channel: ids.opaque("channel", &id)?,
                    is_contact: heavy,
                    recipient_initiated: heavy,
                    after_refusal: blocked,
                    answered: heavy && calls,
                    duration_s: if heavy && calls {
                        173.0 + phase as f64
                    } else {
                        0.0
                    },
                    sender_account_age_days: Some(17.0 + phase as f64),
                    simulated: true,
                    content_hash: text.map(|t| ids.opaque("content", t)).transpose()?,
                    ..StoredEvent::default()
                };
                events.push(Event { stored, text });
                if !calls
                    && (heavy
                        || (class == "normal" && i % 2 == 0)
                        || (class == "noisy_ambiguous" && i % 3 == 0))
                {
                    events.push(Event {
                        text: Some("Thanks for the update; talk soon."),
                        stored: StoredEvent {
                            event_id: format!("{id}-{i:04}-reply"),
                            sender: recipient.clone(),
                            recipient: sender.clone(),
                            timestamp_ms: time + 1,
                            channel: ids.opaque("channel", &id)?,
                            is_contact: true,
                            recipient_initiated: true,
                            simulated: true,
                            ..StoredEvent::default()
                        },
                    });
                }
            }
            conversations.push(Conversation {
                id,
                class,
                positive,
                heavy_benign: heavy,
                calls,
                events,
            });
        }
    }
    Ok(conversations)
}

pub struct TextCase {
    pub id: &'static str,
    pub language: &'static str,
    pub text: &'static str,
    pub positive: bool,
}

/// Fresh authored semantic labels; these are not classifier-derived labels or public benchmark claims.
pub fn text_cases() -> Vec<TextCase> {
    [
        (
            "courtesy",
            "en",
            "Please take your time; no reply is needed tonight.",
            false,
        ),
        (
            "threat",
            "en",
            "I will hurt you outside the library tomorrow.",
            true,
        ),
        (
            "quote_report",
            "en",
            "> I will hurt you\nThis is what I received; please help me report it.",
            false,
        ),
        (
            "negated_threat",
            "en",
            "I would never hurt you; I want this resolved peacefully.",
            false,
        ),
        (
            "substring",
            "en",
            "The Scunthorpe assignment covers a classic classification problem.",
            false,
        ),
        (
            "idiom",
            "en",
            "You killed it at rehearsal; congratulations!",
            false,
        ),
        (
            "targeted_insult",
            "en",
            "You are worthless; leave our group forever.",
            true,
        ),
        (
            "coded_exclusion",
            "en",
            "Everyone agreed you should disappear from this team.",
            true,
        ),
        (
            "coercion",
            "en",
            "Send money or I will leak your private pictures tonight.",
            true,
        ),
        (
            "privacy_consent",
            "en",
            "Please remove my phone number from the public directory.",
            false,
        ),
        (
            "zero_width",
            "en",
            "I will h\u{200b}urt you after class.",
            true,
        ),
        ("homoglyph", "en", "I will kіll you when we meet.", true),
        (
            "hinglish_refusal",
            "hi-Latn",
            "Maine mana kiya hai, please mujhe message mat karo.",
            false,
        ),
        (
            "hinglish_threat",
            "hi-Latn",
            "Jawab do warna tumhe maar dunga.",
            true,
        ),
        (
            "hindi_threat",
            "hi",
            "जवाब दो नहीं तो तुम्हें जान से मार दूंगा।",
            true,
        ),
        (
            "hindi_benign",
            "hi",
            "समय मिले तो जवाब देना, कोई जल्दी नहीं है।",
            false,
        ),
        (
            "marathi_like_benign",
            "mr-like",
            "वेळ मिळाला की कळव, घाई नाही.",
            false,
        ),
        (
            "marathi_like_threat",
            "mr-like",
            "उत्तर दे नाहीतर तुला मारून टाकीन.",
            true,
        ),
    ]
    .into_iter()
    .map(|(id, language, text, positive)| TextCase {
        id,
        language,
        text,
        positive,
    })
    .collect()
}
