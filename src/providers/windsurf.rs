//! Windsurf Provider
//! Reads cascade logs from ~/.codeium/windsurf/

use super::Provider;
use crate::types::{ProviderResult, TimeRange, UsageData, UsageStats};
use crate::utils::paths::windsurf;
use crate::utils::tokenizer::estimate_tokens_from_chars;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Datelike, TimeZone, Utc};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct CascadeLogEntry {
    timestamp: Option<serde_json::Value>,
    usage: Option<CascadeUsage>,
    billable_tokens: Option<u64>,
    generated_tokens: Option<u64>,
    context_length: Option<u64>,
    completion_length: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct CascadeUsage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    context_length: Option<u64>,
    completion_length: Option<u64>,
}

pub struct WindsurfProvider;

impl WindsurfProvider {
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

    fn process_cascade_dir(cascade_dir: &Path, stats: &mut UsageStats, ranges: &(TimeRange, TimeRange, TimeRange)) {
        if let Ok(entries) = fs::read_dir(cascade_dir) {
            for entry in entries.flatten() {
                let path = entry.path();

                // Skip directories
                if path.is_dir() {
                    continue;
                }

                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                if file_name.ends_with(".jsonl") || file_name.ends_with(".log") {
                    Self::process_jsonl_file(&path, stats, ranges);
                } else if file_name.ends_with(".json") {
                    Self::process_json_file(&path, stats, ranges);
                } else if file_name.ends_with(".pb") {
                    // For protobuf files, estimate by file size
                    if let Ok(meta) = fs::metadata(&path) {
                        let size_tokens = (meta.len() / 10) as u64;
                        Self::add_usage_estimate(stats, size_tokens, &path, ranges);
                    }
                }
            }
        }
    }

    fn process_jsonl_file(path: &Path, stats: &mut UsageStats, ranges: &(TimeRange, TimeRange, TimeRange)) {
        if let Ok(content) = fs::read_to_string(path) {
            for line in content.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                if let Ok(entry) = serde_json::from_str::<CascadeLogEntry>(line) {
                    Self::process_log_entry(&entry, stats, ranges);
                }
            }
        }
    }

    fn process_json_file(path: &Path, stats: &mut UsageStats, ranges: &(TimeRange, TimeRange, TimeRange)) {
        if let Ok(content) = fs::read_to_string(path) {
            // Try as array
            if let Ok(entries) = serde_json::from_str::<Vec<CascadeLogEntry>>(&content) {
                for entry in entries {
                    Self::process_log_entry(&entry, stats, ranges);
                }
            }
            // Try as single object
            else if let Ok(entry) = serde_json::from_str::<CascadeLogEntry>(&content) {
                Self::process_log_entry(&entry, stats, ranges);
            }
        }
    }

    fn process_log_entry(entry: &CascadeLogEntry, stats: &mut UsageStats, ranges: &(TimeRange, TimeRange, TimeRange)) {
        let mut usage = UsageData::new();

        // Extract tokens from various possible fields
        if let Some(ref u) = entry.usage {
            usage.input_tokens = u.input_tokens.or(u.context_length).unwrap_or(0);
            usage.output_tokens = u.output_tokens.or(u.completion_length).unwrap_or(0);
        }

        if let Some(billable) = entry.billable_tokens {
            usage.input_tokens = usage.input_tokens.max((billable as f64 * 0.6) as u64);
            usage.output_tokens = usage.output_tokens.max((billable as f64 * 0.4) as u64);
        }

        if let Some(context) = entry.context_length {
            usage.input_tokens = usage.input_tokens.max(context);
        }

        if let Some(completion) = entry.completion_length.or(entry.generated_tokens) {
            usage.output_tokens = usage.output_tokens.max(completion);
        }

        if usage.input_tokens > 0 || usage.output_tokens > 0 {
            usage.request_count = 1;
            stats.total.add(&usage);

            // Get timestamp
            let timestamp = entry.timestamp.as_ref().and_then(|ts| {
                match ts {
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
                }
            });

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

    fn add_usage_estimate(stats: &mut UsageStats, tokens: u64, path: &Path, ranges: &(TimeRange, TimeRange, TimeRange)) {
        let mut usage = UsageData::new();
        usage.input_tokens = (tokens as f64 * 0.6) as u64;
        usage.output_tokens = (tokens as f64 * 0.4) as u64;
        usage.request_count = 1;

        stats.total.add(&usage);

        if let Ok(meta) = fs::metadata(path) {
            if let Ok(mtime) = meta.modified() {
                let file_date: DateTime<Utc> = mtime.into();
                if ranges.0.contains(file_date) {
                    stats.today.add(&usage);
                }
                if ranges.1.contains(file_date) {
                    stats.this_week.add(&usage);
                }
                if ranges.2.contains(file_date) {
                    stats.this_month.add(&usage);
                }
            }
        }
    }

    fn process_memories_dir(memories_dir: &Path, stats: &mut UsageStats, ranges: &(TimeRange, TimeRange, TimeRange)) {
        if let Ok(entries) = fs::read_dir(memories_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "json" || e == "txt").unwrap_or(false) {
                    if let Ok(content) = fs::read_to_string(&path) {
                        let tokens = estimate_tokens_from_chars(content.len());
                        Self::add_usage_estimate(stats, tokens, &path, ranges);
                    }
                }
            }
        }
    }
}

#[async_trait]
impl Provider for WindsurfProvider {
    fn name(&self) -> &'static str {
        "windsurf"
    }

    fn display_name(&self) -> &'static str {
        "Windsurf"
    }

    async fn is_available(&self) -> bool {
        windsurf::cascade_dir().map(|p| p.exists()).unwrap_or(false)
            || windsurf::memories_dir().map(|p| p.exists()).unwrap_or(false)
            || windsurf::config_dir().map(|p| p.exists()).unwrap_or(false)
    }

    fn get_paths_to_check(&self) -> Vec<String> {
        vec![
            windsurf::cascade_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
            windsurf::memories_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
            windsurf::config_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        ]
    }

    async fn get_usage(&self, _time_range: Option<&TimeRange>) -> Result<ProviderResult> {
        let cascade_dir = windsurf::cascade_dir();
        let memories_dir = windsurf::memories_dir();

        let has_cascade = cascade_dir.as_ref().map(|p| p.exists()).unwrap_or(false);
        let has_memories = memories_dir.as_ref().map(|p| p.exists()).unwrap_or(false);

        if !has_cascade && !has_memories {
            // Check if at least config exists
            if windsurf::config_dir().map(|p| p.exists()).unwrap_or(false) {
                return Ok(ProviderResult::active(
                    self.name(),
                    self.display_name(),
                    UsageStats::default(),
                    "Installed (no usage data found)",
                ));
            }
            return Ok(ProviderResult::not_found(self.name(), self.display_name()));
        }

        let ranges = Self::get_time_ranges();
        let mut stats = UsageStats::default();

        if let Some(ref dir) = cascade_dir {
            if dir.exists() {
                Self::process_cascade_dir(dir, &mut stats, &ranges);
            }
        }

        if let Some(ref dir) = memories_dir {
            if dir.exists() {
                Self::process_memories_dir(dir, &mut stats, &ranges);
            }
        }

        let data_source = cascade_dir
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "Windsurf".to_string());

        Ok(ProviderResult::active(
            self.name(),
            self.display_name(),
            stats,
            &data_source,
        ))
    }
}
