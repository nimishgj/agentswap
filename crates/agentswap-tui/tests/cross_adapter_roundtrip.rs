//! Cross-adapter round-trip tests: write to one adapter, read from another.

use std::collections::HashMap;

use agentswap_claude::ClaudeAdapter;
use agentswap_core::adapter::AgentAdapter;
use agentswap_core::types::*;
use agentswap_gemini::GeminiAdapter;
use chrono::Utc;
use serde_json::json;
use tempfile::TempDir;
use uuid::Uuid;

/// Build a sample UCF conversation for testing cross-adapter transfers.
fn sample_conversation() -> Conversation {
    let now = Utc::now();
    Conversation {
        id: "cross-test".to_string(),
        source_agent: AgentKind::Claude,
        project_dir: "/Users/test/project".to_string(),
        created_at: now,
        updated_at: now,
        summary: Some("Cross-adapter test".to_string()),
        messages: vec![
            Message {
                id: Uuid::new_v4(),
                timestamp: now,
                role: Role::User,
                content: "Hello from Claude!".to_string(),
                tool_calls: Vec::new(),
                metadata: HashMap::new(),
            },
            Message {
                id: Uuid::new_v4(),
                timestamp: now,
                role: Role::Assistant,
                content: "I can help with that.".to_string(),
                tool_calls: vec![ToolCall {
                    name: "readFile".to_string(),
                    input: json!({"path": "/tmp/test.rs"}),
                    output: Some("fn main() {}".to_string()),
                    status: ToolStatus::Success,
                }],
                metadata: HashMap::new(),
            },
            Message {
                id: Uuid::new_v4(),
                timestamp: now,
                role: Role::User,
                content: "Thanks!".to_string(),
                tool_calls: Vec::new(),
                metadata: HashMap::new(),
            },
        ],
        file_changes: Vec::new(),
    }
}

#[test]
fn test_claude_to_gemini_roundtrip() {
    let claude_tmp = TempDir::new().unwrap();
    let gemini_tmp = TempDir::new().unwrap();

    let _claude = ClaudeAdapter::with_projects_dir(claude_tmp.path().to_path_buf());
    let gemini = GeminiAdapter::with_tmp_dir(gemini_tmp.path().to_path_buf());

    let conv = sample_conversation();

    // Write to Gemini's native format
    let gemini_id = gemini.write_conversation(&conv).unwrap();

    // Read back from Gemini
    let read_conv = gemini.read_conversation(&gemini_id).unwrap();

    assert_eq!(read_conv.source_agent, AgentKind::Gemini);
    assert_eq!(read_conv.summary.as_deref(), Some("Cross-adapter test"));

    // Verify user messages preserved
    let user_msgs: Vec<&Message> = read_conv
        .messages
        .iter()
        .filter(|m| m.role == Role::User)
        .collect();
    assert_eq!(user_msgs.len(), 2);
    assert_eq!(user_msgs[0].content, "Hello from Claude!");
    assert_eq!(user_msgs[1].content, "Thanks!");

    // Verify assistant message preserved
    let assistant_msgs: Vec<&Message> = read_conv
        .messages
        .iter()
        .filter(|m| m.role == Role::Assistant)
        .collect();
    assert!(!assistant_msgs.is_empty());
    assert_eq!(assistant_msgs[0].content, "I can help with that.");
    assert_eq!(assistant_msgs[0].tool_calls.len(), 1);
    assert_eq!(assistant_msgs[0].tool_calls[0].name, "readFile");
}

#[test]
fn test_gemini_to_claude_roundtrip() {
    let claude_tmp = TempDir::new().unwrap();
    let gemini_tmp = TempDir::new().unwrap();

    let claude = ClaudeAdapter::with_projects_dir(claude_tmp.path().to_path_buf());
    let _gemini = GeminiAdapter::with_tmp_dir(gemini_tmp.path().to_path_buf());

    let mut conv = sample_conversation();
    conv.source_agent = AgentKind::Gemini;

    // Write to Claude's native format
    let claude_id = claude.write_conversation(&conv).unwrap();

    // Read back from Claude
    let read_conv = claude.read_conversation(&claude_id).unwrap();

    assert_eq!(read_conv.source_agent, AgentKind::Claude);
    assert_eq!(read_conv.summary.as_deref(), Some("Cross-adapter test"));

    // Verify user messages preserved
    let user_msgs: Vec<&Message> = read_conv
        .messages
        .iter()
        .filter(|m| m.role == Role::User && !m.content.is_empty())
        .collect();
    assert!(user_msgs.len() >= 2);
    assert_eq!(user_msgs[0].content, "Hello from Claude!");

    // Verify assistant message preserved
    let assistant_msgs: Vec<&Message> = read_conv
        .messages
        .iter()
        .filter(|m| m.role == Role::Assistant)
        .collect();
    assert!(!assistant_msgs.is_empty());
    assert_eq!(assistant_msgs[0].content, "I can help with that.");
}
