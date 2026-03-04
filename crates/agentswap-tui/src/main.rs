mod app;
mod ui;

use std::io;

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
use agentswap_gemini::GeminiAdapter;

use app::{AgentInfo, App, Screen};

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
                    KeyCode::Char('j') | KeyCode::Down => {
                        app.move_down();
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        app.move_up();
                    }
                    KeyCode::Enter => {
                        handle_enter(app, adapters);
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
                    KeyCode::Tab => {
                        if app.screen == Screen::Transfer {
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
fn handle_enter(app: &mut App, adapters: &[Box<dyn AgentAdapter>]) {
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
                        app.status_message =
                            Some(format!("Error loading conversations: {}", e));
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
            let target_name = &app.agents[target_agent_idx].name;

            let conv_title = app
                .filtered_conversations()
                .get(app.selected_conv_idx)
                .and_then(|c| c.summary.as_deref())
                .unwrap_or("conversation")
                .to_string();

            app.status_message = Some(format!(
                "Transfer of '{}' to {} via {} — not yet implemented",
                conv_title, target_name, app.transfer_method,
            ));
        }
    }
}

/// Handle the Esc key: go back one screen.
fn handle_esc(app: &mut App) {
    match app.screen {
        Screen::AgentOverview => {
            // Nothing to go back to; could quit, but spec says q/Ctrl+C for that.
        }
        Screen::ConversationList => {
            app.screen = Screen::AgentOverview;
            app.conversations.clear();
            app.search_query.clear();
            app.searching = false;
            app.status_message = None;
        }
        Screen::Transfer => {
            app.screen = Screen::ConversationList;
            app.status_message = None;
        }
    }
}
