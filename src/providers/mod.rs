//! Provider implementations for all supported AI coding tools

mod claude_code;
mod cursor;
mod github_copilot;
mod cline;
mod windsurf;
mod opencode;
mod openai_codex;
mod gemini_cli;
mod amazon_q;
mod tabnine;
mod gemini_code_assist;
mod sourcegraph_cody;
mod replit;
mod warp;

pub use claude_code::ClaudeCodeProvider;
pub use cursor::CursorProvider;
pub use github_copilot::GitHubCopilotProvider;
pub use cline::ClineProvider;
pub use windsurf::WindsurfProvider;
pub use opencode::OpenCodeProvider;
pub use openai_codex::OpenAICodexProvider;
pub use gemini_cli::GeminiCLIProvider;
pub use amazon_q::AmazonQProvider;
pub use tabnine::TabnineProvider;
pub use gemini_code_assist::GeminiCodeAssistProvider;
pub use sourcegraph_cody::SourcegraphCodyProvider;
pub use replit::ReplitProvider;
pub use warp::WarpProvider;

use crate::types::{ProviderResult, TimeRange};
use anyhow::Result;
use async_trait::async_trait;

/// Base trait for all providers
#[async_trait]
pub trait Provider: Send + Sync {
    /// Get the provider's unique name
    fn name(&self) -> &'static str;

    /// Get the provider's display name
    fn display_name(&self) -> &'static str;

    /// Check if the provider is available (data source exists or can be accessed)
    #[allow(dead_code)]
    async fn is_available(&self) -> bool;

    /// Get usage statistics
    async fn get_usage(&self, time_range: Option<&TimeRange>) -> Result<ProviderResult>;

    /// Get paths to check for doctor command
    fn get_paths_to_check(&self) -> Vec<String>;
}

/// Get all available providers
pub fn get_all_providers() -> Vec<Box<dyn Provider>> {
    vec![
        Box::new(ClaudeCodeProvider::new()),
        Box::new(CursorProvider::new()),
        Box::new(GitHubCopilotProvider::new()),
        Box::new(ClineProvider::new()),
        Box::new(WindsurfProvider::new()),
        Box::new(WarpProvider::new()),
        Box::new(OpenCodeProvider::new()),
        Box::new(OpenAICodexProvider::new()),
        Box::new(GeminiCLIProvider::new()),
        Box::new(AmazonQProvider::new()),
        Box::new(TabnineProvider::new()),
        Box::new(GeminiCodeAssistProvider::new()),
        Box::new(SourcegraphCodyProvider::new()),
        Box::new(ReplitProvider::new()),
    ]
}
