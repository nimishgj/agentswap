# AgentSwap

Transfer conversation history between AI coding agents seamlessly.

## The Problem

You're mid-conversation with Claude Code, 50 messages deep into debugging a complex issue, and you want to try Gemini CLI's approach — but there's no way to bring your conversation context along. You'd have to start from scratch, re-explain the problem, and lose all the context you've built up.

AgentSwap solves this. It reads conversations from one agent and writes them into another agent's native format, so you can pick up exactly where you left off.

## Supported Agents

| Agent | Read | Write | Resume |
|-------|------|-------|--------|
| Claude Code | Yes | Yes | `claude --resume <id>` |
| Gemini CLI | Yes | Yes | `gemini --resume <id>` |
| Codex CLI | Yes | Yes | `codex --resume <id>` |

## Features

- **Native format conversion** — writes directly into each agent's storage format so `--resume` works out of the box
- **Tool name mapping** — automatically translates tool calls between agents (e.g., Claude's `Bash` becomes Gemini's `run_shell_command`)
- **Conversation preview** — Tab to preview full conversation content before transferring
- **Vim-style navigation** — `j`/`k`, `G`/`gg` for fast browsing
- **Copy resume command** — press `c` to copy the exact resume command to clipboard

## Quick Start

### Install

```bash
# Clone and build
git clone https://github.com/nimishgj/agentswap.git
cd agentswap
cargo build --release

# Binary is at target/release/agentswap-tui
```

### Usage

```bash
# Launch the TUI
cargo run -p agentswap-tui

# Or run the built binary directly
./target/release/agentswap-tui
```

### Controls

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate up/down |
| `G` | Jump to bottom |
| `gg` | Jump to top |
| `Enter` | Select / confirm |
| `Tab` | Toggle conversation preview |
| `c` | Copy resume command |
| `Esc` | Go back |
| `q` | Quit |

### Workflow

1. Select the **source agent** (where your conversation lives)
2. Browse and select a **conversation**
3. Select the **target agent** (where you want to transfer it)
4. AgentSwap writes the conversation in the target's native format
5. Press `c` to copy the resume command, then run it in your terminal

## Architecture

AgentSwap is a Cargo workspace with 5 crates:

```
crates/
  agentswap-core/     # Universal Conversation Format types, adapter trait, tool mapping
  agentswap-claude/   # Claude Code adapter (JSONL)
  agentswap-gemini/   # Gemini CLI adapter (JSON + SHA-256 project hashing)
  agentswap-codex/    # Codex CLI adapter (SQLite + JSONL)
  agentswap-tui/      # Terminal UI (ratatui)
```

Each adapter implements the `AgentAdapter` trait:

```rust
pub trait AgentAdapter {
    fn list_conversations(&self) -> Result<Vec<ConversationMeta>>;
    fn read_conversation(&self, id: &str) -> Result<Conversation>;
    fn write_conversation(&self, conversation: &Conversation) -> Result<String>;
    fn render_prompt(&self, conversation: &Conversation) -> String;
}
```

## Development

```bash
make test    # Run all tests
make lint    # Run clippy and format check
make ci      # Run lint + test (used in CI)
```

## License

[MIT](LICENSE)
