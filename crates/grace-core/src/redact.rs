//! Redaction: removes secrets and personal data before anything is stored or
//! shown (spec §12, R8). Replacements read `[redacted:<kind>]`.

use regex::{Captures, Regex, RegexSet};
use std::borrow::Cow;
use std::sync::LazyLock;

const API_KEY: &str = "[redacted:api-key]";
const TOKEN: &str = "[redacted:token]";

/// One pattern plus a check that decides whether a match is really a secret
/// (`None` keeps the original text).
struct Rule {
    re: Regex,
    replace: fn(&Captures) -> Option<String>,
}

fn rule(pattern: &str, replace: fn(&Captures) -> Option<String>) -> Rule {
    Rule {
        re: Regex::new(pattern).expect("redaction pattern is valid"),
        replace,
    }
}

fn has_digit_and_letter(text: &str) -> bool {
    text.chars().any(|c| c.is_ascii_digit()) && text.chars().any(|c| c.is_ascii_alphabetic())
}

/// `name = value` and `"name": "value"` pairs whose name suggests a secret.
fn assignment(c: &Captures) -> Option<String> {
    let name = c[1].to_ascii_lowercase();
    let value = &c[3];
    if value.starts_with("[redacted") {
        return None;
    }
    let lower = value.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "true"
            | "false"
            | "null"
            | "none"
            | "undefined"
            | "required"
            | "optional"
            | "string"
            | "yes"
            | "no"
    ) {
        return None;
    }
    let is_password = name.contains("pass") || name.contains("pwd") || name.contains("secret");
    let looks_secret = if is_password {
        value.chars().count() >= 3
    } else {
        (value.len() >= 12 && has_digit_and_letter(value))
            || (value.len() >= 24 && !value.chars().all(|ch| ch.is_ascii_digit()))
    };
    if !looks_secret {
        return None;
    }
    let kind = if is_password { "password" } else { "token" };
    Some(format!("{}{}[redacted:{kind}]", &c[1], &c[2]))
}

/// Ordered from most to least specific; later rules never undo earlier ones.
static RULES: LazyLock<Vec<Rule>> = LazyLock::new(|| {
    vec![
        // PEM private keys, complete or cut off.
        rule(
            r"(?s)-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----.*?(?:-----END [A-Z0-9 ]*PRIVATE KEY-----|\z)",
            |_| Some("[redacted:private-key]".into()),
        ),
        rule(
            r"\beyJ[A-Za-z0-9_-]{8,}\.eyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}",
            |_| Some("[redacted:jwt]".into()),
        ),
        rule(r"\bsk-ant-[A-Za-z0-9_-]{20,}", |_| Some(API_KEY.into())),
        rule(r"\bsk-[A-Za-z0-9_-]{32,}", |c| {
            has_digit_and_letter(&c[0][3..]).then(|| API_KEY.into())
        }),
        rule(
            r"\b(?:gh[pousr]_[A-Za-z0-9]{36,}|github_pat_[A-Za-z0-9_]{22,})",
            |_| Some(TOKEN.into()),
        ),
        rule(r"\b(?:AKIA|ASIA)[A-Z0-9]{16}\b", |_| Some(API_KEY.into())),
        rule(r"\bAIza[0-9A-Za-z_-]{35}", |_| Some(API_KEY.into())),
        rule(r"\bxox[abprs]-[A-Za-z0-9-]{10,}", |_| Some(TOKEN.into())),
        rule(r"\b(?:sk|rk)_(?:live|test)_[A-Za-z0-9]{16,}", |_| {
            Some(API_KEY.into())
        }),
        rule(r"(?i)\b(bearer)\s+[A-Za-z0-9._~+/=-]{16,}", |c| {
            Some(format!("{} {TOKEN}", &c[1]))
        }),
        rule(r"(?i)\b(basic)\s+([A-Za-z0-9+/]{12,}={0,2})", |c| {
            let value = &c[2];
            (has_digit_and_letter(value) || value.contains(['+', '/', '=']))
                .then(|| format!("{} [redacted:credentials]", &c[1]))
        }),
        rule(
            r"\b([a-zA-Z][a-zA-Z0-9+.-]*://[^\s:/@]+):([^\s@/]+)@",
            |c| {
                (!c[2].starts_with("[redacted"))
                    .then(|| format!("{}:[redacted:credentials]@", &c[1]))
            },
        ),
        rule(
            r#"(?i)\b([A-Za-z0-9_.-]*(?:passwd|password|pwd|secret|token|api[_-]?key|apikey|access[_-]?key|auth)[A-Za-z0-9_.-]*)(["']?\s*[:=]\s*["']?)([^\s"',;]+)"#,
            assignment,
        ),
    ]
});

/// Fast path: one pass to see whether any rule could match at all.
static ANY_RULE: LazyLock<RegexSet> = LazyLock::new(|| {
    RegexSet::new(RULES.iter().map(|r| r.re.as_str())).expect("redaction patterns are valid")
});

static EMAIL: LazyLock<Rule> = LazyLock::new(|| {
    rule(
        r"\b([A-Za-z0-9._%+-]+)@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)*\.[A-Za-z]{2,}\b",
        // `git@host:org/repo` is an SSH address, not a person.
        |c| (&c[1] != "git").then(|| "[redacted:email]".into()),
    )
});

/// Applies `rule` to `text`, keeping it borrowed when nothing changes.
fn apply<'a>(rule: &Rule, text: Cow<'a, str>) -> Cow<'a, str> {
    let replaced = match rule.re.replace_all(&text, |c: &Captures| {
        (rule.replace)(c).unwrap_or_else(|| c[0].to_string())
    }) {
        Cow::Borrowed(_) => None,
        Cow::Owned(new) => Some(new),
    };
    match replaced {
        Some(new) if new != *text => Cow::Owned(new),
        _ => text,
    }
}

/// Removes secrets (and, by default, email addresses) from text.
#[derive(Debug, Clone)]
pub struct Redactor {
    emails: bool,
}

impl Redactor {
    pub fn new(redact_emails: bool) -> Self {
        Self {
            emails: redact_emails,
        }
    }

    /// Returns `text` with every secret replaced. Borrowed when nothing changed.
    pub fn redact<'a>(&self, text: &'a str) -> Cow<'a, str> {
        let mut out = Cow::Borrowed(text);
        let hits = ANY_RULE.matches(text);
        if hits.matched_any() {
            for (index, rule) in RULES.iter().enumerate() {
                if hits.matched(index) {
                    out = apply(rule, out);
                }
            }
        }
        if self.emails && out.contains('@') {
            out = apply(&EMAIL, out);
        }
        out
    }

    /// A one-line, redacted summary of at most `max` characters.
    pub fn summary(&self, text: &str, max: usize) -> String {
        truncate_chars(
            &one_line(&self.redact(prefix_chars(text, max * 4 + 64))),
            max,
        )
    }
}

impl Default for Redactor {
    fn default() -> Self {
        Self::new(true)
    }
}

/// Cuts `text` to at most `max` characters, ending with `…` when cut. Never splits a character.
pub fn truncate_chars(text: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    if text.char_indices().nth(max).is_none() {
        return text.to_string();
    }
    let cut = text
        .char_indices()
        .nth(max - 1)
        .map_or(0, |(index, _)| index);
    format!("{}…", &text[..cut])
}

/// The first `max` characters of `text`, without an ellipsis.
pub fn prefix_chars(text: &str, max: usize) -> &str {
    match text.char_indices().nth(max) {
        Some((index, _)) => &text[..index],
        None => text,
    }
}

/// Collapses every run of whitespace, including newlines, into one space.
pub fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(text: &str) -> String {
        Redactor::new(true).redact(text).into_owned()
    }

    #[test]
    fn redacts_anthropic_key() {
        let out =
            r("export ANTHROPIC_API_KEY sk-ant-api03-AbCdEf0123456789_xyzXYZ-0123456789abcdef");
        assert!(out.contains("[redacted:api-key]"), "{out}");
        assert!(!out.contains("sk-ant-api03"), "{out}");
    }

    #[test]
    fn redacts_openai_keys() {
        assert_eq!(
            r("key sk-proj-Ab12Cd34Ef56Gh78Ij90Kl12Mn34Op56Qr78"),
            "key [redacted:api-key]"
        );
        assert_eq!(
            r("sk-0123456789abcdefABCDEF0123456789abcd"),
            "[redacted:api-key]"
        );
    }

    #[test]
    fn redacts_github_tokens() {
        assert_eq!(
            r("ghp_0123456789abcdefghijABCDEFGHIJ012345"),
            "[redacted:token]"
        );
        assert_eq!(
            r("gho_0123456789abcdefghijABCDEFGHIJ012345"),
            "[redacted:token]"
        );
        assert_eq!(
            r("github_pat_11ABCDEFG0123456789_abcdefghijklmnopqrstuv"),
            "[redacted:token]"
        );
    }

    #[test]
    fn redacts_cloud_and_saas_keys() {
        assert_eq!(r("AKIAIOSFODNN7EXAMPLE"), "[redacted:api-key]");
        assert_eq!(
            r("AIzaSyA-0123456789abcdefghijklmnopqrstu"),
            "[redacted:api-key]"
        );
        assert_eq!(r("xoxb-123456789012-abcdefABCDEF"), "[redacted:token]");
        assert_eq!(r("sk_live_0123456789abcdefABCD"), "[redacted:api-key]");
        assert_eq!(r("rk_live_0123456789abcdefABCD"), "[redacted:api-key]");
    }

    #[test]
    fn redacts_jwt() {
        let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        assert_eq!(r(&format!("token {jwt} end")), "token [redacted:jwt] end");
    }

    #[test]
    fn redacts_private_key_blocks_even_when_cut_off() {
        let full = "a\n-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAA\n-----END OPENSSH PRIVATE KEY-----\nb";
        assert_eq!(r(full), "a\n[redacted:private-key]\nb");
        let cut = "x -----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA";
        assert_eq!(r(cut), "x [redacted:private-key]");
    }

    #[test]
    fn redacts_bearer_and_basic_auth() {
        assert_eq!(
            r("Authorization: Bearer abcdef0123456789abcdef"),
            "Authorization: Bearer [redacted:token]"
        );
        assert_eq!(
            r("Authorization: Basic dXNlcjpwYXNzd29yZDEyMw=="),
            "Authorization: Basic [redacted:credentials]"
        );
    }

    #[test]
    fn redacts_url_credentials() {
        assert_eq!(
            r("git clone https://user:p4ssw0rd@example.com/repo.git"),
            "git clone https://user:[redacted:credentials]@example.com/repo.git"
        );
    }

    #[test]
    fn redacts_assignments() {
        assert_eq!(r("password=hunter2"), "password=[redacted:password]");
        assert_eq!(
            r("DB_PASSWORD: correct-horse"),
            "DB_PASSWORD: [redacted:password]"
        );
        assert_eq!(
            r(r#"{"api_key": "abc123def456ghi789"}"#),
            r#"{"api_key": "[redacted:token]"}"#
        );
    }

    #[test]
    fn redacts_email_by_default_and_keeps_it_when_disabled() {
        assert_eq!(
            r("mail jane.doe+x@example.co.uk now"),
            "mail [redacted:email] now"
        );
        let keep = Redactor::new(false).redact("jane@example.com").into_owned();
        assert_eq!(keep, "jane@example.com");
    }

    #[test]
    fn keeps_normal_text() {
        for text in [
            "Use pnpm, never npm.",
            "550e8400-e29b-41d4-a716-446655440000",
            "commit 9fceb02d0ae598e95dc970b74767f19372d61af8",
            "the sk- prefix",
            "AKIA alone",
            "tokens: 150000",
            "max_tokens: 4096",
            "bearer tokens are short",
            "basic configuration",
            "git@github.com:smitkohale/grace.git",
            "react@18.2.0",
        ] {
            assert_eq!(r(text), text, "changed: {text}");
        }
    }

    #[test]
    fn is_idempotent() {
        let once = r("password=hunter2 sk-ant-api03-AbCdEf0123456789_xyzXYZ-0123456789 a@b.io");
        assert_eq!(r(&once), once);
    }

    #[test]
    fn unchanged_text_is_borrowed() {
        let redactor = Redactor::new(true);
        assert!(matches!(redactor.redact("plain words"), Cow::Borrowed(_)));
    }

    #[test]
    fn truncates_on_char_boundaries() {
        assert_eq!(truncate_chars("hello", 10), "hello");
        assert_eq!(truncate_chars("hello world", 6), "hello…");
        assert_eq!(truncate_chars("héllo wörld", 3), "hé…");
        assert_eq!(truncate_chars("abc", 0), "");
    }

    #[test]
    fn summary_is_one_redacted_line() {
        let s = Redactor::new(true).summary("line one\n  line two with password=hunter2", 30);
        assert_eq!(s, "line one line two with passwo…");
        let s = Redactor::new(true).summary("pw password=hunter2", 40);
        assert_eq!(s, "pw password=[redacted:password]");
    }
}
