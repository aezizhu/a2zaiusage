//! Cline Provider (including Roo Code fork)
//! Reads usage data from VS Code extension storage

use super::Provider;
use crate::types::{ProviderResult, TimeRange, UsageData, UsageStats};
use crate::utils::paths::cline;
use crate::utils::time::get_local_time_ranges;
use crate::utils::tokenizer::calculate_cost;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct ClineTaskData {
    #[serde(rename = "tokensIn")]
    tokens_in: Option<u64>,
    #[serde(rename = "tokensOut")]
    tokens_out: Option<u64>,
    #[serde(rename = "cacheWrites")]
    cache_writes: Option<u64>,
    #[serde(rename = "cacheReads")]
    cache_reads: Option<u64>,
    #[serde(rename = "totalCost")]
    total_cost: Option<f64>,
    ts: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct RooUsageTracking {
    #[serde(rename = "totalInputTokens")]
    total_input_tokens: Option<u64>,
    #[serde(rename = "totalOutputTokens")]
    total_output_tokens: Option<u64>,
    #[serde(rename = "totalCacheWriteTokens")]
    total_cache_write_tokens: Option<u64>,
    #[serde(rename = "totalCacheReadTokens")]
    total_cache_read_tokens: Option<u64>,
    #[serde(rename = "totalCost")]
    total_cost: Option<f64>,
}

pub struct ClineProvider;

impl ClineProvider {
    pub fn new() -> Self {
        Self
    }

    fn get_time_ranges() -> (TimeRange, TimeRange, TimeRange) {
        get_local_time_ranges()
    }

    fn get_roo_usage_tracking() -> Option<(UsageStats, String)> {
        let tracking_path = cline::roo_usage_tracking()?;

        if !tracking_path.exists() {
            return None;
        }

        let content = fs::read_to_string(&tracking_path).ok()?;
        let data: RooUsageTracking = serde_json::from_str(&content).ok()?;

        let mut usage = UsageData::new();
        usage.input_tokens = data.total_input_tokens.unwrap_or(0);
        usage.output_tokens = data.total_output_tokens.unwrap_or(0);
        usage.cache_read_tokens = data.total_cache_read_tokens.unwrap_or(0);
        usage.cache_write_tokens = data.total_cache_write_tokens.unwrap_or(0);
        usage.estimated_cost = data.total_cost.unwrap_or(0.0);

        // Roo's tracking file only has totals, not time-based
        let mut stats = UsageStats::default();
        stats.total = usage;

        Some((stats, tracking_path.to_string_lossy().to_string()))
    }

    fn process_tasks_dir(tasks_dir: &Path, stats: &mut UsageStats, ranges: &(TimeRange, TimeRange, TimeRange)) {
        if let Ok(entries) = fs::read_dir(tasks_dir) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    let task_path = entry.path();
                    Self::process_task_dir(&task_path, stats, ranges);
                }
            }
        }
    }

    fn process_task_dir(task_path: &Path, stats: &mut UsageStats, ranges: &(TimeRange, TimeRange, TimeRange)) {
        // Look for task.json
        let task_json = task_path.join("task.json");
        if task_json.exists() {
            if let Ok(content) = fs::read_to_string(&task_json) {
                if let Ok(data) = serde_json::from_str::<ClineTaskData>(&content) {
                    let file_mtime = fs::metadata(&task_json)
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|t| DateTime::<Utc>::from(t).into());

                    Self::process_task_data(&data, stats, ranges, file_mtime);
                }
            }
        }
    }

    fn process_task_data(
        data: &ClineTaskData,
        stats: &mut UsageStats,
        ranges: &(TimeRange, TimeRange, TimeRange),
        file_mtime: Option<DateTime<Utc>>,
    ) {
        let mut usage = UsageData::new();
        usage.input_tokens = data.tokens_in.unwrap_or(0);
        usage.output_tokens = data.tokens_out.unwrap_or(0);
        usage.cache_read_tokens = data.cache_reads.unwrap_or(0);
        usage.cache_write_tokens = data.cache_writes.unwrap_or(0);
        usage.estimated_cost = data.total_cost.unwrap_or(0.0);
        usage.request_count = 1;

        if usage.input_tokens > 0 || usage.output_tokens > 0 {
            stats.total.add(&usage);

            // Determine timestamp
            let timestamp = data.ts
                .map(|ts| {
                    if ts > 1_000_000_000_000 {
                        Utc.timestamp_millis_opt(ts).single()
                    } else {
                        Utc.timestamp_opt(ts, 0).single()
                    }
                })
                .flatten()
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
impl Provider for ClineProvider {
    fn name(&self) -> &'static str {
        "cline"
    }

    fn display_name(&self) -> &'static str {
        "Cline"
    }

    async fn is_available(&self) -> bool {
        cline::original_tasks_dir().map(|p| p.exists()).unwrap_or(false)
            || cline::roo_code_tasks_dir().map(|p| p.exists()).unwrap_or(false)
            || cline::roo_usage_tracking().map(|p| p.exists()).unwrap_or(false)
    }

    fn get_paths_to_check(&self) -> Vec<String> {
        vec![
            cline::original_tasks_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
            cline::roo_code_tasks_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
            cline::roo_usage_tracking().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        ]
    }

    async fn get_usage(&self, _time_range: Option<&TimeRange>) -> Result<ProviderResult> {
        // First, check for Roo Code's usage-tracking.json (easiest and most accurate)
        if let Some((stats, data_source)) = Self::get_roo_usage_tracking() {
            return Ok(ProviderResult::active(
                self.name(),
                "Cline (Roo)",
                stats,
                &data_source,
            ));
        }

        // Fall back to scanning task directories
        let tasks_dir = if cline::roo_code_tasks_dir().map(|p| p.exists()).unwrap_or(false) {
            cline::roo_code_tasks_dir()
        } else {
            cline::original_tasks_dir()
        };

        let tasks_dir = match tasks_dir {
            Some(p) if p.exists() => p,
            _ => return Ok(ProviderResult::not_found(self.name(), self.display_name())),
        };

        let ranges = Self::get_time_ranges();
        let mut stats = UsageStats::default();

        Self::process_tasks_dir(&tasks_dir, &mut stats, &ranges);

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
            &tasks_dir.to_string_lossy(),
        ))
    }
}
