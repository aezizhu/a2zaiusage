//! OpenCode Provider
//! Reads session data from ~/.local/share/opencode/storage/message/

use super::Provider;
use crate::types::{ProviderResult, TimeRange, UsageData, UsageStats};
use crate::utils::paths::opencode;
use crate::utils::tokenizer::calculate_cost;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Datelike, TimeZone, Utc};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct OpenCodeSession {
    messages: Option<Vec<OpenCodeMessage>>,
    usage: Option<SessionUsage>,
    created_at: Option<String>,
    updated_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenCodeMessage {
    role: Option<String>,
    usage: Option<MessageUsage>,
    timestamp: Option<serde_json::Value>,
    created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SessionUsage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    #[allow(dead_code)]
    total_tokens: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct MessageUsage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    reasoning_tokens: Option<u64>,
}

pub struct OpenCodeProvider;

impl OpenCodeProvider {
    pub fn new() -> Self {
        Self
    }

    fn get_time_ranges() -> (TimeRange, TimeRange, TimeRange) {
        let now = Utc::now();
        let today_start = Utc.with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0).unwrap();
        let week_start = today_start - chrono::Duration::days(now.weekday().num_days_from_sunday() as i64);
        let month_start = Utc.with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0).unwrap();

        (
            TimeRange { start: today_start, end: now },
            TimeRange { start: week_start, end: now },
            TimeRange { start: month_start, end: now },
        )
    }

    fn process_storage_dir(storage_dir: &Path, stats: &mut UsageStats, ranges: &(TimeRange, TimeRange, TimeRange)) {
        if let Ok(entries) = fs::read_dir(storage_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "json").unwrap_or(false) {
                    Self::process_session_file(&path, stats, ranges);
                }
            }
        }
    }

    fn process_session_file(path: &Path, stats: &mut UsageStats, ranges: &(TimeRange, TimeRange, TimeRange)) {
        let file_mtime = fs::metadata(path)
            .and_then(|m| m.modified())
            .ok()
            .map(|t| DateTime::<Utc>::from(t));

        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(session) = serde_json::from_str::<OpenCodeSession>(&content) {
                Self::process_session(&session, stats, ranges, file_mtime);
            }
        }
    }

    fn process_session(
        session: &OpenCodeSession,
        stats: &mut UsageStats,
        ranges: &(TimeRange, TimeRange, TimeRange),
        file_mtime: Option<DateTime<Utc>>,
    ) {
        // Get session timestamp
        let session_time = session.created_at.as_ref()
            .or(session.updated_at.as_ref())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .or(file_mtime);

        // If session has usage summary
        if let Some(ref u) = session.usage {
            let mut usage = UsageData::new();
            usage.input_tokens = u.input_tokens.unwrap_or(0);
            usage.output_tokens = u.output_tokens.unwrap_or(0);
            usage.request_count = 1;

            Self::add_usage_to_stats(&usage, session_time, stats, ranges);
        }

        // Process individual messages
        if let Some(ref messages) = session.messages {
            for message in messages {
                Self::process_message(message, stats, ranges, session_time);
            }
        }
    }

    fn process_message(
        message: &OpenCodeMessage,
        stats: &mut UsageStats,
        ranges: &(TimeRange, TimeRange, TimeRange),
        session_time: Option<DateTime<Utc>>,
    ) {
        if let Some(ref u) = message.usage {
            let mut usage = UsageData::new();
            usage.input_tokens = u.input_tokens.unwrap_or(0);
            usage.output_tokens = u.output_tokens.unwrap_or(0);

            // Handle reasoning tokens (for models like o1)
            if let Some(reasoning) = u.reasoning_tokens {
                usage.output_tokens += reasoning;
            }

            if message.role.as_deref() == Some("assistant") {
                usage.request_count = 1;
            }

            // Get message timestamp
            let msg_time = message.timestamp.as_ref()
                .and_then(|ts| match ts {
                    serde_json::Value::Number(n) => {
                        let ts = n.as_i64()?;
                        if ts > 1_000_000_000_000 {
                            Utc.timestamp_millis_opt(ts).single()
                        } else {
                            Utc.timestamp_opt(ts, 0).single()
                        }
                    }
                    serde_json::Value::String(s) => {
                        DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.with_timezone(&Utc))
                    }
                    _ => None,
                })
                .or_else(|| message.created_at.as_ref()
                    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.with_timezone(&Utc)))
                .or(session_time);

            Self::add_usage_to_stats(&usage, msg_time, stats, ranges);
        }
    }

    fn add_usage_to_stats(
        usage: &UsageData,
        timestamp: Option<DateTime<Utc>>,
        stats: &mut UsageStats,
        ranges: &(TimeRange, TimeRange, TimeRange),
    ) {
        if usage.input_tokens == 0 && usage.output_tokens == 0 {
            return;
        }

        stats.total.add(usage);

        if let Some(ts) = timestamp {
            if ranges.0.contains(ts) {
                stats.today.add(usage);
            }
            if ranges.1.contains(ts) {
                stats.this_week.add(usage);
            }
            if ranges.2.contains(ts) {
                stats.this_month.add(usage);
            }
        }
    }
}

#[async_trait]
impl Provider for OpenCodeProvider {
    fn name(&self) -> &'static str {
        "opencode"
    }

    fn display_name(&self) -> &'static str {
        "OpenCode"
    }

    async fn is_available(&self) -> bool {
        opencode::storage_dir().map(|p| p.exists()).unwrap_or(false)
    }

    fn get_paths_to_check(&self) -> Vec<String> {
        vec![
            opencode::storage_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        ]
    }

    async fn get_usage(&self, _time_range: Option<&TimeRange>) -> Result<ProviderResult> {
        let storage_dir = match opencode::storage_dir() {
            Some(p) if p.exists() => p,
            _ => return Ok(ProviderResult::not_found(self.name(), self.display_name())),
        };

        let ranges = Self::get_time_ranges();
        let mut stats = UsageStats::default();

        Self::process_storage_dir(&storage_dir, &mut stats, &ranges);

        // Calculate costs
        stats.today.estimated_cost = calculate_cost(stats.today.input_tokens, stats.today.output_tokens, None);
        stats.this_week.estimated_cost = calculate_cost(stats.this_week.input_tokens, stats.this_week.output_tokens, None);
        stats.this_month.estimated_cost = calculate_cost(stats.this_month.input_tokens, stats.this_month.output_tokens, None);
        stats.total.estimated_cost = calculate_cost(stats.total.input_tokens, stats.total.output_tokens, None);

        Ok(ProviderResult::active(
            self.name(),
            self.display_name(),
            stats,
            &storage_dir.to_string_lossy(),
        ))
    }
}
