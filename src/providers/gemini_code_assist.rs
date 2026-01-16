//! Gemini Code Assist Provider
//! Reads from VS Code extension storage

use super::Provider;
use crate::types::{ProviderResult, TimeRange, UsageStats};
use crate::utils::paths::gemini_code_assist;
use anyhow::Result;
use async_trait::async_trait;
use std::fs;

pub struct GeminiCodeAssistProvider;

impl GeminiCodeAssistProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Provider for GeminiCodeAssistProvider {
    fn name(&self) -> &'static str {
        "gemini-code-assist"
    }

    fn display_name(&self) -> &'static str {
        "Gemini Assist"
    }

    async fn is_available(&self) -> bool {
        gemini_code_assist::vscode_extension().map(|p| p.exists()).unwrap_or(false)
    }

    fn get_paths_to_check(&self) -> Vec<String> {
        vec![
            gemini_code_assist::vscode_extension().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        ]
    }

    async fn get_usage(&self, _time_range: Option<&TimeRange>) -> Result<ProviderResult> {
        let extension_dir = match gemini_code_assist::vscode_extension() {
            Some(p) if p.exists() => p,
            _ => return Ok(ProviderResult::not_found(self.name(), self.display_name())),
        };

        // Check if there are any state files
        let has_data = fs::read_dir(&extension_dir)
            .map(|entries| entries.count() > 0)
            .unwrap_or(false);

        if has_data {
            // Extension is installed but detailed parsing would require
            // understanding Google's internal format
            return Ok(ProviderResult::active(
                self.name(),
                self.display_name(),
                UsageStats::default(),
                "Installed (see Google Cloud Console for usage)",
            ));
        }

        Ok(ProviderResult::not_found(self.name(), self.display_name()))
    }
}
