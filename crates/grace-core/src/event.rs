//! Normalized events and context changes (spec §7.4).
//!
//! Every tool's transcript is turned into the same small vocabulary, so the
//! same checks run for every tool (spec §4, principle 5).

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Declares a fieldless enum whose variants map to fixed snake_case strings,
/// with `as_str`, `parse`, `ALL` and serde support.
macro_rules! str_enum {
    ($(#[$meta:meta])* pub enum $name:ident { $($(#[$vmeta:meta])* $variant:ident => $text:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        pub enum $name {
            $($(#[$vmeta])* #[serde(rename = $text)] $variant),+
        }

        impl $name {
            /// Every variant, in declaration order.
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            /// The stable string form used in the database and the API.
            pub fn as_str(self) -> &'static str {
                match self { $($name::$variant => $text),+ }
            }

            /// Parses the string form produced by [`Self::as_str`].
            pub fn parse(text: &str) -> Option<Self> {
                match text { $($text => Some($name::$variant),)+ _ => None }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

str_enum! {
    /// The AI tools Grace knows about.
    pub enum ToolId {
        ClaudeCode => "claude_code",
        Codex => "codex",
        Antigravity => "antigravity",
        Ollama => "ollama",
        Cursor => "cursor",
    }
}

impl ToolId {
    /// The name people know the tool by.
    pub fn display_name(self) -> &'static str {
        match self {
            ToolId::ClaudeCode => "Claude Code",
            ToolId::Codex => "Codex",
            ToolId::Antigravity => "Antigravity",
            ToolId::Ollama => "Ollama",
            ToolId::Cursor => "Cursor",
        }
    }
}

str_enum! {
    /// What kind of thing happened in a session.
    pub enum EventKind {
        UserMessage => "user_message",
        AssistantMessage => "assistant_message",
        ToolCall => "tool_call",
        ToolResult => "tool_result",
        /// The exact system prompt, tools or request the model received.
        RequestSnapshot => "request_snapshot",
        InstructionLoad => "instruction_load",
        Compaction => "compaction",
        Resume => "resume",
        ModelSwitch => "model_switch",
        Handoff => "handoff",
        TaskUpdate => "task_update",
        HookRun => "hook_run",
        Unknown => "unknown",
    }
}

str_enum! {
    /// A moment the AI's context changes (UI label: "context change").
    pub enum BoundaryKind {
        SessionStart => "session_start",
        Compaction => "compaction",
        Resume => "resume",
        ModelSwitch => "model_switch",
        Handoff => "handoff",
    }
}

/// One normalized event. `line_no` and `turn` let any finding point at its
/// evidence (R11); the source file lives on the session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    /// Position within the session, starting at 0.
    pub seq: u64,
    pub kind: EventKind,
    /// The source entry type, e.g. `assistant/tool_use` or `attachment/instructions`.
    pub label: String,
    /// 1-based line in the source file.
    pub line_no: u64,
    /// 1-based turn (human prompt) number, when known.
    pub turn: Option<u32>,
    pub timestamp: Option<String>,
    pub uuid: Option<String>,
    pub parent_uuid: Option<String>,
    pub model: Option<String>,
    /// One redacted line, at most 160 characters.
    pub summary: String,
    /// Small, redacted, structured facts about the event.
    pub detail: Value,
}

/// A context change, pointing at the first event after it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Boundary {
    pub kind: BoundaryKind,
    pub event_seq: u64,
    pub line_no: u64,
    pub timestamp: Option<String>,
    pub detail: Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_forms_round_trip() {
        for kind in EventKind::ALL {
            assert_eq!(EventKind::parse(kind.as_str()), Some(*kind));
        }
        for tool in ToolId::ALL {
            assert_eq!(ToolId::parse(tool.as_str()), Some(*tool));
        }
        assert_eq!(
            BoundaryKind::parse("compaction"),
            Some(BoundaryKind::Compaction)
        );
        assert_eq!(EventKind::parse("nope"), None);
    }

    #[test]
    fn serde_uses_snake_case() {
        let json = serde_json::to_string(&EventKind::RequestSnapshot).unwrap();
        assert_eq!(json, "\"request_snapshot\"");
        assert_eq!(ToolId::ClaudeCode.display_name(), "Claude Code");
    }
}
