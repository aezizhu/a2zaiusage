//! Tabnine Provider
//! Reads local logs from TabNine log directory

use super::Provider;
use crate::types::{ProviderResult, TimeRange, UsageData, UsageStats};
use crate::utils::paths::tabnine;
use crate::utils::time::get_local_time_ranges;
use crate::utils::tokenizer::estimate_tokens_from_chars;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct TabnineLogEntry {
    #[serde(rename = "type")]
    entry_type: Option<String>,
    event: Option<String>,
    timestamp: Option<serde_json::Value>,
    meta: Option<TabnineMeta>,
    usage: Option<TabnineUsage>,
}

#[derive(Debug, Deserialize)]
struct TabnineMeta {
    net_length: Option<u64>,
    tokens_used: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct TabnineUsage {
    tokens: Option<u64>,
    chars: Option<u64>,
}

pub struct TabnineProvider;

impl TabnineProvider {
    pub fn new() -> Self {
        Self
    }

    fn get_time_ranges() -> (TimeRange, TimeRange, TimeRange) {
        get_local_time_ranges()
    }

    fn process_logs_dir(logs_dir: &Path, stats: &mut UsageStats, ranges: &(TimeRange, TimeRange, TimeRange)) {
        if let Ok(entries) = fs::read_dir(logs_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "log" || e == "json" || e == "jsonl").unwrap_or(false) {
                    Self::process_log_file(&path, stats, ranges);
                }
            }
        }
    }

    fn process_log_file(path: &Path, stats: &mut UsageStats, ranges: &(TimeRange, TimeRange, TimeRange)) {
        let file_mtime = fs::metadata(path)
            .and_then(|m| m.modified())
            .ok()
            .map(|t| DateTime::<Utc>::from(t));

        if let Ok(content) = fs::read_to_string(path) {
            for line in content.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                if let Ok(entry) = serde_json::from_str::<TabnineLogEntry>(line) {
                    Self::process_log_entry(&entry, stats, ranges, file_mtime);
                }
            }
        }
    }

    fn process_log_entry(
        entry: &TabnineLogEntry,
        stats: &mut UsageStats,
        ranges: &(TimeRange, TimeRange, TimeRange),
        file_mtime: Option<DateTime<Utc>>,
    ) {
        // Only process completion events
        let is_completion = entry.entry_type.as_deref() == Some("completion")
            || entry.event.as_deref() == Some("usage")
            || entry.event.as_deref() == Some("completion");

        if !is_completion && entry.meta.is_none() && entry.usage.is_none() {
            return;
        }

        let mut usage = UsageData::new();

        // Extract from meta
        if let Some(ref meta) = entry.meta {
            if let Some(tokens) = meta.tokens_used {
                usage.output_tokens = tokens;
            } else if let Some(net_length) = meta.net_length {
                // Convert character length to tokens
                usage.output_tokens = estimate_tokens_from_chars(net_length as usize);
            }
        }

        // Extract from usage
        if let Some(ref u) = entry.usage {
            if let Some(tokens) = u.tokens {
                usage.output_tokens = usage.output_tokens.max(tokens);
            } else if let Some(chars) = u.chars {
                usage.output_tokens = usage.output_tokens.max(estimate_tokens_from_chars(chars as usize));
            }
        }

        if usage.output_tokens > 0 {
            // Tabnine is mostly completion, so estimate input as context
            usage.input_tokens = usage.output_tokens * 2; // rough estimate
            usage.request_count = 1;
            stats.total.add(&usage);

            // Get timestamp
            let timestamp = entry.timestamp.as_ref().and_then(|ts| match ts {
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
            }).or(file_mtime);

            if let Some(ts) = timestamp {
                if ranges.0.contains(ts) {
                    stats.today.add(&usage);
                }
                if ranges.1.contains(ts) {
                    stats.this_week.add(&usage);
                }
                if ranges.2.contains(ts) {
                    stats.this_month.add(&usage);
                }
            }
        }
    }
}

#[async_trait]
impl Provider for TabnineProvider {
    fn name(&self) -> &'static str {
        "tabnine"
    }

    fn display_name(&self) -> &'static str {
        "Tabnine"
    }

    async fn is_available(&self) -> bool {
        tabnine::logs_dir().map(|p| p.exists()).unwrap_or(false)
    }

    fn get_paths_to_check(&self) -> Vec<String> {
        vec![
            tabnine::logs_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        ]
    }

    async fn get_usage(&self, _time_range: Option<&TimeRange>) -> Result<ProviderResult> {
        let logs_dir = match tabnine::logs_dir() {
            Some(p) if p.exists() => p,
            _ => return Ok(ProviderResult::not_found(self.name(), self.display_name())),
        };

        let ranges = Self::get_time_ranges();
        let mut stats = UsageStats::default();

        Self::process_logs_dir(&logs_dir, &mut stats, &ranges);

        Ok(ProviderResult::active(
            self.name(),
            self.display_name(),
            stats,
            &logs_dir.to_string_lossy(),
        ))
    }
}
