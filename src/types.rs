//! Core types for a2zusage

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Usage data for a single time period
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageData {
    /// Total input tokens
    pub input_tokens: u64,
    /// Total output tokens
    pub output_tokens: u64,
    /// Cache read tokens (for providers that support it)
    #[serde(default)]
    pub cache_read_tokens: u64,
    /// Cache write tokens (for providers that support it)
    #[serde(default)]
    pub cache_write_tokens: u64,
    /// Number of requests/interactions
    #[serde(default)]
    pub request_count: u64,
    /// Estimated cost in USD
    #[serde(default)]
    pub estimated_cost: f64,
}

impl UsageData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens + self.cache_read_tokens + self.cache_write_tokens
    }

    pub fn add(&mut self, other: &UsageData) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.cache_read_tokens += other.cache_read_tokens;
        self.cache_write_tokens += other.cache_write_tokens;
        self.request_count += other.request_count;
        self.estimated_cost += other.estimated_cost;
    }
}

/// Usage statistics across time periods
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageStats {
    pub today: UsageData,
    pub this_week: UsageData,
    pub this_month: UsageData,
    pub total: UsageData,
}

/// Provider status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderStatus {
    /// Data found and parsed successfully
    Active,
    /// Tool installed, but usage exists in an unsupported/unknown format so accurate totals can't be computed
    Unsupported,
    /// Tool not installed or no data found
    NotFound,
    /// API key required but not provided
    NoKey,
    /// Manual authentication required (e.g., Replit)
    AuthRequired,
    /// Parse/read error occurred
    Error,
    /// Can only provide a link to web UI
    LinkOnly,
}

impl fmt::Display for ProviderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProviderStatus::Active => write!(f, "Active"),
            ProviderStatus::Unsupported => write!(f, "Unsupported"),
            ProviderStatus::NotFound => write!(f, "N/A"),
            ProviderStatus::NoKey => write!(f, "No Key"),
            ProviderStatus::AuthRequired => write!(f, "Auth Required"),
            ProviderStatus::Error => write!(f, "Error"),
            ProviderStatus::LinkOnly => write!(f, "Link Only"),
        }
    }
}

/// Result from a provider query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderResult {
    /// Provider identifier
    pub name: String,
    /// Display name for output
    pub display_name: String,
    /// Provider status
    pub status: ProviderStatus,
    /// Usage statistics (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<UsageStats>,
    /// Error message (if status is Error)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Data source path or description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_source: Option<String>,
}

impl ProviderResult {
    pub fn not_found(name: &str, display_name: &str) -> Self {
        Self {
            name: name.to_string(),
            display_name: display_name.to_string(),
            status: ProviderStatus::NotFound,
            usage: None,
            error: None,
            data_source: None,
        }
    }

    pub fn error(name: &str, display_name: &str, error: &str) -> Self {
        Self {
            name: name.to_string(),
            display_name: display_name.to_string(),
            status: ProviderStatus::Error,
            usage: None,
            error: Some(error.to_string()),
            data_source: None,
        }
    }

    pub fn no_key(name: &str, display_name: &str) -> Self {
        Self {
            name: name.to_string(),
            display_name: display_name.to_string(),
            status: ProviderStatus::NoKey,
            usage: None,
            error: None,
            data_source: None,
        }
    }

    pub fn unsupported(name: &str, display_name: &str, message: &str, data_source: Option<&str>) -> Self {
        Self {
            name: name.to_string(),
            display_name: display_name.to_string(),
            status: ProviderStatus::Unsupported,
            usage: None,
            error: Some(message.to_string()),
            data_source: data_source.map(|s| s.to_string()),
        }
    }

    pub fn link_only(name: &str, display_name: &str, url: &str) -> Self {
        Self {
            name: name.to_string(),
            display_name: display_name.to_string(),
            status: ProviderStatus::LinkOnly,
            usage: None,
            error: None,
            data_source: Some(url.to_string()),
        }
    }

    pub fn active(name: &str, display_name: &str, usage: UsageStats, data_source: &str) -> Self {
        Self {
            name: name.to_string(),
            display_name: display_name.to_string(),
            status: ProviderStatus::Active,
            usage: Some(usage),
            error: None,
            data_source: Some(data_source.to_string()),
        }
    }
}

/// Time range for filtering usage data
#[derive(Debug, Clone)]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl TimeRange {
    pub fn contains(&self, time: DateTime<Utc>) -> bool {
        time >= self.start && time <= self.end
    }
}

/// Provider metadata for doctor command
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ProviderMeta {
    pub name: &'static str,
    pub display_name: &'static str,
    pub data_source_type: DataSourceType,
    pub paths: Vec<String>,
    pub requires_auth: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum DataSourceType {
    LocalFile,
    LocalDb,
    Api,
    LinkOnly,
}

/// CLI output format
#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
pub enum OutputFormat {
    #[default]
    Table,
    Json,
    Csv,
}
