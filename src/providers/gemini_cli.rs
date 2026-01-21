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

/// Entry from a2zusage telemetry file (gemini-wrapper.sh output)
#[derive(Debug, Deserialize)]
struct A2zTelemetryEntry {
    timestamp: Option<String>,
    #[allow(dead_code)]
    model: Option<String>,
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    #[allow(dead_code)]
    total_tokens: Option<u64>,
    cached_tokens: Option<u64>,
    #[allow(dead_code)]
    duration_ms: Option<u64>,
    #[allow(dead_code)]
    tool_calls: Option<u64>,
}

/// Token info from native Gemini CLI session files
#[derive(Debug, Deserialize)]
struct GeminiSessionTokens {
    input: Option<u64>,
    output: Option<u64>,
    cached: Option<u64>,
    #[allow(dead_code)]
    thoughts: Option<u64>,
    #[allow(dead_code)]
    tool: Option<u64>,
    #[allow(dead_code)]
    total: Option<u64>,
}

/// Message in native Gemini CLI session file
#[derive(Debug, Deserialize)]
struct GeminiSessionMessage {
    timestamp: Option<String>,
    #[serde(rename = "type")]
    msg_type: Option<String>,
    tokens: Option<GeminiSessionTokens>,
}

/// Native Gemini CLI session file format (~/.gemini/tmp/<hash>/chats/session-*.json)
#[derive(Debug, Deserialize)]
struct GeminiSession {
    #[allow(dead_code)]
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
    messages: Option<Vec<GeminiSessionMessage>>,
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

                // Skip the a2zusage telemetry file (processed separately)
                if file_name == "a2zusage-telemetry.jsonl" {
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

    /// Process a2zusage telemetry file (real token data from wrapper)
    fn process_a2z_telemetry(path: &Path, stats: &mut UsageStats, ranges: &(TimeRange, TimeRange, TimeRange)) {
        if let Ok(content) = fs::read_to_string(path) {
            for line in content.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                if let Ok(entry) = serde_json::from_str::<A2zTelemetryEntry>(line) {
                    let mut usage = UsageData::new();
                    usage.input_tokens = entry.input_tokens.unwrap_or(0);
                    usage.output_tokens = entry.output_tokens.unwrap_or(0);
                    usage.cache_read_tokens = entry.cached_tokens.unwrap_or(0);
                    
                    if usage.input_tokens > 0 || usage.output_tokens > 0 {
                        usage.request_count = 1;
                        stats.total.add(&usage);

                        // Parse timestamp
                        if let Some(ts_str) = &entry.timestamp {
                            if let Ok(ts) = DateTime::parse_from_rfc3339(ts_str) {
                                let ts = ts.with_timezone(&Utc);
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

    #[allow(dead_code)]
    fn count_pb_files(dir: &Path) -> u64 {
        let mut count = 0u64;
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "pb").unwrap_or(false) {
                    count += 1;
                }
            }
        }
        count
    }

    /// Process native Gemini CLI session files from ~/.gemini/tmp/<hash>/chats/
    fn process_native_sessions(tmp_dir: &Path, stats: &mut UsageStats, ranges: &(TimeRange, TimeRange, TimeRange)) {
        // Iterate through all project hash directories
        if let Ok(entries) = fs::read_dir(tmp_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // Check for chats subdirectory
                    let chats_dir = path.join("chats");
                    if chats_dir.exists() {
                        Self::process_chats_dir(&chats_dir, stats, ranges);
                    }
                }
            }
        }
    }

    /// Process all session JSON files in a chats directory
    fn process_chats_dir(chats_dir: &Path, stats: &mut UsageStats, ranges: &(TimeRange, TimeRange, TimeRange)) {
        if let Ok(entries) = fs::read_dir(chats_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "json").unwrap_or(false) {
                    Self::process_session_file(&path, stats, ranges);
                }
            }
        }
    }

    /// Process a single Gemini CLI session JSON file
    fn process_session_file(path: &Path, stats: &mut UsageStats, ranges: &(TimeRange, TimeRange, TimeRange)) {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(session) = serde_json::from_str::<GeminiSession>(&content) {
                if let Some(messages) = session.messages {
                    for msg in messages {
                        // Only process gemini (model) responses which have token counts
                        if msg.msg_type.as_deref() != Some("gemini") {
                            continue;
                        }
                        
                        if let Some(tokens) = msg.tokens {
                            let mut usage = UsageData::new();
                            usage.input_tokens = tokens.input.unwrap_or(0);
                            usage.output_tokens = tokens.output.unwrap_or(0);
                            usage.cache_read_tokens = tokens.cached.unwrap_or(0);
                            
                            if usage.input_tokens > 0 || usage.output_tokens > 0 {
                                usage.request_count = 1;
                                stats.total.add(&usage);
                                
                                // Parse timestamp
                                if let Some(ts_str) = &msg.timestamp {
                                    if let Ok(ts) = DateTime::parse_from_rfc3339(ts_str) {
                                        let ts = ts.with_timezone(&Utc);
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
                    }
                }
            }
        }
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
        let a2z_telemetry = gemini_cli::a2zusage_telemetry_file();

        let has_config = config_dir.as_ref().map(|p| p.exists()).unwrap_or(false);
        let has_telemetry = telemetry_path.as_ref().map(|p| p.exists()).unwrap_or(false);
        let has_conversations = conversations_dir.as_ref().map(|p| p.exists()).unwrap_or(false);
        let has_a2z_telemetry = a2z_telemetry.as_ref().map(|p| p.exists()).unwrap_or(false);

        if !has_config && !has_telemetry && !has_conversations && !has_a2z_telemetry {
            return Ok(ProviderResult::not_found(self.name(), self.display_name()));
        }

        let ranges = Self::get_time_ranges();
        let mut stats = UsageStats::default();

        // PRIORITY: Process a2zusage telemetry file first (real token data from wrapper)
        if let Some(ref path) = a2z_telemetry {
            if path.exists() {
                Self::process_a2z_telemetry(path, &mut stats, &ranges);
            }
        }

        // PRIORITY 2: Process native Gemini CLI session files
        let tmp_dir = gemini_cli::tmp_dir();
        if let Some(ref dir) = tmp_dir {
            if dir.exists() {
                Self::process_native_sessions(dir, &mut stats, &ranges);
            }
        }

        // If we got data from a2zusage telemetry or native sessions, return it
        if stats.total.input_tokens > 0 || stats.total.output_tokens > 0 {
            // Calculate costs
            stats.today.estimated_cost = calculate_cost(stats.today.input_tokens, stats.today.output_tokens, Some("gemini-2.0-flash"));
            stats.this_week.estimated_cost = calculate_cost(stats.this_week.input_tokens, stats.this_week.output_tokens, Some("gemini-2.0-flash"));
            stats.this_month.estimated_cost = calculate_cost(stats.this_month.input_tokens, stats.this_month.output_tokens, Some("gemini-2.0-flash"));
            stats.total.estimated_cost = calculate_cost(stats.total.input_tokens, stats.total.output_tokens, Some("gemini-2.0-flash"));
            
            let source = if has_a2z_telemetry {
                "a2zusage telemetry + native sessions"
            } else {
                "~/.gemini/tmp/*/chats/"
            };
            
            return Ok(ProviderResult::active(
                self.name(),
                self.display_name(),
                stats,
                source,
            ));
        }

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

        // If we didn't parse any real token data but we do see protobuf logs,
        // report as Unsupported - token data is encrypted and cannot be extracted.
        // The /stats command shows usage only during active sessions.
        if stats.total.input_tokens == 0 && stats.total.output_tokens == 0 && stats.total.request_count == 0 && has_pb_logs {
            return Ok(ProviderResult::unsupported(
                self.name(),
                self.display_name(),
                "Token data is encrypted in .pb files. Use /stats command in Gemini CLI for usage.",
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
