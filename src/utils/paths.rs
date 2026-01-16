//! Cross-platform path utilities for all AI tool data sources

use std::path::PathBuf;

/// Get home directory
pub fn home_dir() -> Option<PathBuf> {
    dirs::home_dir()
}

/// Get application data directory
#[allow(dead_code)]
pub fn app_data_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        dirs::data_dir()
    }
    #[cfg(target_os = "macos")]
    {
        dirs::data_dir() // ~/Library/Application Support
    }
    #[cfg(target_os = "linux")]
    {
        dirs::data_local_dir() // ~/.local/share
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        dirs::data_dir()
    }
}

/// Get config directory
#[allow(dead_code)]
pub fn config_dir() -> Option<PathBuf> {
    dirs::config_dir()
}

/// Path configurations for Claude Code
pub mod claude_code {
    use super::*;

    pub fn projects_dir() -> Option<PathBuf> {
        home_dir().map(|h| h.join(".claude").join("projects"))
    }

    pub fn config_file() -> Option<PathBuf> {
        home_dir().map(|h| h.join(".claude.json"))
    }

    #[allow(dead_code)]
    pub fn settings_file() -> Option<PathBuf> {
        home_dir().map(|h| h.join(".claude").join("settings.json"))
    }
}

/// Path configurations for Cursor
pub mod cursor {
    use super::*;

    pub fn global_storage() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            dirs::data_dir().map(|d| d.join("Cursor").join("User").join("globalStorage").join("state.vscdb"))
        }
        #[cfg(target_os = "macos")]
        {
            dirs::data_dir().map(|d| d.join("Cursor").join("User").join("globalStorage").join("state.vscdb"))
        }
        #[cfg(target_os = "linux")]
        {
            config_dir().map(|d| d.join("Cursor").join("User").join("globalStorage").join("state.vscdb"))
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            None
        }
    }

    pub fn workspace_storage() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            dirs::data_dir().map(|d| d.join("Cursor").join("User").join("workspaceStorage"))
        }
        #[cfg(target_os = "macos")]
        {
            dirs::data_dir().map(|d| d.join("Cursor").join("User").join("workspaceStorage"))
        }
        #[cfg(target_os = "linux")]
        {
            config_dir().map(|d| d.join("Cursor").join("User").join("workspaceStorage"))
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            None
        }
    }
}

/// Path configurations for GitHub Copilot
pub mod github_copilot {
    use super::*;

    pub fn hosts_file() -> Option<PathBuf> {
        home_dir().map(|h| h.join(".config").join("github-copilot").join("hosts.json"))
    }

    pub fn vscode_logs() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            dirs::data_dir().map(|d| d.join("Code").join("logs"))
        }
        #[cfg(target_os = "macos")]
        {
            dirs::data_dir().map(|d| d.join("Code").join("logs"))
        }
        #[cfg(target_os = "linux")]
        {
            config_dir().map(|d| d.join("Code").join("logs"))
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            None
        }
    }
}

/// Path configurations for Windsurf
pub mod windsurf {
    use super::*;

    pub fn cascade_dir() -> Option<PathBuf> {
        home_dir().map(|h| h.join(".codeium").join("windsurf").join("cascade"))
    }

    pub fn memories_dir() -> Option<PathBuf> {
        home_dir().map(|h| h.join(".codeium").join("windsurf").join("memories"))
    }

    pub fn config_dir() -> Option<PathBuf> {
        home_dir().map(|h| h.join(".codeium"))
    }
}

/// Path configurations for OpenCode
pub mod opencode {
    use super::*;

    pub fn storage_dir() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            dirs::data_dir().map(|d| d.join("opencode").join("storage").join("message"))
        }
        #[cfg(not(target_os = "windows"))]
        {
            home_dir().map(|h| h.join(".local").join("share").join("opencode").join("storage").join("message"))
        }
    }
}

/// Path configurations for Amazon Q Developer
pub mod amazon_q {
    use super::*;

    pub fn logs_file() -> Option<PathBuf> {
        home_dir().map(|h| h.join(".aws").join("q").join("q_developer_log.txt"))
    }

    pub fn config_file() -> Option<PathBuf> {
        home_dir().map(|h| h.join(".aws").join("config"))
    }
}

/// Path configurations for Gemini CLI
pub mod gemini_cli {
    use super::*;

    pub fn telemetry_file() -> Option<PathBuf> {
        home_dir().map(|h| h.join(".gemini").join("telemetry.log"))
    }

    pub fn config_dir() -> Option<PathBuf> {
        home_dir().map(|h| h.join(".gemini"))
    }
}

/// Path configurations for Tabnine
pub mod tabnine {
    use super::*;

    pub fn logs_dir() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            dirs::data_dir().map(|d| d.join("TabNine").join("logs"))
        }
        #[cfg(target_os = "macos")]
        {
            dirs::data_dir().map(|d| d.join("TabNine").join("logs"))
        }
        #[cfg(target_os = "linux")]
        {
            home_dir().map(|h| h.join(".local").join("share").join("TabNine").join("logs"))
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            None
        }
    }
}

/// Path configurations for Cline
pub mod cline {
    use super::*;

    fn vscode_global_storage() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            dirs::data_dir().map(|d| d.join("Code").join("User").join("globalStorage"))
        }
        #[cfg(target_os = "macos")]
        {
            dirs::data_dir().map(|d| d.join("Code").join("User").join("globalStorage"))
        }
        #[cfg(target_os = "linux")]
        {
            config_dir().map(|d| d.join("Code").join("User").join("globalStorage"))
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            None
        }
    }

    pub fn original_tasks_dir() -> Option<PathBuf> {
        vscode_global_storage().map(|d| d.join("saoudrizwan.claude-dev").join("tasks"))
    }

    pub fn roo_code_tasks_dir() -> Option<PathBuf> {
        vscode_global_storage().map(|d| d.join("rooveterinary.roo-cline").join("tasks"))
    }

    pub fn roo_usage_tracking() -> Option<PathBuf> {
        home_dir().map(|h| h.join(".roo").join("usage-tracking.json"))
    }
}

/// Path configurations for Sourcegraph Cody
pub mod sourcegraph_cody {
    use super::*;

    pub fn vscode_extension() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            dirs::data_dir().map(|d| d.join("Code").join("User").join("globalStorage").join("sourcegraph.cody-ai"))
        }
        #[cfg(target_os = "macos")]
        {
            dirs::data_dir().map(|d| d.join("Code").join("User").join("globalStorage").join("sourcegraph.cody-ai"))
        }
        #[cfg(target_os = "linux")]
        {
            config_dir().map(|d| d.join("Code").join("User").join("globalStorage").join("sourcegraph.cody-ai"))
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            None
        }
    }
}

/// Path configurations for Warp Terminal
pub mod warp {
    use super::*;

    pub fn sqlite_db() -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            home_dir().map(|h| {
                h.join("Library")
                    .join("Group Containers")
                    .join("2BBY89MBSN.dev.warp")
                    .join("Library")
                    .join("Application Support")
                    .join("dev.warp.Warp-Stable")
                    .join("warp.sqlite")
            })
        }
        #[cfg(target_os = "windows")]
        {
            dirs::data_dir().map(|d| d.join("Warp").join("warp.sqlite"))
        }
        #[cfg(target_os = "linux")]
        {
            home_dir().map(|h| h.join(".local").join("share").join("warp").join("warp.sqlite"))
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            None
        }
    }

    pub fn logs_dir() -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            home_dir().map(|h| h.join("Library").join("Logs"))
        }
        #[cfg(target_os = "windows")]
        {
            dirs::data_dir().map(|d| d.join("Warp").join("logs"))
        }
        #[cfg(target_os = "linux")]
        {
            home_dir().map(|h| h.join(".local").join("share").join("warp").join("logs"))
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            None
        }
    }

    #[allow(dead_code)]
    pub fn config_dir() -> Option<PathBuf> {
        home_dir().map(|h| h.join(".warp"))
    }
}
