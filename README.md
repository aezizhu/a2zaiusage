# a2zusage - AI Coding Tools Usage Tracker

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
  <a href="https://www.npmjs.com/package/a2zusage"><img src="https://img.shields.io/npm/v/a2zusage?style=flat-square" alt="npm version"></a>
  <a href="https://www.npmjs.com/package/a2zusage"><img src="https://img.shields.io/npm/dm/a2zusage?style=flat-square" alt="npm downloads"></a>
  <a href="https://github.com/aezizhu/a2zaiusage/stargazers"><img src="https://img.shields.io/github/stars/aezizhu/a2zaiusage?style=flat-square" alt="GitHub stars"></a>
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
│ Windsurf       │ ~ Unsup  │ -             │ -           │ -           │ -             │
│ Warp AI        │ ✓ Active │ -             │ -           │ 8.2M tokens │ 167.1M tokens │
│ Gemini CLI     │ ✓ Active │ -             │ 72K tokens  │ 101K tokens │ 48.1M tokens  │
│ GitHub Copilot │ ○ N/A    │ -             │ -           │ -           │ -             │
│ ...            │ ...      │ ...           │ ...         │ ...         │ ...           │
╰────────────────┴──────────┴───────────────┴─────────────┴─────────────┴───────────────╯
```

## Installation

### Using npx (Recommended)

```bash
npx a2zusage@latest
```

### Install Globally

```bash
npm install -g a2zusage
```

### Using Cargo (Rust)

```bash
cargo install a2zusage
```

### From Source

```bash
git clone https://github.com/aezizhu/a2zaiusage.git
cd a2zaiusage
cargo build --release
./target/release/a2zusage
```

## Supported Tools

a2zusage supports **14+ AI coding tools** out of the box:

| Tool | Data Source | What’s Accurate |
|------|-------------|----------------|
| **Claude Code** | Local JSONL (`~/.claude/projects/`) | ✅ Exact token counts (input/output + cache tokens when present) |
| **Cursor** | SQLite database | ✅ Exact token counts (when present in DB) |
| **GitHub Copilot** | GitHub API + Local logs | ⚠️ Usage count / requests only (GitHub does not expose reliable token totals here) |
|| **Windsurf** | Cascade logs (`~/.codeium/`) | ❌ Encrypted: Data stored in encrypted `.pb` files. Use windsurf.ai dashboard or Settings > Usage |
| **Warp AI** | SQLite database | ✅ Total tokens (Warp does not expose a reliable input/output split) |
| **Cline / Roo Code** | VS Code extension storage | ✅ Exact token counts (when stored by the extension) |
| **OpenCode** | Local JSON files | ✅ Exact token counts (when present in session/message usage fields) |
| **OpenAI Codex** | OpenAI Usage API | ✅ Exact token counts (requires API key + org access) |
|| **Gemini CLI** | Native sessions (`~/.gemini/tmp/`) | ✅ Exact token counts from native session files (no setup required) |
| **Amazon Q Developer** | Local logs | ⚠️ Best-effort: logs may not contain token totals |
| **Tabnine** | Local logs | ⚠️ Partial: uses explicit token fields when present; no invented prompt/context tokens |
| **Gemini Code Assist** | Google Cloud | ⚠️ Not implemented in this repo yet |
| **Sourcegraph Cody** | VS Code extension | ⚠️ Token counts only when present; otherwise request_count only |
| **Replit Ghostwriter** | Web link | 🔗 Link Only |

## Usage

### Basic Usage

```bash
# Query all detected AI tools
a2zusage

# Or using npx
npx a2zusage@latest
```

### Filter by Tool

```bash
a2zusage -t claude-code    # Only Claude Code
a2zusage -t cursor         # Only Cursor
a2zusage -t warp           # Only Warp AI
```

### Output Formats

```bash
a2zusage -f table    # Pretty table (default)
a2zusage -f json     # JSON output
a2zusage -f csv      # CSV output
```

### Other Commands

```bash
a2zusage doctor      # Check paths and configuration
a2zusage list        # List all supported tools
a2zusage --help      # Show help
a2zusage -v          # Verbose mode with data sources
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

We used a2zusage in our hiring process and found it **incredibly effective** at identifying candidates who are genuinely productive with AI tools. Now we're open-sourcing it for the community.

### 📊 Quantify Your AI Usage

Ever wondered:
- How many tokens you've used across all AI coding tools?
- Which tool you use most frequently?
- How your usage has changed over time?

a2zusage answers all these questions in seconds.

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

If you use a2zusage in your research, hiring process, or project, please cite:

```bibtex
@software{a2zusage,
  author = {aezizhu},
  title = {a2zusage: AI Coding Tools Usage Tracker},
  url = {https://github.com/aezizhu/a2zaiusage},
  year = {2026}
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
