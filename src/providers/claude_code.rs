//! Claude Code Provider (CLI + IDE Extension)
//! Reads usage data from ~/.claude/projects/ directory
//! Note: Both Claude Code CLI and VS Code/Cursor extensions share this data store

use super::Provider;
use crate::types::{ProviderResult, TimeRange, UsageData, UsageStats};
use crate::utils::paths::claude_code;
use crate::utils::tokenizer::calculate_cost;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct ClaudeMessage {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    msg_type: Option<String>,
    message: Option<MessageContent>,
    #[serde(rename = "costUSD")]
    cost_usd: Option<f64>,
    timestamp: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MessageContent {
    usage: Option<UsageInfo>,
    #[allow(dead_code)]
    model: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UsageInfo {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cache_read_input_tokens: Option<u64>,
    cache_creation_input_tokens: Option<u64>,
}

pub struct ClaudeCodeProvider;

impl ClaudeCodeProvider {
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

    fn parse_jsonl_file(path: &Path) -> Vec<ClaudeMessage> {
        let mut messages = Vec::new();

        if let Ok(content) = fs::read_to_string(path) {
            for line in content.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                if let Ok(msg) = serde_json::from_str::<ClaudeMessage>(line) {
                    messages.push(msg);
                }
            }
        }

        messages
    }

    fn extract_usage(msg: &ClaudeMessage) -> UsageData {
        let mut usage = UsageData::new();

        if let Some(ref content) = msg.message {
            if let Some(ref u) = content.usage {
                usage.input_tokens = u.input_tokens.unwrap_or(0);
                usage.output_tokens = u.output_tokens.unwrap_or(0);
                usage.cache_read_tokens = u.cache_read_input_tokens.unwrap_or(0);
                usage.cache_write_tokens = u.cache_creation_input_tokens.unwrap_or(0);
            }
        }

        if let Some(cost) = msg.cost_usd {
            usage.estimated_cost = cost;
        }

        if usage.input_tokens > 0 || usage.output_tokens > 0 {
            usage.request_count = 1;
        }

        usage
    }

    fn parse_timestamp(ts: Option<&String>) -> Option<DateTime<Utc>> {
        ts.and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
    }

    fn process_directory(dir: &Path, stats: &mut UsageStats, ranges: &(TimeRange, TimeRange, TimeRange)) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();

                if path.is_dir() {
                    // Recursively process subdirectories (including subagents/)
                    Self::process_directory(&path, stats, ranges);
                } else if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                    // Process JSONL file
                    let messages = Self::parse_jsonl_file(&path);
                    let file_mtime = fs::metadata(&path)
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|t| DateTime::<Utc>::from(t).into());

                    for msg in messages {
                        let usage = Self::extract_usage(&msg);
                        if usage.input_tokens > 0 || usage.output_tokens > 0 {
                            stats.total.add(&usage);

                            let msg_time = Self::parse_timestamp(msg.timestamp.as_ref())
                                .or(file_mtime)
                                .unwrap_or_else(Utc::now);

                            if ranges.0.contains(msg_time) {
                                stats.today.add(&usage);
                            }
                            if ranges.1.contains(msg_time) {
                                stats.this_week.add(&usage);
                            }
                            if ranges.2.contains(msg_time) {
                                stats.this_month.add(&usage);
                            }
                        }
                    }
                }
            }
        }
    }
}

use chrono::Datelike;

#[async_trait]
impl Provider for ClaudeCodeProvider {
    fn name(&self) -> &'static str {
        "claude-code"
    }

    fn display_name(&self) -> &'static str {
        "Claude Code"  // Includes both CLI and IDE extension (shared data store)
    }

    async fn is_available(&self) -> bool {
        claude_code::projects_dir()
            .map(|p| p.exists())
            .unwrap_or(false)
    }

    fn get_paths_to_check(&self) -> Vec<String> {
        vec![
            claude_code::projects_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
            claude_code::config_file().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        ]
    }

    async fn get_usage(&self, _time_range: Option<&TimeRange>) -> Result<ProviderResult> {
        let projects_dir = match claude_code::projects_dir() {
            Some(p) if p.exists() => p,
            _ => return Ok(ProviderResult::not_found(self.name(), self.display_name())),
        };

        let (today_range, week_range, month_range) = Self::get_time_ranges();
        let mut stats = UsageStats::default();

        // Recursively find and process all JSONL files
        Self::process_directory(&projects_dir, &mut stats, &(today_range, week_range, month_range));

        // Calculate costs if not already set
        if stats.total.estimated_cost == 0.0 && stats.total.total_tokens() > 0 {
            stats.today.estimated_cost = calculate_cost(stats.today.input_tokens, stats.today.output_tokens, Some("claude-sonnet-4"));
            stats.this_week.estimated_cost = calculate_cost(stats.this_week.input_tokens, stats.this_week.output_tokens, Some("claude-sonnet-4"));
            stats.this_month.estimated_cost = calculate_cost(stats.this_month.input_tokens, stats.this_month.output_tokens, Some("claude-sonnet-4"));
            stats.total.estimated_cost = calculate_cost(stats.total.input_tokens, stats.total.output_tokens, Some("claude-sonnet-4"));
        }

        Ok(ProviderResult::active(
            self.name(),
            self.display_name(),
            stats,
            &projects_dir.to_string_lossy(),
        ))
    }
}
