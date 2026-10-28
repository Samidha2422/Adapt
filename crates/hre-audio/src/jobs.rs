use crate::transcribe::AudioReport;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRecord {
    pub job_id: Uuid,
    pub status: JobStatus,
    pub created_at: i64,
    pub completed_at: Option<i64>,
    pub report: Option<AudioReport>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct JobManager {
    jobs: Arc<Mutex<BTreeMap<Uuid, JobRecord>>>,
}

impl JobManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_job(&self) -> Uuid {
        let job_id = Uuid::new_v4();
        let record = JobRecord {
            job_id,
            status: JobStatus::Pending,
            created_at: chrono::Utc::now().timestamp_millis(),
            completed_at: None,
            report: None,
            error: None,
        };
        if let Ok(mut jobs) = self.jobs.lock() {
            jobs.insert(job_id, record);
        }
        job_id
    }

    pub fn complete_job(&self, job_id: Uuid, report: AudioReport) {
        if let Ok(mut jobs) = self.jobs.lock() {
            if let Some(record) = jobs.get_mut(&job_id) {
                record.status = JobStatus::Completed;
                record.completed_at = Some(chrono::Utc::now().timestamp_millis());
                record.report = Some(report);
            }
        }
    }

    pub fn fail_job(&self, job_id: Uuid, error: String) {
        if let Ok(mut jobs) = self.jobs.lock() {
            if let Some(record) = jobs.get_mut(&job_id) {
                record.status = JobStatus::Failed;
                record.completed_at = Some(chrono::Utc::now().timestamp_millis());
                record.error = Some(error);
            }
        }
    }

    pub fn get_job(&self, job_id: Uuid) -> Option<JobRecord> {
        self.jobs.lock().ok()?.get(&job_id).cloned()
    }
}
