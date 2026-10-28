pub use hre_core::{Action, AnalyzeRequest, AnalyzeResponse, SecondaryAction, Tier};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Debug, thiserror::Error)]
pub enum ConnectorError {
    #[error("network connection failed: {0}")]
    Network(String),
    #[error("API returned error status {0}: {1}")]
    Api(u16, String),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("invalid response from server: {0}")]
    InvalidResponse(String),
}

/// Typed client for the Harassment Risk Engine API.
#[derive(Debug, Clone)]
pub struct HreClient {
    pub base_url: String,
    pub api_key: String,
}

impl HreClient {
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key: api_key.into(),
        }
    }

    /// Submits a typed AnalyzeRequest to /v1/analyze via pure async HTTP/1.1 over TcpStream.
    pub async fn analyze_event(
        &self,
        request: &AnalyzeRequest,
    ) -> Result<AnalyzeResponse, ConnectorError> {
        let json_payload = serde_json::to_string(request)
            .map_err(|e| ConnectorError::Serialization(e.to_string()))?;

        let (status, body) = self.post_json("/v1/analyze", &json_payload).await?;
        if status != 200 {
            return Err(ConnectorError::Api(status, body));
        }

        serde_json::from_str(&body).map_err(|e| {
            ConnectorError::InvalidResponse(format!("failed to parse response JSON: {e}"))
        })
    }

    /// Submits an image or voice attachment via multipart/form-data to /v1/analyze/media.
    pub async fn analyze_media(
        &self,
        request: &AnalyzeRequest,
        file_bytes: &[u8],
        field_name: &str,
        filename: &str,
        mime_type: &str,
    ) -> Result<AnalyzeResponse, ConnectorError> {
        let request_json = serde_json::to_string(request)
            .map_err(|e| ConnectorError::Serialization(e.to_string()))?;

        let boundary = "hre-connector-boundary-9876543210";
        let mut multipart_body = Vec::new();

        // Field 1: request JSON
        multipart_body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"request\"\r\nContent-Type: application/json\r\n\r\n{request_json}\r\n"
            )
            .as_bytes(),
        );

        // Field 2: binary media payload
        multipart_body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"{field_name}\"; filename=\"{filename}\"\r\nContent-Type: {mime_type}\r\n\r\n"
            )
            .as_bytes(),
        );
        multipart_body.extend_from_slice(file_bytes);
        multipart_body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

        let (status, body) = self
            .post_raw(
                "/v1/analyze/media",
                &format!("multipart/form-data; boundary={boundary}"),
                &multipart_body,
            )
            .await?;

        if status != 200 {
            return Err(ConnectorError::Api(status, body));
        }

        serde_json::from_str(&body).map_err(|e| {
            ConnectorError::InvalidResponse(format!("failed to parse media response JSON: {e}"))
        })
    }

    /// Checks engine health status.
    pub async fn health_check(&self) -> Result<bool, ConnectorError> {
        let (status, body) = self.get_path("/v1/health").await?;
        if status == 200 {
            let val: serde_json::Value = serde_json::from_str(&body)
                .map_err(|e| ConnectorError::InvalidResponse(e.to_string()))?;
            Ok(val["status"] == "ok")
        } else {
            Ok(false)
        }
    }

    /// Fetches the current list of alerts using a moderator token.
    pub async fn get_alerts(
        &self,
        moderator_token: &str,
    ) -> Result<Vec<AnalyzeResponse>, ConnectorError> {
        let (host, port) = self.parse_host_port()?;
        let addr = format!("{host}:{port}");

        let mut stream = TcpStream::connect(&addr)
            .await
            .map_err(|e| ConnectorError::Network(format!("cannot connect to {addr}: {e}")))?;

        let request_header = format!(
            "GET /v1/alerts HTTP/1.1\r\n\
             Host: {host}:{port}\r\n\
             Authorization: Bearer {moderator_token}\r\n\
             Connection: close\r\n\r\n"
        );

        stream
            .write_all(request_header.as_bytes())
            .await
            .map_err(|e| ConnectorError::Network(format!("write request failed: {e}")))?;

        let mut response_bytes = Vec::new();
        stream
            .read_to_end(&mut response_bytes)
            .await
            .map_err(|e| ConnectorError::Network(format!("read response failed: {e}")))?;

        let (status, body) = Self::parse_http_response(&response_bytes)?;
        if status != 200 {
            return Err(ConnectorError::Api(status, body));
        }

        serde_json::from_str(&body).map_err(|e| {
            ConnectorError::InvalidResponse(format!("failed to parse alerts JSON: {e}"))
        })
    }

    /// Requests an evidence timeline export for a subject.
    pub async fn export_evidence(
        &self,
        subject_id: &str,
    ) -> Result<serde_json::Value, ConnectorError> {
        let payload = serde_json::json!({ "subject_id": subject_id });
        let payload_str = payload.to_string();
        let (status, body) = self.post_json("/v1/evidence/export", &payload_str).await?;
        if status != 200 {
            return Err(ConnectorError::Api(status, body));
        }
        serde_json::from_str(&body).map_err(|e| {
            ConnectorError::InvalidResponse(format!("failed to parse evidence export JSON: {e}"))
        })
    }

    /// Imports call logs from CSV.
    pub async fn import_calls_csv(&self, csv_data: &str) -> Result<usize, ConnectorError> {
        let payload = serde_json::json!({ "csv_data": csv_data });
        let payload_str = payload.to_string();
        let (status, body) = self.post_json("/v1/calllogs/import", &payload_str).await?;
        if status != 200 {
            return Err(ConnectorError::Api(status, body));
        }
        let val: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
            ConnectorError::InvalidResponse(format!(
                "failed to parse call import response JSON: {e}"
            ))
        })?;
        Ok(val.get("imported").and_then(|v| v.as_u64()).unwrap_or(0) as usize)
    }

    async fn post_json(&self, path: &str, json: &str) -> Result<(u16, String), ConnectorError> {
        self.post_raw(path, "application/json", json.as_bytes())
            .await
    }

    async fn post_raw(
        &self,
        path: &str,
        content_type: &str,
        body: &[u8],
    ) -> Result<(u16, String), ConnectorError> {
        let (host, port) = self.parse_host_port()?;
        let addr = format!("{host}:{port}");

        let mut stream = TcpStream::connect(&addr)
            .await
            .map_err(|e| ConnectorError::Network(format!("cannot connect to {addr}: {e}")))?;

        let request_header = format!(
            "POST {path} HTTP/1.1\r\n\
             Host: {host}:{port}\r\n\
             Content-Type: {content_type}\r\n\
             Content-Length: {}\r\n\
             X-API-Key: {}\r\n\
             Connection: close\r\n\r\n",
            body.len(),
            self.api_key
        );

        stream
            .write_all(request_header.as_bytes())
            .await
            .map_err(|e| ConnectorError::Network(format!("write header failed: {e}")))?;
        stream
            .write_all(body)
            .await
            .map_err(|e| ConnectorError::Network(format!("write body failed: {e}")))?;

        let mut response_bytes = Vec::new();
        stream
            .read_to_end(&mut response_bytes)
            .await
            .map_err(|e| ConnectorError::Network(format!("read response failed: {e}")))?;

        Self::parse_http_response(&response_bytes)
    }

    async fn get_path(&self, path: &str) -> Result<(u16, String), ConnectorError> {
        let (host, port) = self.parse_host_port()?;
        let addr = format!("{host}:{port}");

        let mut stream = TcpStream::connect(&addr)
            .await
            .map_err(|e| ConnectorError::Network(format!("cannot connect to {addr}: {e}")))?;

        let request_header = format!(
            "GET {path} HTTP/1.1\r\n\
             Host: {host}:{port}\r\n\
             X-API-Key: {}\r\n\
             Connection: close\r\n\r\n",
            self.api_key
        );

        stream
            .write_all(request_header.as_bytes())
            .await
            .map_err(|e| ConnectorError::Network(format!("write request failed: {e}")))?;

        let mut response_bytes = Vec::new();
        stream
            .read_to_end(&mut response_bytes)
            .await
            .map_err(|e| ConnectorError::Network(format!("read response failed: {e}")))?;

        Self::parse_http_response(&response_bytes)
    }

    fn parse_host_port(&self) -> Result<(String, u16), ConnectorError> {
        let url = &self.base_url;
        let url_no_proto = url
            .strip_prefix("http://")
            .or_else(|| url.strip_prefix("https://"))
            .unwrap_or(url);

        let parts: Vec<&str> = url_no_proto.split(':').collect();
        let host = parts[0].to_string();
        let port: u16 = if parts.len() > 1 {
            parts[1]
                .split('/')
                .next()
                .unwrap_or("8080")
                .parse()
                .map_err(|_| ConnectorError::Network("invalid port in base_url".into()))?
        } else {
            8080
        };

        Ok((host, port))
    }

    fn parse_http_response(raw: &[u8]) -> Result<(u16, String), ConnectorError> {
        let raw_str = String::from_utf8_lossy(raw);
        let header_end = raw_str
            .find("\r\n\r\n")
            .ok_or_else(|| ConnectorError::InvalidResponse("missing header delimiter".into()))?;

        let header_part = &raw_str[..header_end];
        let body_part = &raw_str[header_end + 4..];

        let mut status_code = 200u16;
        if let Some(first_line) = header_part.lines().next() {
            let chunks: Vec<&str> = first_line.split_whitespace().collect();
            if chunks.len() >= 2 {
                if let Ok(code) = chunks[1].parse::<u16>() {
                    status_code = code;
                }
            }
        }

        Ok((status_code, body_part.to_string()))
    }
}

/// Abstract moderation actions emitted by connectors upon evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlatformModerationAction {
    Allow,
    WarnAuthor {
        warning: String,
    },
    DeleteMessage {
        reason: String,
    },
    TimeoutUser {
        duration_seconds: u32,
        reason: String,
    },
    BlurMediaNotice {
        caption: String,
    },
    AlertStaffChannel {
        summary: String,
        severity: u8,
    },
}

/// Translates engine response into platform moderation directives.
pub fn map_engine_response(response: &AnalyzeResponse) -> Vec<PlatformModerationAction> {
    let mut actions = Vec::new();

    if response.tier == Tier::Low {
        actions.push(PlatformModerationAction::Allow);
        return actions;
    }

    let summary_reason = response
        .reasons
        .first()
        .map(|r| r.text.clone())
        .unwrap_or_else(|| "Violates harassment and community safety policy".into());

    match response.recommended_action {
        Action::WarnSender => {
            actions.push(PlatformModerationAction::WarnAuthor {
                warning: format!("Warning: Your message was flagged for potential harassment ({summary_reason}). Please keep conversations respectful."),
            });
        }
        Action::Throttle => {
            actions.push(PlatformModerationAction::DeleteMessage {
                reason: summary_reason.clone(),
            });
            actions.push(PlatformModerationAction::TimeoutUser {
                duration_seconds: 3600, // 1 hour timeout
                reason: format!("Repeated harassment boundary violation: {summary_reason}"),
            });
        }
        Action::BlurMedia => {
            actions.push(PlatformModerationAction::DeleteMessage {
                reason: format!("Unsolicited explicit or abusive media: {summary_reason}"),
            });
            actions.push(PlatformModerationAction::BlurMediaNotice {
                caption: "Media hidden due to explicit or abusive content.".into(),
            });
        }
        Action::Escalate => {
            actions.push(PlatformModerationAction::DeleteMessage {
                reason: format!("Severe violation: {summary_reason}"),
            });
            actions.push(PlatformModerationAction::TimeoutUser {
                duration_seconds: 86400, // 24 hour restriction
                reason: format!("High-severity harassment escalation: {summary_reason}"),
            });
            actions.push(PlatformModerationAction::AlertStaffChannel {
                summary: format!(
                    "URGENT: High-severity event detected (severity {}): {}",
                    response.severity, summary_reason
                ),
                severity: response.severity,
            });
        }
        Action::Log => {
            actions.push(PlatformModerationAction::Allow);
        }
    }

    for secondary in &response.secondary_actions {
        if *secondary == SecondaryAction::AlertModerator
            && !actions
                .iter()
                .any(|a| matches!(a, PlatformModerationAction::AlertStaffChannel { .. }))
        {
            actions.push(PlatformModerationAction::AlertStaffChannel {
                summary: format!(
                    "Moderator Review Required: Event flagged at tier {:?} with severity {}: {}",
                    response.tier, response.severity, summary_reason
                ),
                severity: response.severity,
            });
        }
    }

    actions
}

pub mod discord;
pub mod telegram;

pub use discord::DiscordConnector;
pub use telegram::TelegramConnector;
