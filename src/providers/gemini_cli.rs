//! Gemini CLI Provider
//! Reads from ~/.gemini/ and ~/.gemini/antigravity/

use super::Provider;
use crate::types::{ProviderResult, TimeRange, UsageData, UsageStats};
use crate::utils::paths::gemini_cli;
use crate::utils::time::get_local_time_ranges;
use crate::utils::tokenizer::calculate_cost;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct GeminiLogEntry {
    timestamp: Option<serde_json::Value>,
    input_token_count: Option<u64>,
    output_token_count: Option<u64>,
    total_token_count: Option<u64>,
}

pub struct GeminiCLIProvider;

impl GeminiCLIProvider {
    pub fn new() -> Self {
        Self
    }

    fn get_time_ranges() -> (TimeRange, TimeRange, TimeRange) {
        get_local_time_ranges()
    }

    fn process_telemetry_log(path: &Path, stats: &mut UsageStats, ranges: &(TimeRange, TimeRange, TimeRange)) {
        if let Ok(content) = fs::read_to_string(path) {
            for line in content.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                if let Ok(entry) = serde_json::from_str::<GeminiLogEntry>(line) {
                    Self::process_log_entry(&entry, stats, ranges);
                }
            }
        }
    }

    fn process_config_dir(config_dir: &Path, stats: &mut UsageStats, ranges: &(TimeRange, TimeRange, TimeRange)) {
        if let Ok(entries) = fs::read_dir(config_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                // Skip the main telemetry log (already processed)
                if file_name == "telemetry.log" {
                    continue;
                }

                if file_name.ends_with(".log") || file_name.ends_with(".jsonl") {
                    Self::process_telemetry_log(&path, stats, ranges);
                } else if file_name.ends_with(".json") {
                    Self::process_json_file(&path, stats, ranges);
                }
            }
        }
    }

    fn conversations_has_pb_logs(dir: &Path) -> bool {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "pb").unwrap_or(false) {
                    return true;
                }
            }
        }
        false
    }

    fn process_json_file(path: &Path, stats: &mut UsageStats, ranges: &(TimeRange, TimeRange, TimeRange)) {
        if let Ok(content) = fs::read_to_string(path) {
            // Try as array
            if let Ok(entries) = serde_json::from_str::<Vec<GeminiLogEntry>>(&content) {
                for entry in entries {
                    Self::process_log_entry(&entry, stats, ranges);
                }
            }
            // Try as single object
            else if let Ok(entry) = serde_json::from_str::<GeminiLogEntry>(&content) {
                Self::process_log_entry(&entry, stats, ranges);
            }
        }
    }

    fn process_log_entry(entry: &GeminiLogEntry, stats: &mut UsageStats, ranges: &(TimeRange, TimeRange, TimeRange)) {
        let mut usage = UsageData::new();
        usage.input_tokens = entry.input_token_count.unwrap_or(0);
        usage.output_tokens = entry.output_token_count.unwrap_or(0);

        // Handle total_token_count if individual counts not available
        if usage.input_tokens == 0 && usage.output_tokens == 0 {
            if let Some(total) = entry.total_token_count {
                // Total is provided, but input/output split is unknown.
                // Store total without inventing a split.
                usage.input_tokens = total;
                usage.output_tokens = 0;
            }
        }

        if usage.input_tokens > 0 || usage.output_tokens > 0 {
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
}

#[async_trait]
impl Provider for GeminiCLIProvider {
    fn name(&self) -> &'static str {
        "gemini-cli"
    }

    fn display_name(&self) -> &'static str {
        "Gemini CLI"
    }

    async fn is_available(&self) -> bool {
        gemini_cli::conversations_dir().map(|p| p.exists()).unwrap_or(false)
            || gemini_cli::telemetry_file().map(|p| p.exists()).unwrap_or(false)
            || gemini_cli::config_dir().map(|p| p.exists()).unwrap_or(false)
    }

    fn get_paths_to_check(&self) -> Vec<String> {
        vec![
            gemini_cli::conversations_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
            gemini_cli::telemetry_file().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
            gemini_cli::config_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        ]
    }

    async fn get_usage(&self, _time_range: Option<&TimeRange>) -> Result<ProviderResult> {
        let config_dir = gemini_cli::config_dir();
        let telemetry_path = gemini_cli::telemetry_file();
        let conversations_dir = gemini_cli::conversations_dir();

        let has_config = config_dir.as_ref().map(|p| p.exists()).unwrap_or(false);
        let has_telemetry = telemetry_path.as_ref().map(|p| p.exists()).unwrap_or(false);
        let has_conversations = conversations_dir.as_ref().map(|p| p.exists()).unwrap_or(false);

        if !has_config && !has_telemetry && !has_conversations {
            return Ok(ProviderResult::not_found(self.name(), self.display_name()));
        }

        let ranges = Self::get_time_ranges();
        let mut stats = UsageStats::default();

        let mut has_pb_logs = false;
        if let Some(ref dir) = conversations_dir {
            if dir.exists() {
                has_pb_logs = Self::conversations_has_pb_logs(dir);
            }
        }

        // Process telemetry log (legacy)
        if let Some(ref path) = telemetry_path {
            if path.exists() {
                Self::process_telemetry_log(path, &mut stats, &ranges);
            }
        }

        // Process config directory (legacy)
        if let Some(ref dir) = config_dir {
            if dir.exists() {
                Self::process_config_dir(dir, &mut stats, &ranges);
            }
        }

        // Calculate costs using Gemini pricing only when we have a meaningful input+output split.
        if stats.total.input_tokens > 0 && stats.total.output_tokens > 0 {
            stats.today.estimated_cost = calculate_cost(stats.today.input_tokens, stats.today.output_tokens, Some("gemini-2.0-flash"));
            stats.this_week.estimated_cost = calculate_cost(stats.this_week.input_tokens, stats.this_week.output_tokens, Some("gemini-2.0-flash"));
            stats.this_month.estimated_cost = calculate_cost(stats.this_month.input_tokens, stats.this_month.output_tokens, Some("gemini-2.0-flash"));
            stats.total.estimated_cost = calculate_cost(stats.total.input_tokens, stats.total.output_tokens, Some("gemini-2.0-flash"));
        }

        let data_source = telemetry_path
            .as_ref()
            .filter(|p| p.exists())
            .map(|p| p.to_string_lossy().to_string())
            .or_else(|| config_dir.as_ref().map(|p| p.to_string_lossy().to_string()))
            .or_else(|| conversations_dir.as_ref().map(|p| p.to_string_lossy().to_string()))
            .unwrap_or_else(|| "Gemini CLI".to_string());

        // If we didn't parse any real token data but we do see protobuf logs, report as unsupported.
        if stats.total.input_tokens == 0 && stats.total.output_tokens == 0 && stats.total.request_count == 0 && has_pb_logs {
            return Ok(ProviderResult::unsupported(
                self.name(),
                self.display_name(),
                "Gemini CLI logs detected, but they are protobuf (.pb) and this tool does not yet extract token counts from them.",
                Some(&data_source),
            ));
        }

        Ok(ProviderResult::active(
            self.name(),
            self.display_name(),
            stats,
            &data_source,
        ))
    }
}
