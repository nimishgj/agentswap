use chrono::{DateTime, Utc};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Wrap,
    },
    Frame,
};

use crate::app::{App, Screen};

/// Main draw function: dispatches to the appropriate screen renderer.
pub fn draw(f: &mut Frame, app: &App) {
    match app.screen {
        Screen::AgentOverview => draw_agent_overview(f, app),
        Screen::ConversationList => draw_conversation_list(f, app),
        Screen::Transfer => draw_transfer(f, app),
        Screen::TransferResult => draw_transfer_result(f, app),
    }
}

// ---------------------------------------------------------------------------
// Screen 1: Agent Overview
// ---------------------------------------------------------------------------

fn draw_agent_overview(f: &mut Frame, app: &App) {
    let area = f.area();

    // Split into main content and footer (keybindings + optional status).
    let footer_lines = if app.status_message.is_some() { 2 } else { 1 };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(footer_lines as u16)])
        .split(area);

    // Build list items for each agent.
    let items: Vec<ListItem> = app
        .agents
        .iter()
        .map(|agent| {
            let dot = if agent.available {
                Span::styled("\u{25cf} ", Style::default().fg(Color::Green))
            } else {
                Span::styled("\u{25cb} ", Style::default().fg(Color::DarkGray))
            };

            let name = Span::styled(
                format!("{:<18}", agent.name),
                if agent.available {
                    Style::default().fg(Color::White)
                } else {
                    Style::default().fg(Color::DarkGray)
                },
            );

            let info = if agent.available {
                Span::styled(
                    format!("{} conversations", agent.conversation_count),
                    Style::default().fg(Color::Cyan),
                )
            } else {
                Span::styled(
                    "(not installed)".to_string(),
                    Style::default().fg(Color::DarkGray),
                )
            };

            ListItem::new(Line::from(vec![dot, name, info]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" AgentSwap ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::DarkGray),
        )
        .highlight_symbol("  ");

    let mut state = ListState::default();
    state.select(Some(app.selected_agent_idx));
    f.render_stateful_widget(list, chunks[0], &mut state);

    draw_footer(f, chunks[1], app, "[Enter] Browse  [q] Quit");
}

// ---------------------------------------------------------------------------
// Screen 2: Conversation List
// ---------------------------------------------------------------------------

fn draw_conversation_list(f: &mut Frame, app: &App) {
    let area = f.area();

    let agent_name = app
        .agents
        .get(app.selected_agent_idx)
        .map(|a| a.name.as_str())
        .unwrap_or("Unknown");

    // Determine layout pieces: optional search bar, list, footer.
    let footer_lines = if app.status_message.is_some() { 2 } else { 1 };
    let search_height: u16 = if app.searching { 3 } else { 0 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(search_height),
            Constraint::Min(1),
            Constraint::Length(footer_lines as u16),
        ])
        .split(area);

    // Search bar
    if app.searching {
        let search_text = format!("Search: {}_", app.search_query);
        let search_bar = Paragraph::new(search_text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        );
        f.render_widget(search_bar, chunks[0]);
    }

    // Split main area horizontally if preview is open.
    let (list_area, preview_area) = if app.preview_open {
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(chunks[1]);
        (panes[0], Some(panes[1]))
    } else {
        (chunks[1], None)
    };

    // Build conversation list items.
    let filtered = app.filtered_conversations();
    let items: Vec<ListItem> = filtered
        .iter()
        .map(|conv| {
            let title = conv
                .summary
                .as_deref()
                .unwrap_or_else(|| {
                    if conv.id.len() > 50 {
                        &conv.id[..50]
                    } else {
                        &conv.id
                    }
                })
                .to_string();

            let time_str = format_relative_time(conv.updated_at);
            let meta = format!("{} \u{00b7} {} messages", time_str, conv.message_count);

            ListItem::new(vec![
                Line::from(Span::styled(
                    format!("  {}", title),
                    Style::default().fg(Color::White),
                )),
                Line::from(Span::styled(
                    format!("  {}", meta),
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
            ])
        })
        .collect();

    let block_title = format!(
        " {} \u{2014} Conversations ({}) ",
        agent_name,
        filtered.len()
    );

    let list = List::new(items)
        .block(
            Block::default()
                .title(block_title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::DarkGray),
        )
        .highlight_symbol("> ");

    let mut state = ListState::default();
    if !filtered.is_empty() {
        state.select(Some(
            app.selected_conv_idx.min(filtered.len().saturating_sub(1)),
        ));
    }
    f.render_stateful_widget(list, list_area, &mut state);

    // Preview panel
    if let Some(preview_rect) = preview_area {
        let preview_lines: Vec<Line> = app
            .preview_text
            .lines()
            .map(|line| {
                let style = if line.starts_with("## User") || line.starts_with("## Assistant") {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else if line.starts_with("**") {
                    Style::default().fg(Color::Cyan)
                } else if line.starts_with("---") {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default().fg(Color::White)
                };
                Line::from(Span::styled(line.to_string(), style))
            })
            .collect();

        let total_lines = preview_lines.len();
        let preview = Paragraph::new(preview_lines)
            .block(
                Block::default()
                    .title(" Preview ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .wrap(Wrap { trim: false })
            .scroll((app.preview_scroll, 0));
        f.render_widget(preview, preview_rect);

        // Scrollbar
        let mut scrollbar_state =
            ScrollbarState::new(total_lines).position(app.preview_scroll as usize);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None);
        f.render_stateful_widget(scrollbar, preview_rect, &mut scrollbar_state);
    }

    let footer_hint = if app.preview_open {
        "[j/k] Scroll  [Tab] Close preview  [Esc] Back"
    } else {
        "[Enter] Select  [/] Search  [Tab] Preview  [Esc] Back"
    };
    draw_footer(f, chunks[2], app, footer_hint);
}

// ---------------------------------------------------------------------------
// Screen 3: Transfer
// ---------------------------------------------------------------------------

fn draw_transfer(f: &mut Frame, app: &App) {
    let area = f.area();

    let footer_lines: u16 = if app.status_message.is_some() { 2 } else { 1 };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(footer_lines)])
        .split(area);

    let inner_block = Block::default()
        .title(" Transfer ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));
    let inner_area = inner_block.inner(chunks[0]);
    f.render_widget(inner_block, chunks[0]);

    // Now render content inside the block.
    let content_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // From + Conv
            Constraint::Length(1), // blank
            Constraint::Min(1),    // Target list + Method
        ])
        .split(inner_area);

    // From / Conv lines
    let source_name = app
        .agents
        .get(app.selected_agent_idx)
        .map(|a| a.name.as_str())
        .unwrap_or("Unknown");

    let conv_label = if let Some(conv) = app.filtered_conversations().get(app.selected_conv_idx) {
        let title = conv.summary.as_deref().unwrap_or_else(|| {
            if conv.id.len() > 30 {
                &conv.id[..30]
            } else {
                &conv.id
            }
        });
        format!("{} ({} msgs)", title, conv.message_count)
    } else {
        "None selected".to_string()
    };

    let header_lines = vec![
        Line::from(vec![
            Span::styled("  From: ", Style::default().fg(Color::Yellow)),
            Span::styled(source_name, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Conv: ", Style::default().fg(Color::Yellow)),
            Span::styled(conv_label, Style::default().fg(Color::White)),
        ]),
    ];
    let header = Paragraph::new(header_lines);
    f.render_widget(header, content_chunks[0]);

    // Target list and method in the remaining space.
    let targets = app.available_targets();

    let lower_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),                           // "To:" label
            Constraint::Length(targets.len().max(1) as u16), // target list
            Constraint::Min(0),                              // filler
        ])
        .split(content_chunks[2]);

    // "To:" label
    let to_label = Paragraph::new(Line::from(Span::styled(
        "  To:",
        Style::default().fg(Color::Yellow),
    )));
    f.render_widget(to_label, lower_chunks[0]);

    // Target agent list
    if targets.is_empty() {
        let no_targets = Paragraph::new(Line::from(Span::styled(
            "    (no available targets)",
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(no_targets, lower_chunks[1]);
    } else {
        let target_items: Vec<ListItem> = targets
            .iter()
            .map(|&idx| {
                let name = app.agents[idx].name.clone();
                ListItem::new(Line::from(Span::styled(
                    format!("    {}", name),
                    Style::default().fg(Color::White),
                )))
            })
            .collect();

        let target_list = List::new(target_items)
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::DarkGray),
            )
            .highlight_symbol("  > ");

        let mut target_state = ListState::default();
        if !targets.is_empty() {
            target_state.select(Some(
                app.target_agent_idx.min(targets.len().saturating_sub(1)),
            ));
        }
        f.render_stateful_widget(target_list, lower_chunks[1], &mut target_state);
    }

    draw_footer(f, chunks[1], app, "[Enter] Transfer  [Esc] Back");
}

// ---------------------------------------------------------------------------
// Screen 4: Transfer Result
// ---------------------------------------------------------------------------

fn draw_transfer_result(f: &mut Frame, app: &App) {
    let area = f.area();

    let footer_lines: u16 = if app.status_message.is_some() { 2 } else { 1 };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(footer_lines)])
        .split(area);

    let cmd = app
        .resume_command
        .as_deref()
        .unwrap_or("(no command available)");

    let content = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Transfer complete!",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Run this command to resume the conversation:",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("    {}", cmd),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    let paragraph = Paragraph::new(content).block(
        Block::default()
            .title(" Transfer Result ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green)),
    );
    f.render_widget(paragraph, chunks[0]);

    draw_footer(f, chunks[1], app, "[c] Copy command  [Enter/Esc] Back");
}

// ---------------------------------------------------------------------------
// Footer with optional status message
// ---------------------------------------------------------------------------

fn draw_footer(f: &mut Frame, area: Rect, app: &App, keybindings: &str) {
    if let Some(ref msg) = app.status_message {
        // Split footer area into status line + keybindings line.
        let parts = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(area);

        let status = Paragraph::new(Line::from(Span::styled(
            format!(" {}", msg),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
        f.render_widget(status, parts[0]);

        let keys = Paragraph::new(Line::from(Span::styled(
            format!(" {}", keybindings),
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(keys, parts[1]);
    } else {
        let keys = Paragraph::new(Line::from(Span::styled(
            format!(" {}", keybindings),
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(keys, area);
    }
}

// ---------------------------------------------------------------------------
// Helper: format a DateTime<Utc> as relative time (e.g. "2h ago")
// ---------------------------------------------------------------------------

pub fn format_relative_time(dt: DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(dt);

    if duration.num_minutes() < 1 {
        "just now".to_string()
    } else if duration.num_hours() < 1 {
        format!("{}m ago", duration.num_minutes())
    } else if duration.num_days() < 1 {
        format!("{}h ago", duration.num_hours())
    } else if duration.num_weeks() < 1 {
        format!("{}d ago", duration.num_days())
    } else {
        format!("{}w ago", duration.num_weeks())
    }
}
