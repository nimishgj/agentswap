# AgentSwap

Transfer conversation history between AI coding agents seamlessly.

## The Problem

You're 50 messages deep into a debugging session with Claude Code and you want to try Gemini CLI's approach. What are your options?

- **Copy-paste the conversation?** Good luck — a real session is easily 100,000+ lines with tool calls, file contents, and outputs. Most of it gets truncated or lost.
- **Start from scratch?** You lose all the context, file changes, reasoning, and tool history that got you here.
- **Export and summarize?** You lose the structured tool calls, the exact inputs/outputs, and the agent can't actually resume from a summary.

The core issue: each agent stores conversations in its own format (Claude uses JSONL with UUID chains, Gemini uses JSON with SHA-256 project hashing, Codex uses SQLite + JSONL rollouts). There's no way to move between them.

AgentSwap reads conversations from one agent's native storage, translates tool names and parameters, and writes them into another agent's native format — so `--resume` just works.

## Story Behind Building This

I was deep into a coding session on Codex's free plan — had a massive conversation going, tons of context built up — and then the tokens ran out. No more free credits. The conversation was right there on disk, but I couldn't continue it.

"Fine, I'll just copy-paste it into Claude Code." I opened the conversation and... it was enormous. Hundreds of tool calls, file reads, outputs, reasoning blocks. Way too much to copy. And even if I could, half of it would be meaningless without the structured tool call format the other agent expects.

So I sat there thinking — why isn't there a simple way to just move a conversation from one agent to another? They all store the data locally. Someone just needs to translate between the formats.

That's how AgentSwap started. But it turns out this isn't just a "ran out of tokens" problem. It's useful anytime you want to:

- **Switch agents mid-task** — maybe Claude is better at reasoning through your bug, but Gemini has the tool you need
- **Survive outages** — if one agent's API is down, move your conversation to another and keep working
- **Compare approaches** — hand the same conversation to a different agent and see how it picks up from there

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

## How It Works

Every agent stores conversations differently — Claude uses JSONL with UUID-chained events, Gemini uses JSON files keyed by SHA-256 project hashes, Codex uses SQLite alongside JSONL rollout files. Translating directly between every pair would be a nightmare.

Instead, AgentSwap uses a **Universal Conversation Format (UCF)** as an intermediate representation. Every adapter only needs to know two things: how to read its agent's format into UCF, and how to write UCF back out.

```
┌──────────────┐      ┌─────────────────────────┐      ┌──────────────┐
│  Claude Code │      │  Universal Conversation │      │  Claude Code │
│   (JSONL)    │─────>│        Format (UCF)     │─────>│   (JSONL)    │
├──────────────┤      │                         │      ├──────────────┤
│  Gemini CLI  │─────>│  Messages, tool calls,  │─────>│  Gemini CLI  │
│   (JSON)     │      │  metadata, file changes │      │   (JSON)     │
├──────────────┤      │                         │      ├──────────────┤
│  Codex CLI   │─────>│  + tool name/param      │─────>│  Codex CLI   │
│(SQLite+JSONL)│      │    mapping layer        │      │(SQLite+JSONL)│
└──────────────┘      └─────────────────────────┘      └──────────────┘
   read(id) ──>            Conversation              ──> write(conv)
```

Tool names and parameters are also mapped automatically — Claude's `Bash` becomes Gemini's `run_shell_command`, input fields like `command` vs `cmd` are remapped, and tools that don't exist in the target agent pass through unchanged.

## Development

```bash
make test    # Run all tests
make lint    # Run clippy and format check
make ci      # Run lint + test (used in CI)
```

## License

[MIT](LICENSE)
