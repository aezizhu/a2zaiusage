//! Replit Ghostwriter Provider
//! Implemented as "Link Only" mode since it requires web authentication

use super::Provider;
use crate::types::{ProviderResult, TimeRange};
use anyhow::Result;
use async_trait::async_trait;

pub struct ReplitProvider;

impl ReplitProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Provider for ReplitProvider {
    fn name(&self) -> &'static str {
        "replit"
    }

    fn display_name(&self) -> &'static str {
        "Replit"
    }

    async fn is_available(&self) -> bool {
        // Always return true to show in the list with link_only status
        // User can see they need to check the web UI
        true
    }

    fn get_paths_to_check(&self) -> Vec<String> {
        vec!["https://replit.com/usage (web only)".to_string()]
    }

    async fn get_usage(&self, _time_range: Option<&TimeRange>) -> Result<ProviderResult> {
        // Replit is web-only and requires session cookie authentication
        // We provide a link to the usage page instead
        Ok(ProviderResult::link_only(
            self.name(),
            self.display_name(),
            "https://replit.com/usage",
        ))
    }
}
