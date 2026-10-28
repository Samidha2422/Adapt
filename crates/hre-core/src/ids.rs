use crate::CoreError;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use utoipa::ToSchema;

/// Domain-separated HMAC identity; never construct this from raw user input.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
pub struct SubjectId(pub String);
/// Directional pseudonymous pair key.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
pub struct PairKey(pub String);

/// Pseudonymizes identities without retaining the source string.
#[derive(Clone)]
pub struct Pseudonymizer {
    key: Vec<u8>,
}
impl Pseudonymizer {
    pub fn new(salt: &[u8]) -> Result<Self, CoreError> {
        if salt.len() < 32 {
            return Err(CoreError::Config("salt must have at least 32 bytes".into()));
        }
        Ok(Self { key: salt.to_vec() })
    }
    pub fn subject(&self, platform: &str, raw: &str) -> Result<SubjectId, CoreError> {
        Ok(SubjectId(self.digest(&[
            b"subject",
            platform.as_bytes(),
            raw.as_bytes(),
        ])?))
    }
    pub fn pair(&self, sender: &SubjectId, recipient: &SubjectId) -> Result<PairKey, CoreError> {
        Ok(PairKey(self.digest(&[
            b"pair",
            sender.0.as_bytes(),
            recipient.0.as_bytes(),
        ])?))
    }
    pub fn opaque(&self, domain: &str, raw: &str) -> Result<String, CoreError> {
        self.digest(&[domain.as_bytes(), raw.as_bytes()])
    }
    fn digest(&self, parts: &[&[u8]]) -> Result<String, CoreError> {
        let mut mac =
            Hmac::<Sha256>::new_from_slice(&self.key).map_err(|_| CoreError::InvalidKey)?;
        for part in parts {
            mac.update(&(part.len() as u64).to_be_bytes());
            mac.update(part);
        }
        Ok(mac
            .finalize()
            .into_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect())
    }
}

/// Compares fixed-size keyed digests, avoiding timing leaks of token contents.
pub fn secret_matches(expected: &str, supplied: &str) -> bool {
    let Ok(mut expected_mac) = Hmac::<Sha256>::new_from_slice(b"hre-token-comparison-v1") else {
        return false;
    };
    expected_mac.update(expected.as_bytes());
    let signature = expected_mac.finalize().into_bytes();
    let Ok(mut supplied_mac) = Hmac::<Sha256>::new_from_slice(b"hre-token-comparison-v1") else {
        return false;
    };
    supplied_mac.update(supplied.as_bytes());
    !expected.is_empty() && supplied_mac.verify_slice(&signature).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identities_are_stable_and_separated_by_platform_and_domain() -> Result<(), CoreError> {
        let p = Pseudonymizer::new(b"01234567890123456789012345678901")?;
        let a = p.subject("demo-chat", "alice")?;
        assert_eq!(a, p.subject("demo-chat", "alice")?);
        assert_ne!(a, p.subject("discord", "alice")?);
        assert_eq!(a.0.len(), 64);
        assert!(!a.0.contains("alice"));
        assert_ne!(p.opaque("x", "ab:c")?, p.opaque("x:ab", "c")?);
        let b = p.subject("demo-chat", "bob")?;
        assert_ne!(p.pair(&a, &b)?, p.pair(&b, &a)?);
        Ok(())
    }
    #[test]
    fn token_comparison_rejects_empty_and_wrong_tokens() {
        assert!(secret_matches("token", "token"));
        assert!(!secret_matches("token", "tokeN"));
        assert!(!secret_matches("", ""));
    }
}
