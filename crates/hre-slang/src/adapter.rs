use crate::lexicon::SlangError;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

pub const ADAPTER_FEATURE_DIM: usize = 8;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdapterHead {
    pub version: u32,
    pub weights: Vec<f64>,
    pub bias: f64,
    pub val_precision: f64,
    pub created_at: i64,
}

impl Default for AdapterHead {
    fn default() -> Self {
        Self::new_initial()
    }
}

impl AdapterHead {
    pub fn new_initial() -> Self {
        Self {
            version: 1,
            weights: vec![0.1; ADAPTER_FEATURE_DIM],
            bias: -0.5,
            val_precision: 0.90,
            created_at: chrono::Utc::now().timestamp_millis(),
        }
    }

    pub fn predict(&self, features: &[f64]) -> f64 {
        let mut sum = self.bias;
        for (w, f) in self.weights.iter().zip(features.iter()) {
            sum += w * f;
        }
        sigmoid(sum)
    }
}

#[derive(Debug, Clone, Default)]
pub struct AdapterManager {
    current: Arc<RwLock<AdapterHead>>,
    history: Arc<RwLock<Vec<AdapterHead>>>,
}

impl AdapterManager {
    pub fn new() -> Self {
        Self {
            current: Arc::new(RwLock::new(AdapterHead::new_initial())),
            history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn current_head(&self) -> Result<AdapterHead, SlangError> {
        let head = self
            .current
            .read()
            .map_err(|_| SlangError::LockError("adapter lock poisoned".into()))?;
        Ok(head.clone())
    }

    /// Retrains the adapter head using mini-batch SGD with an explicit drift guard.
    /// Rejects update if precision on the frozen validation set drops by more than 2 points (0.02).
    pub fn retrain(
        &self,
        train_samples: &[([f64; ADAPTER_FEATURE_DIM], f64)],
        val_samples: &[([f64; ADAPTER_FEATURE_DIM], f64)],
    ) -> Result<AdapterHead, SlangError> {
        let old_head = self.current_head()?;

        // Perform SGD training
        let mut weights = old_head.weights.clone();
        let mut bias = old_head.bias;
        let lr = 0.2;
        let epochs = 20;

        for _ in 0..epochs {
            for (features, label) in train_samples {
                let mut z = bias;
                for (w, f) in weights.iter().zip(features.iter()) {
                    z += w * f;
                }
                let pred = sigmoid(z);
                let err = label - pred;

                bias += lr * err;
                for (w, f) in weights.iter_mut().zip(features.iter()) {
                    *w += lr * err * f;
                }
            }
        }

        // Evaluate precision on validation set
        let mut tp = 0;
        let mut fp = 0;
        for (features, label) in val_samples {
            let mut z = bias;
            for (w, f) in weights.iter().zip(features.iter()) {
                z += w * f;
            }
            let pred = sigmoid(z) >= 0.5;
            let actual = *label >= 0.5;
            if pred && actual {
                tp += 1;
            } else if pred && !actual {
                fp += 1;
            }
        }

        let new_precision = if tp + fp > 0 {
            tp as f64 / (tp + fp) as f64
        } else {
            0.0
        };

        // Drift guard: reject if precision drops by > 2 points (0.02)
        if (old_head.val_precision - new_precision) > 0.02 {
            return Err(SlangError::DriftGuardViolation {
                drop: old_head.val_precision - new_precision,
            });
        }

        let new_head = AdapterHead {
            version: old_head.version + 1,
            weights,
            bias,
            val_precision: new_precision,
            created_at: chrono::Utc::now().timestamp_millis(),
        };

        let mut history = self
            .history
            .write()
            .map_err(|_| SlangError::LockError("history lock poisoned".into()))?;
        history.push(old_head);

        let mut current = self
            .current
            .write()
            .map_err(|_| SlangError::LockError("adapter lock poisoned".into()))?;
        *current = new_head.clone();

        Ok(new_head)
    }

    /// Rolls back to the previous version in the history stack.
    pub fn rollback(&self) -> Result<AdapterHead, SlangError> {
        let mut history = self
            .history
            .write()
            .map_err(|_| SlangError::LockError("history lock poisoned".into()))?;

        let prev = history.pop().ok_or(SlangError::NoPreviousVersion)?;

        let mut current = self
            .current
            .write()
            .map_err(|_| SlangError::LockError("adapter lock poisoned".into()))?;
        *current = prev.clone();

        Ok(prev)
    }
}

fn sigmoid(z: f64) -> f64 {
    1.0 / (1.0 + (-z).exp())
}
