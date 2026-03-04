use agentswap_core::types::{AgentKind, ConversationSummary};

/// Which screen the TUI is currently showing.
#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    AgentOverview,
    ConversationList,
    Transfer,
    TransferResult,
}

/// Summary information about a single agent, used for display in the TUI.
#[derive(Debug, Clone)]
pub struct AgentInfo {
    pub name: String,
    pub kind: AgentKind,
    pub available: bool,
    pub conversation_count: usize,
}

/// How to deliver a conversation to the target agent.
#[derive(Debug, Clone, PartialEq)]
pub enum TransferMethod {
    /// Write directly into the target agent's data directory.
    Native,
    /// Pipe the rendered prompt via stdin to the target agent's CLI.
    StdinPipe,
}

impl std::fmt::Display for TransferMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransferMethod::Native => write!(f, "Native (write to data dir)"),
            TransferMethod::StdinPipe => write!(f, "Stdin pipe (launch CLI)"),
        }
    }
}

/// The full application state for the TUI.
pub struct App {
    pub screen: Screen,
    pub agents: Vec<AgentInfo>,
    pub selected_agent_idx: usize,
    pub conversations: Vec<ConversationSummary>,
    pub selected_conv_idx: usize,
    pub target_agent_idx: usize,
    pub transfer_method: TransferMethod,
    pub status_message: Option<String>,
    pub should_quit: bool,
    pub search_query: String,
    pub searching: bool,
    /// The resume command to show after a successful transfer.
    pub resume_command: Option<String>,
    /// Whether the conversation preview panel is open.
    pub preview_open: bool,
    /// The rendered text of the previewed conversation.
    pub preview_text: String,
    /// Scroll offset for the preview panel.
    pub preview_scroll: u16,
    /// Whether 'g' was pressed, waiting for a second 'g' (vim gg).
    pub pending_g: bool,
}

impl App {
    /// Create a new App with the given list of agents.
    pub fn new(agents: Vec<AgentInfo>) -> Self {
        Self {
            screen: Screen::AgentOverview,
            agents,
            selected_agent_idx: 0,
            conversations: Vec::new(),
            selected_conv_idx: 0,
            target_agent_idx: 0,
            transfer_method: TransferMethod::Native,
            status_message: None,
            should_quit: false,
            search_query: String::new(),
            searching: false,
            resume_command: None,
            preview_open: false,
            preview_text: String::new(),
            preview_scroll: 0,
            pending_g: false,
        }
    }

    /// Move the selection cursor up in the current list.
    pub fn move_up(&mut self) {
        match self.screen {
            Screen::AgentOverview => {
                if self.selected_agent_idx > 0 {
                    self.selected_agent_idx -= 1;
                }
            }
            Screen::ConversationList => {
                let count = self.filtered_conversations().len();
                if count > 0 && self.selected_conv_idx > 0 {
                    self.selected_conv_idx -= 1;
                }
            }
            Screen::Transfer => {
                if self.target_agent_idx > 0 {
                    self.target_agent_idx -= 1;
                }
            }
            Screen::TransferResult => {}
        }
    }

    /// Move the selection cursor down in the current list.
    pub fn move_down(&mut self) {
        match self.screen {
            Screen::AgentOverview => {
                if !self.agents.is_empty() && self.selected_agent_idx < self.agents.len() - 1 {
                    self.selected_agent_idx += 1;
                }
            }
            Screen::ConversationList => {
                let count = self.filtered_conversations().len();
                if count > 0 && self.selected_conv_idx < count - 1 {
                    self.selected_conv_idx += 1;
                }
            }
            Screen::Transfer => {
                let targets = self.available_targets();
                if !targets.is_empty() && self.target_agent_idx < targets.len() - 1 {
                    self.target_agent_idx += 1;
                }
            }
            Screen::TransferResult => {}
        }
    }

    /// Move selection to the top of the current list.
    pub fn move_to_top(&mut self) {
        match self.screen {
            Screen::AgentOverview => self.selected_agent_idx = 0,
            Screen::ConversationList => self.selected_conv_idx = 0,
            Screen::Transfer => self.target_agent_idx = 0,
            Screen::TransferResult => {}
        }
    }

    /// Move selection to the bottom of the current list.
    pub fn move_to_bottom(&mut self) {
        match self.screen {
            Screen::AgentOverview => {
                if !self.agents.is_empty() {
                    self.selected_agent_idx = self.agents.len() - 1;
                }
            }
            Screen::ConversationList => {
                let count = self.filtered_conversations().len();
                if count > 0 {
                    self.selected_conv_idx = count - 1;
                }
            }
            Screen::Transfer => {
                let targets = self.available_targets();
                if !targets.is_empty() {
                    self.target_agent_idx = targets.len() - 1;
                }
            }
            Screen::TransferResult => {}
        }
    }

    /// Return conversations filtered by the current search query.
    pub fn filtered_conversations(&self) -> Vec<&ConversationSummary> {
        if self.search_query.is_empty() {
            return self.conversations.iter().collect();
        }
        let query = self.search_query.to_lowercase();
        self.conversations
            .iter()
            .filter(|c| {
                let title = c.summary.as_deref().unwrap_or(&c.id).to_lowercase();
                title.contains(&query)
                    || c.id.to_lowercase().contains(&query)
                    || c.project_dir.to_lowercase().contains(&query)
            })
            .collect()
    }

    /// Return agent indices that are available and not the currently selected source agent.
    pub fn available_targets(&self) -> Vec<usize> {
        self.agents
            .iter()
            .enumerate()
            .filter(|(i, a)| *i != self.selected_agent_idx && a.available)
            .map(|(i, _)| i)
            .collect()
    }

    /// Toggle between Native and StdinPipe transfer methods.
    pub fn toggle_transfer_method(&mut self) {
        self.transfer_method = match self.transfer_method {
            TransferMethod::Native => TransferMethod::StdinPipe,
            TransferMethod::StdinPipe => TransferMethod::Native,
        };
    }
}
