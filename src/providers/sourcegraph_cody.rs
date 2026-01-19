//! Sourcegraph Cody Provider
//! Reads from VS Code extension storage

use super::Provider;
use crate::types::{ProviderResult, TimeRange, UsageData, UsageStats};
use crate::utils::paths::sourcegraph_cody;
use crate::utils::time::get_local_time_ranges;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct CodyStateData {
    messages: Option<Vec<CodyMessage>>,
    #[serde(rename = "tokenCount")]
    token_count: Option<CodyTokenCount>,
}

#[derive(Debug, Deserialize)]
struct CodyMessage {
    role: Option<String>,
    timestamp: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct CodyTokenCount {
    input: Option<u64>,
    output: Option<u64>,
}

pub struct SourcegraphCodyProvider;

impl SourcegraphCodyProvider {
    pub fn new() -> Self {
        Self
    }

    fn get_time_ranges() -> (TimeRange, TimeRange, TimeRange) {
        get_local_time_ranges()
    }

    fn process_extension_dir(dir: &Path, stats: &mut UsageStats, ranges: &(TimeRange, TimeRange, TimeRange)) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "json").unwrap_or(false) {
                    Self::process_json_file(&path, stats, ranges);
                }
            }
        }
    }

    fn process_json_file(path: &Path, stats: &mut UsageStats, ranges: &(TimeRange, TimeRange, TimeRange)) {
        let file_mtime = fs::metadata(path)
            .and_then(|m| m.modified())
            .ok()
            .map(|t| DateTime::<Utc>::from(t));

        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(data) = serde_json::from_str::<CodyStateData>(&content) {
                Self::process_state_data(&data, stats, ranges, file_mtime);
            }
        }
    }

    fn process_state_data(
        data: &CodyStateData,
        stats: &mut UsageStats,
        ranges: &(TimeRange, TimeRange, TimeRange),
        file_mtime: Option<DateTime<Utc>>,
    ) {
        let mut usage = UsageData::new();

        // Extract token counts if available
        if let Some(ref tc) = data.token_count {
            usage.input_tokens = tc.input.unwrap_or(0);
            usage.output_tokens = tc.output.unwrap_or(0);
        }

        // Count messages as requests
        if let Some(ref messages) = data.messages {
            let assistant_count = messages.iter()
                .filter(|m| m.role.as_deref() == Some("assistant"))
                .count() as u64;
            usage.request_count = assistant_count;

            // If no token count but have messages, estimate
            if usage.input_tokens == 0 && usage.output_tokens == 0 && assistant_count > 0 {
                usage.input_tokens = assistant_count * 300; // rough estimate
                usage.output_tokens = assistant_count * 200;
            }
        }

        if usage.input_tokens > 0 || usage.output_tokens > 0 || usage.request_count > 0 {
            stats.total.add(&usage);

            // Try to get timestamp from messages or file
            let timestamp = data.messages.as_ref()
                .and_then(|msgs| msgs.iter().filter_map(|m| m.timestamp).max())
                .and_then(|ts| {
                    if ts > 1_000_000_000_000 {
                        Utc.timestamp_millis_opt(ts).single()
                    } else {
                        Utc.timestamp_opt(ts, 0).single()
                    }
                })
                .or(file_mtime);

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
impl Provider for SourcegraphCodyProvider {
    fn name(&self) -> &'static str {
        "sourcegraph-cody"
    }

    fn display_name(&self) -> &'static str {
        "Cody"
    }

    async fn is_available(&self) -> bool {
        sourcegraph_cody::vscode_extension().map(|p| p.exists()).unwrap_or(false)
    }

    fn get_paths_to_check(&self) -> Vec<String> {
        vec![
            sourcegraph_cody::vscode_extension().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        ]
    }

    async fn get_usage(&self, _time_range: Option<&TimeRange>) -> Result<ProviderResult> {
        let extension_dir = match sourcegraph_cody::vscode_extension() {
            Some(p) if p.exists() => p,
            _ => return Ok(ProviderResult::not_found(self.name(), self.display_name())),
        };

        let ranges = Self::get_time_ranges();
        let mut stats = UsageStats::default();

        Self::process_extension_dir(&extension_dir, &mut stats, &ranges);

        Ok(ProviderResult::active(
            self.name(),
            self.display_name(),
            stats,
            &extension_dir.to_string_lossy(),
        ))
    }
}
