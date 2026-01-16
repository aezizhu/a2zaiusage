# ccusage - AI Coding Tools Usage Tracker

<p align="center">
  <strong>Query usage statistics from ALL your AI coding tools in one command</strong>
</p>

<p align="center">
  <a href="#installation">Installation</a> •
  <a href="#supported-tools">Supported Tools</a> •
  <a href="#usage">Usage</a> •
  <a href="#why-this-tool">Why This Tool</a> •
  <a href="#contributing">Contributing</a>
</p>

<p align="center">
  <img src="https://img.shields.io/npm/v/ccusage?style=flat-square" alt="npm version">
  <img src="https://img.shields.io/badge/license-Apache%202.0-blue?style=flat-square" alt="license">
  <img src="https://img.shields.io/badge/tools-14%2B-green?style=flat-square" alt="supported tools">
</p>

---

```
╭────────────────┬──────────┬───────────────┬─────────────┬─────────────┬───────────────╮
│ Tool           │ Status   │ Today         │ This Week   │ This Month  │ Total         │
├────────────────┼──────────┼───────────────┼─────────────┼─────────────┼───────────────┤
│ Claude Code    │ ✓ Active │ 321K tokens   │ 1.4M tokens │ 6.1M tokens │ 24.2M tokens  │
│ Cursor         │ ✓ Active │ -             │ -           │ -           │ 32.5K tokens  │
│ Windsurf       │ ✓ Active │ -             │ -           │ -           │ 7.1M tokens   │
│ Warp AI        │ ✓ Active │ -             │ -           │ 8.2M tokens │ 167.1M tokens │
│ GitHub Copilot │ ○ N/A    │ -             │ -           │ -           │ -             │
│ ...            │ ...      │ ...           │ ...         │ ...         │ ...           │
╰────────────────┴──────────┴───────────────┴─────────────┴─────────────┴───────────────╯
```

## Installation

### Using npx (Recommended)

```bash
npx ccusage@latest
```

### Install Globally

```bash
npm install -g ccusage
```

### Using Cargo (Rust)

```bash
cargo install ccusage
```

### From Source

```bash
git clone https://github.com/aezizhu/a2zaiusage.git
cd a2zaiusage
cargo build --release
./target/release/ccusage
```

## Supported Tools

ccusage supports **14+ AI coding tools** out of the box:

| Tool | Data Source | Status |
|------|-------------|--------|
| **Claude Code** | Local JSONL (`~/.claude/projects/`) | ✅ Full Support |
| **Cursor** | SQLite database | ✅ Full Support |
| **GitHub Copilot** | GitHub API + Local logs | ✅ Full Support |
| **Windsurf** | Cascade logs (`~/.codeium/`) | ✅ Full Support |
| **Warp AI** | SQLite database | ✅ Full Support |
| **Cline / Roo Code** | VS Code extension storage | ✅ Full Support |
| **OpenCode** | Local JSON files | ✅ Full Support |
| **OpenAI Codex** | OpenAI Usage API | ✅ Full Support |
| **Gemini CLI** | Local telemetry (`~/.gemini/`) | ✅ Full Support |
| **Amazon Q Developer** | Local logs | ✅ Full Support |
| **Tabnine** | Local logs | ✅ Full Support |
| **Gemini Code Assist** | Google Cloud | ✅ Full Support |
| **Sourcegraph Cody** | VS Code extension | ✅ Full Support |
| **Replit Ghostwriter** | Web link | 🔗 Link Only |

## Usage

### Basic Usage

```bash
# Query all detected AI tools
ccusage

# Or using npx
npx ccusage@latest
```

### Filter by Tool

```bash
ccusage -t claude-code    # Only Claude Code
ccusage -t cursor         # Only Cursor
ccusage -t warp           # Only Warp AI
```

### Output Formats

```bash
ccusage -f table    # Pretty table (default)
ccusage -f json     # JSON output
ccusage -f csv      # CSV output
```

### Other Commands

```bash
ccusage doctor      # Check paths and configuration
ccusage list        # List all supported tools
ccusage --help      # Show help
ccusage -v          # Verbose mode with data sources
```

### JSON Output Example

```json
[
  {
    "name": "claude-code",
    "display_name": "Claude Code",
    "status": "active",
    "usage": {
      "today": { "input_tokens": 306929, "output_tokens": 14196, "request_count": 2709 },
      "this_week": { "input_tokens": 1106656, "output_tokens": 278247, "request_count": 4663 },
      "this_month": { "input_tokens": 3902283, "output_tokens": 2166653, "request_count": 32956 },
      "total": { "input_tokens": 13648429, "output_tokens": 10593772, "request_count": 89580 }
    },
    "data_source": "/Users/you/.claude/projects"
  }
]
```

## Why This Tool?

### 🎯 Built for Hiring AI-Native Developers

We created this tool because we needed a way to **identify developers who truly embrace AI-assisted coding** (what we call "Vibe Coding").

When hiring, we found that:

- **Resume skills don't tell the whole story** - Many claim AI proficiency but rarely use it
- **Token usage reveals real habits** - High usage = deep integration into daily workflow
- **Multiple tool usage shows adaptability** - The best devs try everything and use what works

We used ccusage in our hiring process and found it **incredibly effective** at identifying candidates who are genuinely productive with AI tools. Now we're open-sourcing it for the community.

### 📊 Quantify Your AI Usage

Ever wondered:
- How many tokens you've used across all AI coding tools?
- Which tool you use most frequently?
- How your usage has changed over time?

ccusage answers all these questions in seconds.

### 🔒 Privacy-First Design

- **100% Local** - All data is read from local files on your machine
- **No Network Calls** - Unless you explicitly use API-based providers
- **No Data Collection** - We never see your usage data
- **Open Source** - Audit the code yourself

## Environment Variables

For API-based providers, set these environment variables:

```bash
# GitHub Copilot (or use `gh auth login`)
export GITHUB_TOKEN=ghp_xxx

# OpenAI Codex
export OPENAI_API_KEY=sk-xxx

# AWS (for Amazon Q)
export AWS_PROFILE=default
```

## Cross-Platform Support

| Platform | Status |
|----------|--------|
| macOS (Intel) | ✅ Supported |
| macOS (Apple Silicon) | ✅ Supported |
| Linux (x64) | ✅ Supported |
| Linux (ARM64) | ✅ Supported |
| Windows (x64) | ✅ Supported |

## Contributing

We welcome contributions! Here's how you can help:

### Add Support for More Tools

Know of an AI coding tool we don't support? We'd love to add it!

1. Fork the repository
2. Add a new provider in `src/providers/`
3. Update the provider registry in `src/providers/mod.rs`
4. Submit a pull request

### Improve Existing Providers

- Better token parsing
- More accurate cost estimation
- Additional data sources

### Report Issues

Found a bug or have a suggestion? [Open an issue](https://github.com/aezizhu/a2zaiusage/issues)!

## Citation

If you use ccusage in your research, hiring process, or project, please cite:

```bibtex
@software{ccusage,
  author = {aezizhu},
  title = {ccusage: AI Coding Tools Usage Tracker},
  url = {https://github.com/aezizhu/a2zaiusage},
  year = {2025}
}
```

Or simply link to: `https://github.com/aezizhu/a2zaiusage`

## License

Apache License 2.0 - see [LICENSE](LICENSE) for details.

---

<p align="center">
  <strong>Built with ❤️ for the AI-native developer community</strong>
</p>

<p align="center">
  <a href="https://github.com/aezizhu">@aezizhu</a>
</p>
