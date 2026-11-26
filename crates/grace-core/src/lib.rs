//! Grace's engine: the event model, redaction, tool adapters, storage and scanning.
//!
//! Everything here works without any AI model (spec §4, principle 4).

pub mod adapters;
#[cfg(feature = "server")]
pub mod api;
pub mod config;
pub mod discover;
pub mod event;
pub mod fingerprint;
pub mod paths;
pub mod redact;
pub mod scan;
pub mod store;
pub mod timefmt;

pub use event::{Boundary, BoundaryKind, Event, EventKind, ToolId};
pub use redact::{Redactor, one_line, prefix_chars, truncate_chars};
