//! Amazon Q Developer Provider
//! Reads local logs from ~/.aws/q/

use super::Provider;
use crate::types::{ProviderResult, TimeRange, UsageData, UsageStats};
use crate::utils::paths::amazon_q;
use crate::utils::time::get_local_time_ranges;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct AmazonQLogEntry {
    timestamp: Option<serde_json::Value>,
    tokens: Option<u64>,
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
}

pub struct AmazonQProvider;

impl AmazonQProvider {
    pub fn new() -> Self {
        Self
    }

    fn get_time_ranges() -> (TimeRange, TimeRange, TimeRange) {
        get_local_time_ranges()
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

                // Try to parse as JSON
                if let Ok(entry) = serde_json::from_str::<AmazonQLogEntry>(line) {
                    Self::process_json_entry(&entry, stats, ranges);
                } else {
                    // Try to extract token information from text
                    Self::process_text_line(line, stats, ranges, file_mtime);
                }
            }
        }
    }

    fn process_json_entry(entry: &AmazonQLogEntry, stats: &mut UsageStats, ranges: &(TimeRange, TimeRange, TimeRange)) {
        let mut usage = UsageData::new();
        usage.input_tokens = entry.input_tokens.unwrap_or(0);
        usage.output_tokens = entry.output_tokens.unwrap_or(0);

        // Handle combined tokens field
        if usage.input_tokens == 0 && usage.output_tokens == 0 {
            if let Some(tokens) = entry.tokens {
                usage.input_tokens = (tokens as f64 * 0.6) as u64;
                usage.output_tokens = (tokens as f64 * 0.4) as u64;
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

    fn process_text_line(
        line: &str,
        stats: &mut UsageStats,
        ranges: &(TimeRange, TimeRange, TimeRange),
        file_mtime: Option<DateTime<Utc>>,
    ) {
        // Look for patterns like "tokens: 1234" or "input_tokens=567"
        let patterns = [
            r"tokens?[:\s=]+(\d+)",
            r"input_tokens?[:\s=]+(\d+)",
            r"output_tokens?[:\s=]+(\d+)",
        ];

        let mut found_tokens = 0u64;
        for pattern in &patterns {
            if let Ok(re) = regex_lite::Regex::new(pattern) {
                if let Some(caps) = re.captures(line) {
                    if let Some(m) = caps.get(1) {
                        if let Ok(n) = m.as_str().parse::<u64>() {
                            found_tokens += n;
                        }
                    }
                }
            }
        }

        if found_tokens > 0 {
            let mut usage = UsageData::new();
            usage.input_tokens = (found_tokens as f64 * 0.6) as u64;
            usage.output_tokens = (found_tokens as f64 * 0.4) as u64;
            usage.request_count = 1;

            stats.total.add(&usage);

            // Try to extract timestamp from line or use file mtime
            let timestamp = Self::extract_timestamp_from_line(line).or(file_mtime);

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

    fn extract_timestamp_from_line(line: &str) -> Option<DateTime<Utc>> {
        // Try to find ISO timestamp pattern
        if let Ok(re) = regex_lite::Regex::new(r"\d{4}-\d{2}-\d{2}[T\s]\d{2}:\d{2}:\d{2}") {
            if let Some(m) = re.find(line) {
                if let Ok(dt) = DateTime::parse_from_rfc3339(&format!("{}Z", m.as_str().replace(' ', "T"))) {
                    return Some(dt.with_timezone(&Utc));
                }
            }
        }
        None
    }
}

#[async_trait]
impl Provider for AmazonQProvider {
    fn name(&self) -> &'static str {
        "amazon-q"
    }

    fn display_name(&self) -> &'static str {
        "Amazon Q"
    }

    async fn is_available(&self) -> bool {
        amazon_q::logs_file().map(|p| p.exists()).unwrap_or(false)
            || amazon_q::config_file().map(|p| p.exists()).unwrap_or(false)
    }

    fn get_paths_to_check(&self) -> Vec<String> {
        vec![
            amazon_q::logs_file().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
            amazon_q::config_file().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        ]
    }

    async fn get_usage(&self, _time_range: Option<&TimeRange>) -> Result<ProviderResult> {
        let logs_path = amazon_q::logs_file();
        let config_path = amazon_q::config_file();

        let has_logs = logs_path.as_ref().map(|p| p.exists()).unwrap_or(false);
        let has_config = config_path.as_ref().map(|p| p.exists()).unwrap_or(false);

        if !has_logs && !has_config {
            return Ok(ProviderResult::not_found(self.name(), self.display_name()));
        }

        let ranges = Self::get_time_ranges();
        let mut stats = UsageStats::default();

        // Process local log file
        if let Some(ref path) = logs_path {
            if path.exists() {
                Self::process_log_file(path, &mut stats, &ranges);
            }
        }

        // If we found no data but AWS config exists, mark as active but no data
        if stats.total.input_tokens == 0 && stats.total.output_tokens == 0 && has_config {
            return Ok(ProviderResult::active(
                self.name(),
                self.display_name(),
                stats,
                "Configured (no local usage data)",
            ));
        }

        let data_source = logs_path
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "Amazon Q".to_string());

        Ok(ProviderResult::active(
            self.name(),
            self.display_name(),
            stats,
            &data_source,
        ))
    }
}
