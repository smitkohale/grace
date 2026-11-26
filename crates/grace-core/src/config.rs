//! Grace's settings in `~/.grace/config.toml`. People change them in the app;
//! anyone editing the file by hand keeps their comments and layout (toml_edit).

use crate::ToolId;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use toml_edit::{DocumentMut, value};

const HEADER: &str = "# Grace settings. You can change these in the Grace app.\n";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub onboarding_done: bool,
    pub redact_emails: bool,
    pub tools: BTreeMap<String, ToolConfig>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolConfig {
    pub enabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            onboarding_done: false,
            redact_emails: true,
            tools: BTreeMap::new(),
        }
    }
}

impl Config {
    pub fn path(grace_home: &Path) -> PathBuf {
        grace_home.join("config.toml")
    }

    /// Loads settings; a missing file means defaults.
    pub fn load(grace_home: &Path) -> Result<Config> {
        let path = Self::path(grace_home);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(Config::default()),
            Err(err) => {
                return Err(err).with_context(|| format!("couldn't read {}", path.display()));
            }
        };
        let doc: DocumentMut = text
            .parse()
            .with_context(|| format!("{} isn't valid TOML", path.display()))?;
        let mut config = Config::default();
        if let Some(done) = doc.get("onboarding_done").and_then(|item| item.as_bool()) {
            config.onboarding_done = done;
        }
        if let Some(redact) = doc.get("redact_emails").and_then(|item| item.as_bool()) {
            config.redact_emails = redact;
        }
        if let Some(tools) = doc.get("tools").and_then(|item| item.as_table_like()) {
            for (name, item) in tools.iter() {
                let enabled = item
                    .as_table_like()
                    .and_then(|table| table.get("enabled"))
                    .and_then(|enabled| enabled.as_bool());
                if let Some(enabled) = enabled {
                    config
                        .tools
                        .insert(name.to_string(), ToolConfig { enabled });
                }
            }
        }
        Ok(config)
    }

    /// Saves settings, keeping everything else in the file as it was.
    pub fn save(&self, grace_home: &Path) -> Result<()> {
        std::fs::create_dir_all(grace_home)
            .with_context(|| format!("couldn't create {}", grace_home.display()))?;
        let path = Self::path(grace_home);
        let existing = std::fs::read_to_string(&path).ok();
        let mut doc: DocumentMut = match &existing {
            Some(text) => text
                .parse()
                .with_context(|| format!("{} isn't valid TOML", path.display()))?,
            None => DocumentMut::new(),
        };
        set_bool(&mut doc, "onboarding_done", self.onboarding_done);
        set_bool(&mut doc, "redact_emails", self.redact_emails);
        let tools = doc
            .entry("tools")
            .or_insert(toml_edit::table())
            .as_table_mut()
            .context("`tools` in config.toml must be a table")?;
        tools.set_implicit(true);
        for (name, tool) in &self.tools {
            let table = tools
                .entry(name)
                .or_insert(toml_edit::table())
                .as_table_mut()
                .with_context(|| format!("`tools.{name}` in config.toml must be a table"))?;
            set_bool(table, "enabled", tool.enabled);
        }
        let mut text = doc.to_string();
        if existing.is_none() {
            text.insert_str(0, HEADER);
        }
        let temp = path.with_extension("toml.tmp");
        std::fs::write(&temp, text)
            .with_context(|| format!("couldn't write {}", temp.display()))?;
        std::fs::rename(&temp, &path)
            .with_context(|| format!("couldn't save {}", path.display()))?;
        Ok(())
    }
}

/// Sets a boolean, keeping the comments and spacing around an existing value.
fn set_bool(table: &mut toml_edit::Table, key: &str, flag: bool) {
    match table.get_mut(key).and_then(|item| item.as_value_mut()) {
        Some(current) => {
            let decor = current.decor().clone();
            *current = toml_edit::Value::from(flag);
            *current.decor_mut() = decor;
        }
        None => table[key] = value(flag),
    }
}

impl Config {
    /// Claude Code is on unless turned off; other tools are off until supported.
    pub fn tool_enabled(&self, tool: ToolId) -> bool {
        self.tools
            .get(tool.as_str())
            .map_or(tool == ToolId::ClaudeCode, |config| config.enabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::load(dir.path()).unwrap();
        assert_eq!(config, Config::default());
        assert!(config.tool_enabled(ToolId::ClaudeCode));
        assert!(!config.tool_enabled(ToolId::Codex));
    }

    #[test]
    fn config_roundtrip_keeps_comments() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            Config::path(dir.path()),
            "# my notes stay\nredact_emails = false # keep emails\n\n[tools.claude_code]\nenabled = true\n",
        )
        .unwrap();
        let mut config = Config::load(dir.path()).unwrap();
        assert!(!config.redact_emails);
        config.onboarding_done = true;
        config
            .tools
            .insert("claude_code".into(), ToolConfig { enabled: false });
        config.save(dir.path()).unwrap();

        let text = std::fs::read_to_string(Config::path(dir.path())).unwrap();
        assert!(text.contains("# my notes stay"), "{text}");
        assert!(text.contains("# keep emails"), "{text}");
        let reloaded = Config::load(dir.path()).unwrap();
        assert!(reloaded.onboarding_done);
        assert!(!reloaded.tool_enabled(ToolId::ClaudeCode));
    }

    #[test]
    fn saves_into_a_new_folder() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("new home");
        Config::default().save(&home).unwrap();
        let text = std::fs::read_to_string(Config::path(&home)).unwrap();
        assert!(text.starts_with("# Grace settings"));
    }
}
