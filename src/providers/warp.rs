//! Warp Terminal Provider
//! Reads AI usage data from Warp's SQLite database

use super::Provider;
use crate::types::{ProviderResult, TimeRange, UsageData, UsageStats};
use crate::utils::db::with_db_snapshot;
use crate::utils::paths::warp;
use crate::utils::time::get_local_time_ranges;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use rusqlite::Connection;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct ConversationData {
    conversation_usage_metadata: Option<UsageMetadata>,
}

#[derive(Debug, Deserialize)]
struct UsageMetadata {
    token_usage: Option<Vec<TokenUsage>>,
    #[allow(dead_code)]
    credits_spent: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct TokenUsage {
    // Old format
    total_tokens: Option<u64>,
    // New format (current Warp)
    warp_tokens: Option<u64>,
    byok_tokens: Option<u64>,
}

pub struct WarpProvider;

impl WarpProvider {
    pub fn new() -> Self {
        Self
    }

    fn get_time_ranges() -> (TimeRange, TimeRange, TimeRange) {
        get_local_time_ranges()
    }

    fn process_database(db_path: &Path, stats: &mut UsageStats, ranges: &(TimeRange, TimeRange, TimeRange)) -> Result<()> {
        with_db_snapshot(db_path, |snapshot_path| {
            let conn = Connection::open(snapshot_path)?;

            // First, try to get actual token usage from agent_conversations table
            Self::process_agent_conversations(&conn, stats, ranges)?;

            // Also count AI queries for request count
            Self::process_ai_queries(&conn, stats, ranges)?;

            Ok(())
        })?;

        Ok(())
    }

    fn process_agent_conversations(conn: &Connection, stats: &mut UsageStats, ranges: &(TimeRange, TimeRange, TimeRange)) -> Result<()> {
        let table_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='agent_conversations'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !table_exists {
            return Ok(());
        }

        let mut stmt = conn.prepare(
            "SELECT conversation_data, last_modified_at FROM agent_conversations"
        )?;

        let rows = stmt.query_map([], |row| {
            let data: String = row.get(0)?;
            let modified_at: String = row.get(1)?;
            Ok((data, modified_at))
        })?;

        for row in rows.flatten() {
            let (data_str, modified_at) = row;

            if let Ok(conv_data) = serde_json::from_str::<ConversationData>(&data_str) {
                if let Some(ref metadata) = conv_data.conversation_usage_metadata {
                    let mut total_tokens = 0u64;

                    if let Some(ref token_usage) = metadata.token_usage {
                        for tu in token_usage {
                            // Support both old format (total_tokens) and new format (warp_tokens + byok_tokens)
                            let tokens = tu.total_tokens
                                .or_else(|| {
                                    let warp = tu.warp_tokens.unwrap_or(0);
                                    let byok = tu.byok_tokens.unwrap_or(0);
                                    if warp > 0 || byok > 0 {
                                        Some(warp + byok)
                                    } else {
                                        None
                                    }
                                })
                                .unwrap_or(0);
                            total_tokens += tokens;
                        }
                    }

                    if total_tokens > 0 {
                        let mut usage = UsageData::new();
                        // Split tokens roughly 60/40 input/output
                        usage.input_tokens = (total_tokens as f64 * 0.6) as u64;
                        usage.output_tokens = (total_tokens as f64 * 0.4) as u64;
                        usage.request_count = 1;

                        stats.total.add(&usage);

                        // Parse timestamp
                        if let Some(ts) = Self::parse_warp_timestamp(&modified_at) {
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

        Ok(())
    }

    fn process_ai_queries(conn: &Connection, stats: &mut UsageStats, ranges: &(TimeRange, TimeRange, TimeRange)) -> Result<()> {
        let table_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='ai_queries'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !table_exists {
            return Ok(());
        }

        // Count queries for request tracking (tokens are in agent_conversations)
        let mut stmt = conn.prepare(
            "SELECT start_ts FROM ai_queries"
        )?;

        let rows = stmt.query_map([], |row| {
            let start_ts: String = row.get(0)?;
            Ok(start_ts)
        })?;

        let mut total_queries = 0u64;
        let mut today_queries = 0u64;
        let mut week_queries = 0u64;
        let mut month_queries = 0u64;

        for row in rows.flatten() {
            total_queries += 1;

            if let Some(ts) = Self::parse_warp_timestamp(&row) {
                if ranges.0.contains(ts) {
                    today_queries += 1;
                }
                if ranges.1.contains(ts) {
                    week_queries += 1;
                }
                if ranges.2.contains(ts) {
                    month_queries += 1;
                }
            }
        }

        // Update request counts (don't double count if already set)
        if stats.total.request_count == 0 {
            stats.total.request_count = total_queries;
            stats.today.request_count = today_queries;
            stats.this_week.request_count = week_queries;
            stats.this_month.request_count = month_queries;
        }

        Ok(())
    }

    fn parse_warp_timestamp(ts_str: &str) -> Option<DateTime<Utc>> {
        // Format: "2025-10-11 04:35:19.571758" or "2025-10-11 04:35:19"
        let parts: Vec<&str> = ts_str.split('.').collect();
        let datetime_str = parts.first()?;

        // Parse as naive datetime then convert to UTC
        chrono::NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%d %H:%M:%S")
            .ok()
            .map(|naive| Utc.from_utc_datetime(&naive))
    }
}

#[async_trait]
impl Provider for WarpProvider {
    fn name(&self) -> &'static str {
        "warp"
    }

    fn display_name(&self) -> &'static str {
        "Warp AI"
    }

    async fn is_available(&self) -> bool {
        warp::sqlite_db().map(|p| p.exists()).unwrap_or(false)
    }

    fn get_paths_to_check(&self) -> Vec<String> {
        vec![
            warp::sqlite_db().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
            warp::logs_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        ]
    }

    async fn get_usage(&self, _time_range: Option<&TimeRange>) -> Result<ProviderResult> {
        let db_path = match warp::sqlite_db() {
            Some(p) if p.exists() => p,
            _ => return Ok(ProviderResult::not_found(self.name(), self.display_name())),
        };

        let ranges = Self::get_time_ranges();
        let mut stats = UsageStats::default();

        if let Err(e) = Self::process_database(&db_path, &mut stats, &ranges) {
            return Ok(ProviderResult::error(self.name(), self.display_name(), &e.to_string()));
        }

        Ok(ProviderResult::active(
            self.name(),
            self.display_name(),
            stats,
            &db_path.to_string_lossy(),
        ))
    }
}
