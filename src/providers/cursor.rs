//! Cursor Provider
//! Reads usage data from Cursor's SQLite state database
//! Uses snapshot strategy to avoid SQLITE_BUSY errors

use super::Provider;
use crate::types::{ProviderResult, TimeRange, UsageData, UsageStats};
use crate::utils::db::with_db_snapshot;
use crate::utils::paths::cursor;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Datelike, TimeZone, Utc};
use rusqlite::Connection;
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct ComposerData {
    #[serde(rename = "tokenCount")]
    token_count: Option<TokenCount>,
    #[serde(rename = "createdAt")]
    created_at: Option<serde_json::Value>,
    #[serde(rename = "updatedAt")]
    updated_at: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct TokenCount {
    #[serde(rename = "inputTokens")]
    input_tokens: Option<u64>,
    #[serde(rename = "outputTokens")]
    output_tokens: Option<u64>,
}

/// Chat data structure for counting messages
#[derive(Debug, Deserialize)]
struct ChatData {
    messages: Option<Vec<ChatMessage>>,
}

#[derive(Debug, Deserialize)]
struct ChatMessage {
    role: Option<String>,
}

pub struct CursorProvider;

impl CursorProvider {
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

    fn parse_timestamp_value(ts: &serde_json::Value) -> Option<DateTime<Utc>> {
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
    }

    fn process_database(db_path: &Path, stats: &mut UsageStats, ranges: &(TimeRange, TimeRange, TimeRange)) -> Result<()> {
        with_db_snapshot(db_path, |snapshot_path| {
            let conn = Connection::open(snapshot_path)?;

            // Try ItemTable first
            Self::query_item_table(&conn, stats, ranges)?;

            // Try cursorDiskKV table
            Self::query_cursor_disk_kv(&conn, stats, ranges)?;

            Ok(())
        })?;

        Ok(())
    }

    fn query_item_table(conn: &Connection, stats: &mut UsageStats, ranges: &(TimeRange, TimeRange, TimeRange)) -> Result<()> {
        // Check if table exists
        let table_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='ItemTable'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !table_exists {
            return Ok(());
        }

        // ItemTable might have BLOB values, so try both approaches
        let mut stmt = conn.prepare(
            "SELECT key, value FROM ItemTable WHERE key LIKE '%aichat%' OR key LIKE '%composer%' OR key LIKE '%chat%'"
        )?;

        let rows = stmt.query_map([], |row| {
            let key: String = row.get(0)?;
            // Try as text first, fall back to blob
            let value: String = row.get::<_, String>(1)
                .or_else(|_| row.get::<_, Vec<u8>>(1).map(|b| String::from_utf8_lossy(&b).to_string()))
                .unwrap_or_default();
            Ok((key, value))
        })?;

        for row in rows.flatten() {
            Self::process_key_value_str(&row.0, &row.1, stats, ranges);
        }

        Ok(())
    }

    fn query_cursor_disk_kv(conn: &Connection, stats: &mut UsageStats, ranges: &(TimeRange, TimeRange, TimeRange)) -> Result<()> {
        // Check if table exists
        let table_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='cursorDiskKV'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !table_exists {
            return Ok(());
        }

        // Query composerData entries
        // Note: value column in cursorDiskKV is stored as TEXT, not BLOB
        let mut stmt = conn.prepare(
            "SELECT key, value FROM cursorDiskKV WHERE key LIKE 'composerData:%' OR key LIKE 'composer.%'"
        )?;

        let rows = stmt.query_map([], |row| {
            let key: String = row.get(0)?;
            let value: String = row.get(1)?;  // TEXT not BLOB
            Ok((key, value))
        })?;

        for row in rows.flatten() {
            Self::process_key_value_str(&row.0, &row.1, stats, ranges);
        }

        // Query bubbleId entries (where actual token usage is stored)
        // Note: value column in cursorDiskKV is stored as TEXT, not BLOB
        let mut stmt2 = conn.prepare(
            "SELECT key, value FROM cursorDiskKV WHERE key LIKE 'bubbleId:%'"
        )?;

        let rows2 = stmt2.query_map([], |row| {
            let key: String = row.get(0)?;
            let value: String = row.get(1)?;  // TEXT not BLOB
            Ok((key, value))
        })?;

        for row in rows2.flatten() {
            Self::process_bubble_data_str(&row.1, stats, ranges);
        }

        Ok(())
    }

    fn process_bubble_data_str(value_str: &str, stats: &mut UsageStats, ranges: &(TimeRange, TimeRange, TimeRange)) {
        // Parse as generic JSON first to extract tokenCount
        if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(value_str) {
            if let Some(tc) = json_val.get("tokenCount") {
                let input_tokens = tc.get("inputTokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let output_tokens = tc.get("outputTokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);

                if input_tokens > 0 || output_tokens > 0 {
                    let mut usage = UsageData::new();
                    usage.input_tokens = input_tokens;
                    usage.output_tokens = output_tokens;
                    usage.request_count = 1;
                    stats.total.add(&usage);

                    // Try to get timestamp from createdAt
                    let timestamp = json_val.get("createdAt")
                        .and_then(|v| v.as_str())
                        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                        .map(|dt| dt.with_timezone(&Utc));

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
    }

    fn process_key_value_str(_key: &str, value_str: &str, stats: &mut UsageStats, ranges: &(TimeRange, TimeRange, TimeRange)) {
        // Try to parse as ComposerData
        if let Ok(data) = serde_json::from_str::<ComposerData>(value_str) {
            let mut usage = UsageData::new();

            if let Some(tc) = data.token_count {
                usage.input_tokens = tc.input_tokens.unwrap_or(0);
                usage.output_tokens = tc.output_tokens.unwrap_or(0);
            }

            if usage.input_tokens > 0 || usage.output_tokens > 0 {
                usage.request_count = 1;
                stats.total.add(&usage);

                // Try to get timestamp (can be number or string)
                let timestamp = data.created_at.as_ref()
                    .or(data.updated_at.as_ref())
                    .and_then(|ts| Self::parse_timestamp_value(ts));

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

        // Try to parse as ChatData and count messages
        if let Ok(data) = serde_json::from_str::<ChatData>(value_str) {
            if let Some(messages) = data.messages {
                let assistant_count = messages.iter()
                    .filter(|m| m.role.as_deref() == Some("assistant"))
                    .count() as u64;

                if assistant_count > 0 {
                    let mut usage = UsageData::new();
                    usage.request_count = assistant_count;
                    stats.total.add(&usage);
                }
            }
        }
    }

    fn get_recent_workspaces(workspace_dir: &Path, limit: usize) -> Vec<std::path::PathBuf> {
        let mut workspaces: Vec<(std::path::PathBuf, std::time::SystemTime)> = Vec::new();

        if let Ok(entries) = fs::read_dir(workspace_dir) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    let path = entry.path();
                    let db_path = path.join("state.vscdb");

                    if db_path.exists() {
                        let mtime = fs::metadata(&db_path)
                            .and_then(|m| m.modified())
                            .unwrap_or(std::time::UNIX_EPOCH);
                        workspaces.push((path, mtime));
                    }
                }
            }
        }

        workspaces.sort_by(|a, b| b.1.cmp(&a.1));
        workspaces.into_iter().take(limit).map(|(p, _)| p).collect()
    }
}

#[async_trait]
impl Provider for CursorProvider {
    fn name(&self) -> &'static str {
        "cursor"
    }

    fn display_name(&self) -> &'static str {
        "Cursor"
    }

    async fn is_available(&self) -> bool {
        cursor::global_storage().map(|p| p.exists()).unwrap_or(false)
            || cursor::workspace_storage().map(|p| p.exists()).unwrap_or(false)
    }

    fn get_paths_to_check(&self) -> Vec<String> {
        vec![
            cursor::global_storage().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
            cursor::workspace_storage().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        ]
    }

    async fn get_usage(&self, _time_range: Option<&TimeRange>) -> Result<ProviderResult> {
        let global_db = cursor::global_storage();
        let workspace_dir = cursor::workspace_storage();

        let has_global = global_db.as_ref().map(|p| p.exists()).unwrap_or(false);
        let has_workspace = workspace_dir.as_ref().map(|p| p.exists()).unwrap_or(false);

        if !has_global && !has_workspace {
            return Ok(ProviderResult::not_found(self.name(), self.display_name()));
        }

        let ranges = Self::get_time_ranges();
        let mut stats = UsageStats::default();

        // Process global storage database
        if let Some(ref db_path) = global_db {
            if db_path.exists() {
                let _ = Self::process_database(db_path, &mut stats, &ranges);
            }
        }

        // Process workspace storage databases (most recent first)
        if let Some(ref ws_dir) = workspace_dir {
            if ws_dir.exists() {
                let workspaces = Self::get_recent_workspaces(ws_dir, 10);
                for workspace in workspaces {
                    let db_path = workspace.join("state.vscdb");
                    if db_path.exists() {
                        let _ = Self::process_database(&db_path, &mut stats, &ranges);
                    }
                }
            }
        }

        let data_source = global_db
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "Cursor".to_string());

        Ok(ProviderResult::active(
            self.name(),
            self.display_name(),
            stats,
            &data_source,
        ))
    }
}
