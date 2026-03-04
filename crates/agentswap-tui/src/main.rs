mod app;
mod ui;

use std::io::{self, Write};
use std::process::{Command, Stdio};

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use agentswap_claude::ClaudeAdapter;
use agentswap_codex::CodexAdapter;
use agentswap_core::adapter::AgentAdapter;
use agentswap_core::types::AgentKind;
use agentswap_gemini::GeminiAdapter;

use app::{AgentInfo, App, Screen, TransferMethod};

fn main() -> Result<()> {
    // ---- Build adapters and gather agent info ----
    let adapters: Vec<Box<dyn AgentAdapter>> = vec![
        Box::new(ClaudeAdapter::new()),
        Box::new(CodexAdapter::new()),
        Box::new(GeminiAdapter::new()),
    ];

    let agents: Vec<AgentInfo> = adapters
        .iter()
        .map(|adapter| {
            let available = adapter.is_available();
            let conversation_count = if available {
                adapter.list_conversations().map(|c| c.len()).unwrap_or(0)
            } else {
                0
            };
            AgentInfo {
                name: adapter.display_name().to_string(),
                kind: adapter.agent_kind(),
                available,
                conversation_count,
            }
        })
        .collect();

    let mut app = App::new(agents);

    // ---- Setup terminal ----
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // ---- Main loop ----
    let result = run_event_loop(&mut terminal, &mut app, &adapters);

    // ---- Restore terminal ----
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    adapters: &[Box<dyn AgentAdapter>],
) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if let Event::Key(key) = event::read()? {
            // Global: Ctrl+C always quits.
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                app.should_quit = true;
            }

            // Clear pending_g on any key that isn't 'g'
            if app.pending_g && key.code != KeyCode::Char('g') && key.code != KeyCode::Char('G') {
                app.pending_g = false;
            }

            // If in search mode, handle search-specific keys first.
            if app.searching {
                match key.code {
                    KeyCode::Esc => {
                        app.searching = false;
                        app.search_query.clear();
                        app.selected_conv_idx = 0;
                    }
                    KeyCode::Enter => {
                        app.searching = false;
                        // Keep the filter applied; selection stays.
                        app.selected_conv_idx = 0;
                    }
                    KeyCode::Backspace => {
                        app.search_query.pop();
                        app.selected_conv_idx = 0;
                    }
                    KeyCode::Char(c) => {
                        app.search_query.push(c);
                        app.selected_conv_idx = 0;
                    }
                    _ => {}
                }
            } else {
                // Normal (non-search) key handling.
                match key.code {
                    KeyCode::Char('q') => {
                        app.should_quit = true;
                    }
                    KeyCode::Char('c') if app.screen == Screen::TransferResult => {
                        if let Some(cmd) = &app.resume_command {
                            copy_to_clipboard(cmd);
                            app.status_message = Some("Copied to clipboard!".to_string());
                        }
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        if app.preview_open && app.screen == Screen::ConversationList {
                            app.preview_scroll = app.preview_scroll.saturating_add(1);
                        } else {
                            app.move_down();
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if app.preview_open && app.screen == Screen::ConversationList {
                            app.preview_scroll = app.preview_scroll.saturating_sub(1);
                        } else {
                            app.move_up();
                        }
                    }
                    KeyCode::Enter => {
                        handle_enter(app, adapters, terminal);
                    }
                    KeyCode::Esc => {
                        handle_esc(app);
                    }
                    KeyCode::Char('/') => {
                        if app.screen == Screen::ConversationList {
                            app.searching = true;
                            app.search_query.clear();
                        }
                    }
                    KeyCode::Char('G') => {
                        app.pending_g = false;
                        if app.preview_open && app.screen == Screen::ConversationList {
                            let total = app.preview_text.lines().count() as u16;
                            app.preview_scroll = total.saturating_sub(1);
                        } else {
                            app.move_to_bottom();
                        }
                    }
                    KeyCode::Char('g') => {
                        if app.pending_g {
                            // gg = go to top
                            app.pending_g = false;
                            if app.preview_open && app.screen == Screen::ConversationList {
                                app.preview_scroll = 0;
                            } else {
                                app.move_to_top();
                            }
                        } else {
                            app.pending_g = true;
                        }
                    }
                    KeyCode::Tab => {
                        if app.screen == Screen::ConversationList {
                            if app.preview_open {
                                app.preview_open = false;
                                app.preview_text.clear();
                            } else {
                                app.preview_open = true;
                                load_preview(app, adapters);
                            }
                        } else if app.screen == Screen::Transfer {
                            app.toggle_transfer_method();
                        }
                    }
                    _ => {}
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

/// Handle the Enter key depending on the current screen.
fn handle_enter(
    app: &mut App,
    adapters: &[Box<dyn AgentAdapter>],
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) {
    match app.screen {
        Screen::AgentOverview => {
            let idx = app.selected_agent_idx;
            if let Some(agent) = app.agents.get(idx) {
                if !agent.available {
                    app.status_message = Some(format!("{} is not installed", agent.name));
                    return;
                }
                // Load conversations from the selected adapter.
                match adapters[idx].list_conversations() {
                    Ok(convos) => {
                        app.conversations = convos;
                        app.selected_conv_idx = 0;
                        app.search_query.clear();
                        app.searching = false;
                        app.status_message = None;
                        app.screen = Screen::ConversationList;
                    }
                    Err(e) => {
                        app.status_message = Some(format!("Error loading conversations: {}", e));
                    }
                }
            }
        }
        Screen::ConversationList => {
            let filtered = app.filtered_conversations();
            if filtered.is_empty() {
                app.status_message = Some("No conversations to select".to_string());
                return;
            }
            // Advance to the Transfer screen.
            app.target_agent_idx = 0;
            app.transfer_method = app::TransferMethod::Native;
            app.status_message = None;
            app.screen = Screen::Transfer;
        }
        Screen::Transfer => {
            let targets = app.available_targets();
            if targets.is_empty() {
                app.status_message = Some("No available target agents".to_string());
                return;
            }
            let target_idx = app.target_agent_idx.min(targets.len().saturating_sub(1));
            let target_agent_idx = targets[target_idx];
            let target_name = app.agents[target_agent_idx].name.clone();
            let target_kind = app.agents[target_agent_idx].kind.clone();
            let source_idx = app.selected_agent_idx;

            // Get the selected conversation ID.
            let conv_id = match app.filtered_conversations().get(app.selected_conv_idx) {
                Some(c) => c.id.clone(),
                None => {
                    app.status_message = Some("No conversation selected".to_string());
                    return;
                }
            };

            // Read the full conversation from the source adapter.
            let mut conversation = match adapters[source_idx].read_conversation(&conv_id) {
                Ok(conv) => conv,
                Err(e) => {
                    app.status_message = Some(format!("Error reading conversation: {}", e));
                    return;
                }
            };

            // Use the current working directory so the target agent can find
            // the session when the user runs the resume command from here.
            if let Ok(cwd) = std::env::current_dir() {
                conversation.project_dir = cwd.to_string_lossy().to_string();
            }

            match app.transfer_method {
                TransferMethod::StdinPipe => {
                    // Render the conversation as a prompt using the source adapter.
                    let prompt = match adapters[source_idx].render_prompt(&conversation) {
                        Ok(p) => p,
                        Err(e) => {
                            app.status_message = Some(format!("Error rendering prompt: {}", e));
                            return;
                        }
                    };

                    match execute_stdin_transfer(&prompt, &target_kind, terminal) {
                        Ok(()) => {
                            app.status_message =
                                Some(format!("Transfer complete — launched {}", target_name));
                        }
                        Err(e) => {
                            app.status_message = Some(format!("Transfer failed: {}", e));
                        }
                    }
                }
                TransferMethod::Native => {
                    // Try native write via the target adapter.
                    match adapters[target_agent_idx].write_conversation(&conversation) {
                        Ok(new_id) => {
                            let resume_cmd = build_resume_command(&target_kind, &new_id);
                            app.resume_command = Some(resume_cmd);
                            app.status_message = None;
                            app.screen = Screen::TransferResult;
                        }
                        Err(e) => {
                            app.status_message = Some(format!("Native transfer failed: {}", e));
                        }
                    }
                }
            }
        }
        Screen::TransferResult => {
            // Enter goes back to agent overview
            app.screen = Screen::AgentOverview;
            app.resume_command = None;
            app.status_message = None;
        }
    }
}

/// Temporarily leave the TUI, spawn the target agent's CLI with the prompt
/// piped to stdin, wait for it to finish, then re-enter the TUI.
fn execute_stdin_transfer(
    prompt: &str,
    target_kind: &AgentKind,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    // Restore terminal so the child process can use it.
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    let cmd_args: Vec<&str> = match target_kind {
        AgentKind::Claude => vec!["claude", "-p", "--output-format", "text"],
        AgentKind::Codex => vec!["codex", "exec"],
        AgentKind::Gemini => vec!["gemini", "-p"],
    };

    let result = (|| -> Result<()> {
        let mut child = Command::new(cmd_args[0])
            .args(&cmd_args[1..])
            .stdin(Stdio::piped())
            .spawn()?;

        if let Some(mut stdin_handle) = child.stdin.take() {
            stdin_handle.write_all(prompt.as_bytes())?;
            // Drop stdin to signal EOF to the child process.
        }

        child.wait()?;
        Ok(())
    })();

    // Always re-enter TUI mode, even if the child process failed.
    enable_raw_mode()?;
    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    terminal.hide_cursor()?;
    terminal.clear()?;

    result
}

/// Handle the Esc key: go back one screen.
fn handle_esc(app: &mut App) {
    match app.screen {
        Screen::AgentOverview => {
            // Nothing to go back to; could quit, but spec says q/Ctrl+C for that.
        }
        Screen::ConversationList => {
            if app.preview_open {
                app.preview_open = false;
                app.preview_text.clear();
            } else {
                app.screen = Screen::AgentOverview;
                app.conversations.clear();
                app.search_query.clear();
                app.searching = false;
                app.status_message = None;
            }
        }
        Screen::Transfer => {
            app.screen = Screen::ConversationList;
            app.status_message = None;
        }
        Screen::TransferResult => {
            app.screen = Screen::AgentOverview;
            app.resume_command = None;
            app.status_message = None;
        }
    }
}

/// Load the preview text for the currently selected conversation.
fn load_preview(app: &mut App, adapters: &[Box<dyn AgentAdapter>]) {
    app.preview_scroll = 0;
    let filtered = app.filtered_conversations();
    let conv_id = match filtered.get(app.selected_conv_idx) {
        Some(c) => c.id.clone(),
        None => {
            app.preview_text = "(no conversation selected)".to_string();
            return;
        }
    };
    let source_idx = app.selected_agent_idx;
    match adapters[source_idx].read_conversation(&conv_id) {
        Ok(conv) => match adapters[source_idx].render_prompt(&conv) {
            Ok(text) => app.preview_text = text,
            Err(e) => app.preview_text = format!("Error rendering: {}", e),
        },
        Err(e) => {
            app.preview_text = format!("Error loading: {}", e);
        }
    }
}

/// Build the CLI resume command for a given agent and session ID.
fn build_resume_command(target_kind: &AgentKind, session_id: &str) -> String {
    match target_kind {
        AgentKind::Claude => format!("claude --resume {}", session_id),
        AgentKind::Codex => format!("codex resume {}", session_id),
        AgentKind::Gemini => format!("gemini --resume {}", session_id),
    }
}

/// Copy text to the system clipboard using platform-native commands.
fn copy_to_clipboard(text: &str) {
    // Try pbcopy (macOS), then xclip (Linux), then xsel (Linux)
    let result = Command::new("pbcopy")
        .stdin(Stdio::piped())
        .spawn()
        .or_else(|_| {
            Command::new("xclip")
                .args(["-selection", "clipboard"])
                .stdin(Stdio::piped())
                .spawn()
        })
        .or_else(|_| {
            Command::new("xsel")
                .arg("--clipboard")
                .stdin(Stdio::piped())
                .spawn()
        });

    if let Ok(mut child) = result {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = child.wait();
    }
}
