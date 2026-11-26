//! Plain-text tables for the terminal. Words follow DESIGN.md §4.

use grace_core::discover::DoctorReport;
use grace_core::store::SessionRow;
use grace_core::{timefmt, truncate_chars};

fn count(n: u32, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

pub fn doctor(report: &DoctorReport) -> String {
    let mut out = String::from("Grace doctor (read-only: nothing was changed)\n\n");
    for tool in &report.tools {
        if !tool.found {
            out += &format!("  – {:<12} not found\n", tool.name);
            continue;
        }
        let sessions = tool
            .sessions
            .map(|n| count(u32::try_from(n).unwrap_or(u32::MAX), "session"))
            .unwrap_or_default();
        out += &format!(
            "  ✓ {:<12} {:<18} {:<13} {}\n",
            tool.name,
            tool.version.as_deref().unwrap_or(""),
            sessions,
            tool.data_dir.as_deref().unwrap_or("")
        );
        for note in &tool.notes {
            out += &format!("      {note}\n");
        }
    }
    out += &format!(
        "\nGrace's data: {}{}\n",
        report.grace_home,
        if report.grace_home_exists {
            ""
        } else {
            " (created on first use)"
        }
    );
    out
}

pub fn sessions(rows: &[SessionRow], now_ms: i64) -> String {
    if rows.is_empty() {
        return "No sessions yet. Start a chat in Claude Code and Grace will pick it up.\n".into();
    }
    let mut out = format!(
        "{} (most recent first)\n\n",
        count(u32::try_from(rows.len()).unwrap_or(u32::MAX), "session")
    );
    for row in rows {
        let title = truncate_chars(row.title.as_deref().unwrap_or("Untitled session"), 44);
        let project = truncate_chars(row.project_name.as_deref().unwrap_or("–"), 18);
        let when = row
            .last_active_at
            .as_deref()
            .and_then(timefmt::parse_rfc3339_ms)
            .map(|ms| relative_time(now_ms - ms))
            .unwrap_or_default();
        let compactions = if row.compactions > 0 {
            count(row.compactions, "compaction")
        } else {
            String::new()
        };
        let line = format!(
            "  {title:<44}  {project:<18}  {when:<14}  {:<9}  {compactions}",
            count(row.turns, "turn")
        );
        out += line.trim_end();
        out.push('\n');
    }
    out
}

pub fn relative_time(ago_ms: i64) -> String {
    let minutes = ago_ms / 60_000;
    match minutes {
        m if m < 1 => "just now".into(),
        1 => "1 minute ago".into(),
        m if m < 60 => format!("{m} minutes ago"),
        m if m < 120 => "1 hour ago".into(),
        m if m < 1_440 => format!("{} hours ago", m / 60),
        m if m < 2_880 => "yesterday".into(),
        m if m < 43_200 => format!("{} days ago", m / 1_440),
        m if m < 86_400 => "1 month ago".into(),
        m => format!("{} months ago", m / 43_200),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_times() {
        let minute = 60_000;
        assert_eq!(relative_time(10_000), "just now");
        assert_eq!(relative_time(minute), "1 minute ago");
        assert_eq!(relative_time(5 * minute), "5 minutes ago");
        assert_eq!(relative_time(3 * 60 * minute), "3 hours ago");
        assert_eq!(relative_time(30 * 60 * minute), "yesterday");
        assert_eq!(relative_time(4 * 24 * 60 * minute), "4 days ago");
        assert_eq!(relative_time(75 * 24 * 60 * minute), "2 months ago");
    }
}
