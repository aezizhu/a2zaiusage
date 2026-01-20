//! GitHub Copilot Provider
//! Attempts to use gh CLI auth, falls back to manual token

use super::Provider;
use crate::types::{ProviderResult, TimeRange, UsageStats};
use crate::utils::paths::github_copilot;
use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use std::fs;
use std::process::Command;

#[derive(Debug, Deserialize)]
struct CopilotUserResponse {
    #[allow(dead_code)]
    limited_user_reset_date: Option<String>,
    limited_user_usage: Option<u64>,
    #[allow(dead_code)]
    limited_user_limit: Option<u64>,
    #[allow(dead_code)]
    chat_enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct HostsJson {
    #[serde(rename = "github.com")]
    github_com: Option<GitHubHost>,
}

#[derive(Debug, Deserialize)]
struct GitHubHost {
    oauth_token: Option<String>,
}

pub struct GitHubCopilotProvider;

impl GitHubCopilotProvider {
    pub fn new() -> Self {
        Self
    }

    fn get_token() -> Option<String> {
        // Strategy 1: Check environment variables
        if let Ok(token) = std::env::var("A2Z_GITHUB_TOKEN") {
            return Some(token);
        }
        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            return Some(token);
        }
        if let Ok(token) = std::env::var("GH_TOKEN") {
            return Some(token);
        }

        // Strategy 2: Try gh CLI
        if let Some(token) = Self::get_gh_cli_token() {
            return Some(token);
        }

        // Strategy 3: Check hosts.json
        if let Some(token) = Self::get_hosts_token() {
            return Some(token);
        }

        None
    }

    fn get_gh_cli_token() -> Option<String> {
        let output = Command::new("gh")
            .args(["auth", "token"])
            .output()
            .ok()?;

        if output.status.success() {
            let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if token.starts_with("gh") {
                return Some(token);
            }
        }

        None
    }

    fn get_hosts_token() -> Option<String> {
        let hosts_path = github_copilot::hosts_file()?;

        if !hosts_path.exists() {
            return None;
        }

        let content = fs::read_to_string(hosts_path).ok()?;
        let hosts: HostsJson = serde_json::from_str(&content).ok()?;

        hosts.github_com?.oauth_token
    }

    async fn fetch_copilot_user(token: &str) -> Option<CopilotUserResponse> {
        let client = reqwest::Client::new();

        let response = client
            .get("https://api.github.com/copilot_internal/user")
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "application/json")
            .header("User-Agent", "a2zusage/1.0")
            .send()
            .await
            .ok()?;

        if !response.status().is_success() {
            return None;
        }

        response.json::<CopilotUserResponse>().await.ok()
    }
}

#[async_trait]
impl Provider for GitHubCopilotProvider {
    fn name(&self) -> &'static str {
        "github-copilot"
    }

    fn display_name(&self) -> &'static str {
        "GitHub Copilot"
    }

    async fn is_available(&self) -> bool {
        Self::get_token().is_some()
    }

    fn get_paths_to_check(&self) -> Vec<String> {
        vec![
            github_copilot::hosts_file().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
            github_copilot::vscode_logs().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        ]
    }

    async fn get_usage(&self, _time_range: Option<&TimeRange>) -> Result<ProviderResult> {
        let token = match Self::get_token() {
            Some(t) => t,
            None => return Ok(ProviderResult::no_key(self.name(), self.display_name())),
        };

        let mut stats = UsageStats::default();

        // Try the internal API for individual usage
        if let Some(user_response) = Self::fetch_copilot_user(&token).await {
            if let Some(usage_count) = user_response.limited_user_usage {
                // The API provides a usage count, but does NOT provide reliable token totals.
                // Report it as request_count only (tokens remain 0).
                stats.this_month.request_count = usage_count;
                stats.total.request_count = usage_count;
            }

            return Ok(ProviderResult::active(
                self.name(),
                self.display_name(),
                stats,
                "GitHub API",
            ));
        }

        // Fallback: check if Copilot is installed by looking for hosts.json
        if github_copilot::hosts_file().map(|p| p.exists()).unwrap_or(false) {
            return Ok(ProviderResult::active(
                self.name(),
                self.display_name(),
                stats,
                "Installed (API data unavailable)",
            ));
        }

        Ok(ProviderResult::not_found(self.name(), self.display_name()))
    }
}
