//! Where Grace and the tools it reads keep their files.
//!
//! Every location can be overridden with an environment variable, which keeps
//! tests away from the real home folder.

use std::path::{Path, PathBuf};

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// The home folder used for discovery: `$GRACE_DISCOVERY_HOME` (tests) or the OS home.
pub fn home_dir() -> Option<PathBuf> {
    env_path("GRACE_DISCOVERY_HOME").or_else(dirs::home_dir)
}

/// Grace's own data folder: `$GRACE_HOME` or `~/.grace`.
pub fn grace_home() -> PathBuf {
    env_path("GRACE_HOME")
        .or_else(|| home_dir().map(|home| home.join(".grace")))
        .unwrap_or_else(|| PathBuf::from(".grace"))
}

/// Claude Code's folder: `$CLAUDE_CONFIG_DIR` or `~/.claude`.
pub fn claude_home() -> Option<PathBuf> {
    env_path("CLAUDE_CONFIG_DIR").or_else(|| home_dir().map(|home| home.join(".claude")))
}

/// Codex's folder: `$CODEX_HOME` or `~/.codex`.
pub fn codex_home() -> Option<PathBuf> {
    env_path("CODEX_HOME").or_else(|| home_dir().map(|home| home.join(".codex")))
}

/// Gemini CLI and Antigravity's folder: `~/.gemini`.
pub fn gemini_home() -> Option<PathBuf> {
    home_dir().map(|home| home.join(".gemini"))
}

const CREDENTIAL_FILES: &[&str] = &[
    "auth.json",
    ".credentials.json",
    "credentials.json",
    "oauth_creds.json",
    "google_accounts.json",
];

/// True for files Grace must never open (R8): tool logins and `.env` files.
pub fn is_credential_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let name = name.to_ascii_lowercase();
    CREDENTIAL_FILES.contains(&name.as_str()) || name == ".env" || name.starts_with(".env.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spots_credential_files() {
        for name in [
            "auth.json",
            ".credentials.json",
            "oauth_creds.json",
            ".env",
            ".env.local",
            "AUTH.JSON",
        ] {
            assert!(is_credential_file(Path::new(name)), "{name}");
        }
        for name in [
            "session.jsonl",
            "config.toml",
            "environment.md",
            "author.json",
        ] {
            assert!(!is_credential_file(Path::new(name)), "{name}");
        }
    }
}
