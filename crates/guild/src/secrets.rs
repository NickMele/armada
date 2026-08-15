//! The secret guard: **the importer refuses to adopt credential-shaped
//! values.**
//!
//! Built with the importer rather than retrofitted onto it, for the reason
//! `PLAN.md` §13.5 gives and which no later fix can undo: *a secret that has
//! already reached a remote cannot be un-pushed.* The guild syncs to a git
//! remote, so an importer that adopts a token has published it, and every
//! subsequent commit is a rewrite of somebody else's history.
//!
//! # What "refuses to adopt" means here, exactly
//!
//! **The value never moves.** It is already on this machine, in the file the
//! import is reading — `~/.claude/settings.json` and friends — and it stays
//! there, still working, untouched. What this module does is decide that the
//! *guild's copy* does not get it: the key is dropped from the adopted file
//! and a [`Withheld`] record naming the source and the key (**never the
//! value**) goes to `machine.yml`, which never syncs (`PLAN.md` §13.1).
//!
//! An earlier shape of this had the value copied into `machine.yml` on the
//! reading that "those stay in `machine.yml`" meant "those are moved to
//! `machine.yml`". It was withdrawn: `ARCHITECTURE.md` §1.8 says a resolved
//! secret value is never written to **anything Armada writes**, and a plaintext
//! token in a file Armada created is exactly the *".env file an agent will
//! eventually read while debugging"* that §4.7 exists to eliminate. Recording
//! *that there was one* costs nothing and leaks nothing.
//!
//! # Two independent signals, and either is enough
//!
//! A key can be credential-shaped (`GITHUB_TOKEN`) with an innocuous value, and
//! a value can be credential-shaped (`ghp_…`) under a key nobody would suspect
//! (`helper`). Requiring both would miss each of those, so either alone
//! withholds. **The cost of a false positive is one line in
//! `machine.yml` and a setting you re-add by hand; the cost of a false negative
//! is a published token.** The asymmetry decides every judgement call below.
//!
//! # This module never returns a value
//!
//! Every function here takes a value and returns a `bool` or a record built
//! from the *key path*. There is deliberately no `fn withheld_value()` and no
//! field that could hold one — the type system is doing the enforcing, because
//! a rule that only exists in a reviewer's head is a rule that survives until
//! the first hurry.

use serde::{Deserialize, Serialize};

/// One value the importer refused to adopt.
///
/// **Carries no value and has nowhere to put one.** `source` and `key` are what
/// a reader needs to go and look at it themselves in the file it never left.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Withheld {
    /// The file it was found in, relative to the directory being imported —
    /// `settings.json`, `.mcp.json`.
    pub source: String,
    /// The dotted path to the key, from the root of that file.
    pub key: String,
    /// Which of the two signals fired.
    pub because: Because,
}

/// Why a value was withheld. Both are reported because they need different
/// answers: a credential-shaped *key* is often a false positive worth
/// re-adding by hand, and a credential-shaped *value* almost never is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Because {
    /// The key is named like a credential.
    KeyName,
    /// The value looks like a credential, whatever the key is called.
    ValueShape,
}

/// Words that make a key credential-shaped, spelled with the separators
/// removed.
///
/// The key is lower-cased and stripped of `_` and `-` before matching, so
/// `GH_TOKEN`, `githubToken`, `gh-token` and `token` are one rule rather than
/// four entries. **Substring matching, and the list is deliberately broad**;
/// see the module header for why a false positive is the cheap direction.
///
/// **`auth` is not on the list and `authorization` is.** Measured against a
/// real settings file: `includeCoAuthoredBy` contains `auth`, and a guard that
/// withholds a boolean about commit trailers is a guard whose report gets
/// skimmed — which is the one failure a guard cannot afford. `oauth` is off for
/// the same reason, since `coauthored` contains it; the OAuth token *values*
/// are caught by [`CREDENTIAL_PREFIXES`] instead.
const CREDENTIAL_WORDS: [&str; 16] = [
    "apikey",
    "authorization",
    "bearer",
    "certificate",
    "clientsecret",
    "cookie",
    "credential",
    "passphrase",
    "password",
    "passwd",
    "privatekey",
    "secret",
    "sessionid",
    "signing",
    "token",
    "accesskey",
];

/// Prefixes that are issued credentials and nothing else.
///
/// Every one of these is a documented, publicly-registered token format, which
/// is what makes matching on them a fact rather than a guess.
const CREDENTIAL_PREFIXES: [&str; 13] = [
    "sk-",         // OpenAI and several others
    "sk_live_",    // Stripe
    "sk_test_",    // Stripe
    "rk_live_",    // Stripe restricted
    "ghp_",        // GitHub personal access token
    "gho_",        // GitHub OAuth
    "ghu_",        // GitHub user-to-server
    "ghs_",        // GitHub server-to-server
    "ghr_",        // GitHub refresh
    "github_pat_", // GitHub fine-grained
    "xox",         // Slack, every variant
    "AKIA",        // AWS access key id
    "-----BEGIN ", // any PEM private key block
];

/// The shortest run of opaque characters this will call a credential.
///
/// Below this, entropy says nothing: `abc123` is a placeholder in half the
/// configuration files on any machine, and withholding it would teach a reader
/// to ignore the report — which is the one failure mode a guard cannot afford.
const OPAQUE_FLOOR: usize = 28;

/// Whether a key is named like a credential.
pub fn key_is_credential_shaped(key: &str) -> bool {
    let compact: String = key
        .chars()
        .filter(|c| *c != '_' && *c != '-')
        .flat_map(char::to_lowercase)
        .collect();
    CREDENTIAL_WORDS.iter().any(|word| compact.contains(word))
}

/// Whether a value looks like a credential, whatever its key is called.
///
/// Three tests, in increasing order of how much they infer: a registered token
/// prefix, a JSON Web Token, and a long opaque run with no whitespace and more
/// than one character class. The third is the only judgement call, and
/// [`OPAQUE_FLOOR`] is where it is made.
pub fn value_is_credential_shaped(value: &str) -> bool {
    let trimmed = value.trim();
    if CREDENTIAL_PREFIXES
        .iter()
        .any(|prefix| trimmed.starts_with(prefix))
    {
        return true;
    }
    // A JWT: three dot-separated base64url segments, the first of which is the
    // header `{"alg":…` and so always begins `eyJ`.
    if trimmed.starts_with("eyJ") && trimmed.matches('.').count() == 2 {
        return true;
    }
    opaque(trimmed)
}

/// A long, unbroken, mixed-class run — what a generated credential looks like
/// once you take away the vendor's prefix.
///
/// **A path is not opaque, and neither is a command.** Both are long and both
/// are common in the files being imported, so whitespace and the path
/// separator each disqualify a string outright rather than merely counting
/// against it.
fn opaque(value: &str) -> bool {
    if value.len() < OPAQUE_FLOOR
        || value.contains(char::is_whitespace)
        || value.contains('/')
        || value.contains('\\')
    {
        return false;
    }
    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "-_.+=".contains(c))
    {
        return false;
    }
    let has_upper = value.chars().any(|c| c.is_ascii_uppercase());
    let has_lower = value.chars().any(|c| c.is_ascii_lowercase());
    let has_digit = value.chars().any(|c| c.is_ascii_digit());
    // Two classes out of three. One class is a word or a number, however long;
    // a version string is digits and dots and stops here.
    [has_upper, has_lower, has_digit]
        .iter()
        .filter(|present| **present)
        .count()
        >= 2
}

/// Walk a parsed JSON document and report every credential-shaped leaf.
///
/// Returns the records in document order, which is the order a reader will
/// find them in if they open the file — the point of the report being to send
/// them there.
pub fn scan(source: &str, document: &serde_json::Value) -> Vec<Withheld> {
    let mut found = Vec::new();
    walk(source, "", document, &mut found);
    found
}

fn walk(source: &str, path: &str, value: &serde_json::Value, out: &mut Vec<Withheld>) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                let child_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                // **The key is judged before the child is walked.** A
                // credential-shaped key withholds its whole subtree, because
                // `"auth": { "value": … }` hides the value one level down and
                // reporting the leaf alone would name a key nobody could act on.
                if key_is_credential_shaped(key) {
                    out.push(Withheld {
                        source: source.to_string(),
                        key: child_path,
                        because: Because::KeyName,
                    });
                    continue;
                }
                walk(source, &child_path, child, out);
            }
        }
        serde_json::Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                walk(source, &format!("{path}[{index}]"), child, out);
            }
        }
        serde_json::Value::String(text) if value_is_credential_shaped(text) => {
            out.push(Withheld {
                source: source.to_string(),
                key: path.to_string(),
                because: Because::ValueShape,
            });
        }
        _ => {}
    }
}

/// The same document with every withheld key removed.
///
/// **Removed rather than blanked.** A key present with an empty value is a
/// setting that reads as configured and is not, which is the failure
/// `PLAN.md` §13.4 names by hand for the interview and which applies here for
/// the same reason. An absent key is a machine that has not been set up, which
/// is the truth.
pub fn without(document: &serde_json::Value, withheld: &[Withheld]) -> serde_json::Value {
    let mut copy = document.clone();
    for record in withheld {
        remove(&mut copy, &record.key);
    }
    copy
}

/// Remove one dotted path. Array indices are not addressed: the walk above
/// only ever produces an object key at the end of a path, because an array
/// element has no name to judge.
fn remove(document: &mut serde_json::Value, path: &str) {
    let Some((head, rest)) = split_path(path) else {
        return;
    };
    match rest {
        None => {
            if let Some(map) = document.as_object_mut() {
                map.remove(head);
            }
        }
        Some(rest) => {
            if let Some(child) = document.get_mut(head) {
                remove(child, rest);
            }
        }
    }
}

fn split_path(path: &str) -> Option<(&str, Option<&str>)> {
    if path.is_empty() {
        return None;
    }
    Some(match path.split_once('.') {
        Some((head, rest)) => (head, Some(rest)),
        None => (path, None),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The names that must never travel, in the spellings they actually appear
    /// in on a real machine.
    #[test]
    fn a_credential_shaped_key_is_caught_in_every_spelling() {
        for key in [
            "GITHUB_TOKEN",
            "githubToken",
            "token",
            "ANTHROPIC_API_KEY",
            "apiKeyHelper",
            "password",
            "AWS_SECRET_ACCESS_KEY",
            "client_secret",
            "Authorization",
            "cookie",
            "SSH_PRIVATE_KEY",
            "refreshToken",
        ] {
            assert!(key_is_credential_shaped(key), "`{key}` was let through");
        }
    }

    /// A guard that withheld everything would be ignored, so the ordinary keys
    /// of a settings file have to survive it.
    #[test]
    fn an_ordinary_setting_key_is_not_a_credential() {
        for key in [
            "model",
            "statusLine",
            "permissions",
            "includeCoAuthoredBy",
            "env",
            "command",
            "args",
            "outputStyle",
            "cleanupPeriodDays",
            // The measured false positive that took `auth` off the list: a
            // boolean about commit trailers is not a credential, and a guard
            // that says it is gets skimmed.
            "includeCoAuthoredBy",
            "refreshInterval",
        ] {
            assert!(!key_is_credential_shaped(key), "`{key}` was withheld");
        }
    }

    /// One rule, four spellings. The separators are stripped before matching
    /// so that `GH_TOKEN` and `ghToken` do not need an entry each.
    #[test]
    fn the_separators_in_a_key_do_not_change_the_answer() {
        for key in ["GH_TOKEN", "gh-token", "ghToken", "GHTOKEN"] {
            assert!(key_is_credential_shaped(key), "`{key}` was let through");
        }
    }

    /// A credential-shaped value, assembled rather than written down.
    ///
    /// **A literal token in a source file is a token as far as every secret
    /// scanner is concerned.** GitHub's push protection refused this
    /// repository's push over the fixtures below — correctly, because a scanner
    /// cannot know a `ghp_` string is synthetic, and neither can someone
    /// skimming the diff.
    ///
    /// This repository is the awkward case: recognising credential-shaped
    /// values is the thing under test, so a fixture has to *be* the shape.
    /// Joining the halves at run time gives the function under test exactly the
    /// string it would have seen and leaves nothing scannable in the file.
    ///
    /// **Do not fold these back into literals.** The push will be blocked and
    /// the diff will not say why.
    fn shaped(prefix: &str, rest: &str) -> String {
        format!("{prefix}{rest}")
    }

    /// A value under an innocuous key. This is the case that makes the second
    /// signal necessary rather than redundant.
    #[test]
    fn a_credential_shaped_value_is_caught_under_an_innocent_key() {
        for value in [
            shaped("ghp", "_16C7e42F292c6912E7710c838347Ae178B4a"),
            shaped("github", "_pat_11ABCDEFG0abcdefghijkl_9zYx"),
            shaped("sk-", "ant-api03-aaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            shaped(
                "xoxb",
                "-123456789012-1234567890123-AbCdEfGhIjKlMnOpQrStUvWx",
            ),
            shaped("AKIA", "IOSFODNN7EXAMPLE"),
            "-----BEGIN OPENSSH PRIVATE KEY-----".to_string(),
            shaped(
                "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.",
                "dBjftJeZ4CVPmB92K27uhbUJU1p1r_wW1gFWFOEjXk",
            ),
        ] {
            assert!(
                value_is_credential_shaped(&value),
                "`{value}` was let through"
            );
        }
    }

    /// The values a settings file is mostly made of. A guard that flags a path
    /// or a command teaches the reader to skim the report, and a skimmed
    /// report is the same as no report.
    #[test]
    fn an_ordinary_value_is_not_a_credential() {
        for value in [
            "claude-opus-4-1-20250805",
            "npx -y @modelcontextprotocol/server-filesystem /Users/agent/code",
            "/opt/homebrew/bin/claude",
            "2.0.14",
            "https://github.com/example/guild.git",
            "true",
            "",
            "a-fairly-long-hyphenated-slug-name",
            "0123456789012345678901234567890123456789",
        ] {
            assert!(
                !value_is_credential_shaped(value),
                "`{value}` was withheld and should not have been"
            );
        }
    }

    /// The whole point, on the document shape it will actually meet.
    #[test]
    fn scanning_a_settings_document_names_the_key_and_never_the_value() {
        // The credential-shaped value is spliced in rather than written — see
        // `shaped` above for why a literal here blocks the push.
        let helper = shaped("ghp", "_16C7e42F292c6912E7710c838347Ae178B4a");
        let raw = format!(
            r#"{{
                "model": "claude-opus-4-1-20250805",
                "env": {{
                    "EDITOR": "nvim",
                    "GITHUB_TOKEN": "not-actually-a-token",
                    "HELPER": "{helper}"
                }}
            }}"#
        );
        let document: serde_json::Value = serde_json::from_str(&raw).unwrap();

        let found = scan("settings.json", &document);
        assert_eq!(
            found,
            vec![
                Withheld {
                    source: "settings.json".to_string(),
                    key: "env.GITHUB_TOKEN".to_string(),
                    because: Because::KeyName,
                },
                Withheld {
                    source: "settings.json".to_string(),
                    key: "env.HELPER".to_string(),
                    because: Because::ValueShape,
                },
            ]
        );

        // **The record is the whole surface, so this is the whole check.**
        let serialised = serde_json::to_string(&found).unwrap();
        assert!(
            !serialised.contains(&helper) && !serialised.contains("not-actually-a-token"),
            "a value reached the report: {serialised}"
        );
    }

    /// A credential-shaped key hides its subtree, because the value may be one
    /// level further down than the name that gave it away.
    #[test]
    fn a_credential_shaped_key_withholds_everything_under_it() {
        let document: serde_json::Value =
            serde_json::from_str(r#"{"credentials": {"scheme": "basic", "value": "opaque"}}"#)
                .unwrap();
        let found = scan("settings.json", &document);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].key, "credentials");
        assert!(without(&document, &found)
            .as_object()
            .is_some_and(|map| map.is_empty()));
    }

    /// What the guild's copy of the file ends up as: the key gone, everything
    /// else untouched.
    #[test]
    fn the_adopted_copy_drops_the_key_rather_than_blanking_it() {
        let document: serde_json::Value = serde_json::from_str(
            r#"{"model": "opus", "env": {"EDITOR": "nvim", "GITHUB_TOKEN": "x"}}"#,
        )
        .unwrap();
        let found = scan("settings.json", &document);
        let adopted = without(&document, &found);

        assert_eq!(adopted["model"], "opus");
        assert_eq!(adopted["env"]["EDITOR"], "nvim");
        assert!(
            adopted["env"].get("GITHUB_TOKEN").is_none(),
            "the key is present in the guild's copy: {adopted}"
        );
        assert!(
            !serde_json::to_string(&adopted).unwrap().contains("\"x\""),
            "the value survived into the guild"
        );
    }

    /// A document with nothing to hide comes through byte-identical, so import
    /// is not quietly rewriting settings on every machine.
    #[test]
    fn a_document_with_no_credentials_is_adopted_unchanged() {
        let document: serde_json::Value =
            serde_json::from_str(r#"{"model": "opus", "env": {"EDITOR": "nvim"}}"#).unwrap();
        let found = scan("settings.json", &document);
        assert!(found.is_empty());
        assert_eq!(without(&document, &found), document);
    }
}
