//! OpenAI Codex Provider
//! Uses OpenAI Usage API to fetch usage data

use super::Provider;
use crate::types::{ProviderResult, TimeRange, UsageData, UsageStats};
use crate::utils::time::get_local_time_ranges;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use reqwest::StatusCode;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct OpenAIUsageResponse {
    data: Option<Vec<UsageEntry>>,
}

#[derive(Debug, Deserialize)]
struct UsageEntry {
    aggregation_timestamp: Option<i64>,
    n_requests: Option<u64>,
    n_context_tokens_total: Option<u64>,
    n_generated_tokens_total: Option<u64>,
}

pub struct OpenAICodexProvider;

/// Result of fetching usage data - includes error details for better reporting
enum FetchResult {
    Success(UsageStats),
    Forbidden,
    Unauthorized,
    NotFound,
    RateLimited,
    NetworkError,
    ParseError,
}

impl OpenAICodexProvider {
    pub fn new() -> Self {
        Self
    }

    fn get_api_key() -> Option<String> {
        std::env::var("A2Z_OPENAI_KEY")
            .or_else(|_| std::env::var("OPENAI_API_KEY"))
            .or_else(|_| std::env::var("OPENAI_KEY"))
            .ok()
    }

    fn get_time_ranges() -> (TimeRange, TimeRange, TimeRange) {
        get_local_time_ranges()
    }

    async fn fetch_usage_data(api_key: &str, ranges: &(TimeRange, TimeRange, TimeRange)) -> FetchResult {
        let client = reqwest::Client::new();

        let start_date = ranges.2.start.format("%Y-%m-%d").to_string();
        let end_date = Utc::now().format("%Y-%m-%d").to_string();

        let response = match client
            .get(format!(
                "https://api.openai.com/v1/organization/usage?start_date={}&end_date={}",
                start_date, end_date
            ))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return FetchResult::NetworkError,
        };

        match response.status() {
            StatusCode::OK => {},
            StatusCode::FORBIDDEN => return FetchResult::Forbidden,
            StatusCode::UNAUTHORIZED => return FetchResult::Unauthorized,
            StatusCode::NOT_FOUND => return FetchResult::NotFound,
            StatusCode::TOO_MANY_REQUESTS => return FetchResult::RateLimited,
            _ => return FetchResult::NetworkError,
        }

        let data: OpenAIUsageResponse = match response.json().await {
            Ok(d) => d,
            Err(_) => return FetchResult::ParseError,
        };
        let mut stats = UsageStats::default();

        if let Some(entries) = data.data {
            for entry in entries {
                let mut usage = UsageData::new();
                usage.input_tokens = entry.n_context_tokens_total.unwrap_or(0);
                usage.output_tokens = entry.n_generated_tokens_total.unwrap_or(0);
                usage.request_count = entry.n_requests.unwrap_or(0);

                stats.total.add(&usage);
                stats.this_month.add(&usage);

                if let Some(ts) = entry.aggregation_timestamp {
                    if let Some(entry_date) = Utc.timestamp_opt(ts, 0).single() {
                        if ranges.0.contains(entry_date) {
                            stats.today.add(&usage);
                        }
                        if ranges.1.contains(entry_date) {
                            stats.this_week.add(&usage);
                        }
                    }
                }
            }
        }

        FetchResult::Success(stats)
    }
}

#[async_trait]
impl Provider for OpenAICodexProvider {
    fn name(&self) -> &'static str {
        "openai-codex"
    }

    fn display_name(&self) -> &'static str {
        "OpenAI Codex"
    }

    async fn is_available(&self) -> bool {
        Self::get_api_key().is_some()
    }

    fn get_paths_to_check(&self) -> Vec<String> {
        vec!["OPENAI_API_KEY environment variable".to_string()]
    }

    async fn get_usage(&self, _time_range: Option<&TimeRange>) -> Result<ProviderResult> {
        let api_key = match Self::get_api_key() {
            Some(k) => k,
            None => return Ok(ProviderResult::no_key(self.name(), self.display_name())),
        };

        let ranges = Self::get_time_ranges();

        match Self::fetch_usage_data(&api_key, &ranges).await {
            FetchResult::Success(stats) => Ok(ProviderResult::active(
                self.name(),
                self.display_name(),
                stats,
                "OpenAI API",
            )),
            FetchResult::Forbidden => Ok(ProviderResult::error(
                self.name(),
                self.display_name(),
                "API key lacks permission to access organization usage data (requires org admin or usage:read scope).",
            )),
            FetchResult::Unauthorized => Ok(ProviderResult::error(
                self.name(),
                self.display_name(),
                "Invalid API key. Please check your OPENAI_API_KEY.",
            )),
            FetchResult::NotFound => Ok(ProviderResult::error(
                self.name(),
                self.display_name(),
                "Usage API endpoint not found. This may require an organization account.",
            )),
            FetchResult::RateLimited => Ok(ProviderResult::error(
                self.name(),
                self.display_name(),
                "Rate limited by OpenAI API. Please try again later.",
            )),
            FetchResult::NetworkError => Ok(ProviderResult::error(
                self.name(),
                self.display_name(),
                "Network error connecting to OpenAI API.",
            )),
            FetchResult::ParseError => Ok(ProviderResult::error(
                self.name(),
                self.display_name(),
                "Failed to parse OpenAI API response.",
            )),
        }
    }
}
