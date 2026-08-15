//! **Nothing Armada writes down about a run may carry a secret.**
//!
//! The rule was stated in one clause — *"but don't include secrets"* — and it
//! binds harder than anything else in `armada report`, because a report attaches
//! command lines, environment-derived text and captured output, and all three
//! routinely carry tokens. The repository is public permanently and the eventual
//! GitHub-issue path makes that worse rather than better: **a record that has
//! already leaked a secret locally cannot be un-published once it is.**
//!
//! # One detector, and this is not it
//!
//! [`armada_guild::secrets`] already decides whether a *value* looks like a
//! credential — registered vendor prefixes, JWTs, and long opaque mixed-class
//! runs — and whether a *key* is named like one. That rule is not restated here.
//! What this module adds is the part Guild's importer never needed: Guild walks
//! a parsed JSON document, where a value is a leaf and its key is in hand. A
//! report attaches **prose and shell lines**, where the same value arrives as
//! `GITHUB_TOKEN=ghp_…`, as `--token ghp_…`, or as a bare word in a sentence.
//! So this is a *tokeniser* over Guild's predicate, not a second predicate.
//!
//! # Where it lives, and why not lower
//!
//! Helm is the only crate that may name both Guild and the core
//! (`ARCHITECTURE.md` §1.9), and the two places redaction has to happen — the
//! entrypoint that writes the ring buffer, and the `report` verb — are both
//! Helm's. Pushing the detector down into `armada-core` so that
//! [`armada_core::failure`] could call it directly was the alternative, and it
//! was declined: it would move a rule out from under the module that owns
//! secrets in order to save passing a function pointer. Core takes the
//! redactor as an argument instead ([`armada_core::failure::reported`]), which
//! keeps the *walk over every field* in the core where the record lives and the
//! *rule* in Guild where the detector lives.
//!
//! # Three deliberate asymmetries
//!
//! - **A false positive costs a word; a false negative costs a token.** A
//!   redacted version string is a mild annoyance in a bug report. That is Guild's
//!   own stated asymmetry and every judgement below follows it.
//! - **The key is redacted from, never with.** `GITHUB_TOKEN=[redacted]` keeps
//!   the fact that the variable was set, which is often the diagnostic, and
//!   loses only the part nobody may see.
//! - **Redaction happens on the way in, not on the way out.** The ring buffer is
//!   scrubbed as each run ends rather than when a report reads it, so a
//!   credential is never at rest in `~/.armada/` even for the runs nobody ever
//!   reports.

use armada_guild::secrets::{key_is_credential_shaped, value_is_credential_shaped};

/// What replaces a value that must not be written down.
///
/// **A word rather than stars**, because a reader has to be able to tell a
/// redaction from a value that genuinely looks like `****` — and because the
/// test that proves nothing leaked asserts on this string.
pub const MASK: &str = "[redacted]";

/// Characters that end a value inside prose or a shell line.
///
/// **Quotes, brackets and sentence punctuation.** `"ghp_…"` in a JSON payload
/// and `ghp_…,` at the end of a clause are the same value with furniture around
/// it, and a tokeniser that kept the furniture attached would ask Guild about a
/// string Guild has never seen.
const FURNITURE: &[char] = &[
    '"', '\'', '`', ',', ';', '(', ')', '[', ']', '{', '}', '<', '>',
];

/// Every credential-shaped value in this text, replaced by [`MASK`].
///
/// **Three shapes, because a secret reaches a report in three ways:**
///
/// | Shape | Where it comes from | What survives |
/// |---|---|---|
/// | `NAME=value` | the environment, and `--flag=value` | the name |
/// | `--flag value` | a command line | the flag |
/// | a bare word | captured output, and prose | nothing of it |
///
/// The first two are keyed: the *name* decides, so `GITHUB_TOKEN=hunter2`
/// redacts even though `hunter2` is nobody's idea of opaque. The third is
/// unkeyed and the value alone has to carry it, which is exactly what
/// [`value_is_credential_shaped`] is for.
///
/// **Line structure is preserved and whitespace is not normalised**, because
/// this runs over captured envelopes and a redactor that reflowed JSON would
/// make every attachment unreadable to the agent it exists for.
pub fn scrub(text: &str) -> String {
    text.split_inclusive('\n')
        .map(scrub_line)
        .collect::<Vec<_>>()
        .join("")
}

fn scrub_line(line: &str) -> String {
    let ending = match line.strip_suffix('\n') {
        Some(_) => "\n",
        None => "",
    };
    let body = line.strip_suffix('\n').unwrap_or(line);

    // **Split keeping the gaps**, so `  "key": "value"` comes back with its
    // indentation intact. `split_inclusive` on whitespace is not enough — the
    // gap has to survive on the *left* of each word, since a word is what gets
    // replaced.
    let mut out = String::with_capacity(body.len());
    let mut keyed = false;
    let mut word = String::new();
    let mut gap = String::new();
    let flush = |word: &mut String, gap: &mut String, out: &mut String, keyed: &mut bool| {
        out.push_str(gap);
        gap.clear();
        if word.is_empty() {
            return;
        }
        let (replacement, names_a_credential) = scrub_word(word, *keyed);
        out.push_str(&replacement);
        *keyed = names_a_credential;
        word.clear();
    };

    for character in body.chars() {
        if character.is_whitespace() {
            flush(&mut word, &mut gap, &mut out, &mut keyed);
            gap.push(character);
        } else {
            word.push(character);
        }
    }
    flush(&mut word, &mut gap, &mut out, &mut keyed);
    out.push_str(ending);
    out
}

/// One word, and whether the *next* word is one this word named as a secret.
///
/// The second half of the answer is what makes `--token ghp_…` work at all: the
/// value is a separate word, so the decision about it was made one word earlier.
fn scrub_word(word: &str, keyed: bool) -> (String, bool) {
    // The furniture is peeled off both ends and put back, so a value inside a
    // JSON payload is asked about as the value it is.
    let core = word.trim_matches(|c| FURNITURE.contains(&c));
    let (lead, trail) = split_furniture(word, core);

    // **Scrubbing twice must be scrubbing once.** The ring buffer is redacted as
    // each run ends, and then again when a report attaches it — which is
    // deliberate belt and braces — so without this a masked value comes back as
    // `[[redacted]]` and then `[[[redacted]]]`, and the reader learns to
    // distrust the mask.
    if word.contains(MASK) {
        return (word.to_string(), false);
    }

    // Shape one: `NAME=value`, which covers the environment and `--flag=value`.
    if let Some((name, value)) = core.split_once('=') {
        // **A bare `=` at the end is not an assignment.** `--force=` names no
        // value, and base64 padding (`abc==`) is part of the value, not a
        // separator — which is why the name has to look like a name.
        if !name.is_empty() && names_a_value(name) {
            let secret = key_is_credential_shaped(strip_dashes(name))
                || (!value.is_empty() && value_is_credential_shaped(value));
            if secret {
                return (format!("{lead}{name}={MASK}{trail}"), false);
            }
            return (word.to_string(), false);
        }
    }

    // Shape two: the previous word was `--token` or `GITHUB_TOKEN`, so this one
    // is its value whatever it looks like.
    if keyed {
        return (format!("{lead}{MASK}{trail}"), false);
    }

    // Shape three: a bare value that carries its own evidence.
    if !core.is_empty() && value_is_credential_shaped(core) {
        return (format!("{lead}{MASK}{trail}"), false);
    }

    // **Only a flag arms the next word, never a bare noun.** `armada report
    // "the token was wrong"` must not redact `was`, and a sentence containing
    // the word *password* must not eat the word after it. A flag is a name
    // somebody typed as a flag, which is a much narrower thing than a word that
    // happens to appear in prose.
    let arms = word.starts_with('-') && key_is_credential_shaped(strip_dashes(core));
    (word.to_string(), arms)
}

/// Whether this looks like the left-hand side of an assignment rather than part
/// of a value.
///
/// **Letters, digits, `_`, `-` and `.` only.** A base64 blob ending in `=` has
/// characters no variable name has, so it falls through to the value tests
/// instead of being read as `blob=`.
fn names_a_value(name: &str) -> bool {
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || "_-.".contains(c))
}

fn strip_dashes(word: &str) -> &str {
    word.trim_start_matches('-')
}

/// The furniture that was peeled off each end of `word` to leave `core`.
fn split_furniture<'a>(word: &'a str, core: &str) -> (&'a str, &'a str) {
    match word.find(core) {
        // `core` is a trimmed slice of `word`, so it is always found; the
        // fallback keeps this total rather than panicking on a value that is
        // nothing but furniture.
        Some(at) if !core.is_empty() => (&word[..at], &word[at + core.len()..]),
        _ => ("", ""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real-shaped GitHub token. **Not a real one** — the repository is
    /// public permanently, so every fixture here is a value with the right
    /// *shape* and no account behind it.
    const TOKEN: &str = "ghp_A1b2C3d4E5f6G7h8I9j0K1l2M3n4O5p6Q7r8";
    /// A long opaque mixed-class run — what a generated credential looks like
    /// with the vendor prefix taken off.
    const OPAQUE: &str = "Zk3Lq9Xm2Pv8Rt5Yw1Nb6Hd4Jf7Gs0Ac";

    /// **The environment shape.** The name survives because the name is the
    /// diagnostic; the value does not.
    #[test]
    fn an_environment_assignment_keeps_its_name_and_loses_its_value() {
        let scrubbed = scrub(&format!("GITHUB_TOKEN={TOKEN} HOME=/scratch/home"));
        assert_eq!(
            scrubbed,
            format!("GITHUB_TOKEN={MASK} HOME=/scratch/home"),
            "the name is the diagnostic and the value is the secret"
        );
    }

    /// **The key alone is enough**, even when the value looks like nothing.
    /// Guild's own asymmetry: a false positive costs a word.
    #[test]
    fn a_credential_shaped_name_redacts_however_ordinary_the_value_looks() {
        assert_eq!(
            scrub("ANTHROPIC_API_KEY=hunter2"),
            format!("ANTHROPIC_API_KEY={MASK}")
        );
        assert_eq!(scrub("password=letmein"), format!("password={MASK}"));
    }

    /// **The command-line shape**, in both spellings, which is how a token
    /// reaches the argv of a run the ring buffer is about to write down.
    #[test]
    fn a_token_on_a_command_line_is_redacted_in_both_spellings() {
        for line in [
            format!("armada manifest check --token {TOKEN}"),
            format!("armada manifest check --token={TOKEN}"),
        ] {
            let scrubbed = scrub(&line);
            assert!(!scrubbed.contains("ghp_"), "{scrubbed}");
            assert!(
                scrubbed.contains("--token"),
                "the flag survives: {scrubbed}"
            );
            assert!(scrubbed.contains(MASK), "{scrubbed}");
        }
    }

    /// **The captured-output shape**: no key at all, and the value has to carry
    /// itself. This is the one that matters most, because output is the
    /// attachment nobody curated.
    #[test]
    fn a_bare_credential_in_captured_output_is_redacted_with_its_quotes_kept() {
        let scrubbed = scrub(&format!("  \"authorization\": \"Bearer {TOKEN}\","));
        assert!(!scrubbed.contains("ghp_"), "{scrubbed}");
        assert!(scrubbed.contains("\"authorization\""), "{scrubbed}");
        assert!(scrubbed.contains("Bearer"), "{scrubbed}");
        assert!(
            scrubbed.starts_with("  "),
            "indentation survives: {scrubbed}"
        );
        assert!(scrubbed.ends_with(","), "{scrubbed}");
    }

    /// A long opaque run with no vendor prefix — the judgement call, and the
    /// direction it is made in.
    #[test]
    fn a_long_opaque_run_goes_even_without_a_recognised_prefix() {
        assert!(!scrub(&format!("session {OPAQUE} started")).contains(OPAQUE));
    }

    /// **The false-positive budget is real and it has a floor.** A report is
    /// prose, and a redactor that ate ordinary words would make every
    /// attachment useless — which is the failure mode that ends with the guard
    /// being turned off.
    #[test]
    fn ordinary_prose_paths_and_versions_survive_intact() {
        for line in [
            "the dry-run said CREATED worktree and created nothing",
            "the token was wrong and the password prompt never appeared",
            "~/.armada/failures.jsonl is not readable",
            "armada 0.1.0 on darwin arm64, with claude 2.0.14",
            "OPEN  a1b2c3d4  armada_bug, the worktree was not there",
            "armada fleet spawn --dry-run -C ~/code/api",
        ] {
            assert_eq!(scrub(line), line, "prose must survive");
        }
    }

    /// **A noun in a sentence never arms the next word.** Only something typed
    /// as a flag does, which is what keeps `"the token was wrong"` readable.
    #[test]
    fn a_secret_word_in_prose_does_not_eat_the_word_after_it() {
        assert_eq!(scrub("token abc"), "token abc");
        assert_eq!(scrub("--token abc"), format!("--token {MASK}"));
    }

    /// Structure survives: line breaks, indentation and the shape of a JSON
    /// payload, because the attachment exists for an agent to read.
    #[test]
    fn line_structure_and_indentation_are_preserved_exactly() {
        let payload = format!("{{\n  \"verb\": \"doctor\",\n  \"key\": \"{TOKEN}\"\n}}\n");
        let scrubbed = scrub(&payload);
        assert_eq!(scrubbed.lines().count(), payload.lines().count());
        assert!(scrubbed.ends_with('\n'), "the trailing newline survives");
        assert!(scrubbed.contains("  \"verb\": \"doctor\","), "{scrubbed}");
        assert!(!scrubbed.contains("ghp_"), "{scrubbed}");
    }

    /// A PEM block's opening line is a registered prefix, and it arrives with
    /// spaces in it — so it is caught by the words around it rather than as one
    /// token. What must not happen is the key material surviving.
    #[test]
    fn private_key_material_does_not_survive() {
        let body = "MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQ";
        assert!(!scrub(&format!("-----BEGIN PRIVATE KEY-----\n{body}\n")).contains(body));
    }

    /// **Scrubbing twice is scrubbing once.** The ring buffer is redacted on the
    /// way in and again when a report attaches it, and a mask that accreted
    /// brackets on each pass would teach a reader to distrust it.
    #[test]
    fn scrubbing_an_already_scrubbed_line_changes_nothing() {
        for line in [
            format!("armada manifest check --token {TOKEN}"),
            format!("GITHUB_TOKEN={TOKEN}"),
            format!("  \"key\": \"{TOKEN}\","),
        ] {
            let once = scrub(&line);
            assert_eq!(scrub(&once), once, "not idempotent: {once}");
            assert!(!once.contains("[["), "{once}");
        }
    }

    /// Empty input, a lone `=`, and a value that is nothing but furniture — the
    /// shapes that would panic a naive tokeniser.
    #[test]
    fn degenerate_input_is_returned_rather_than_panicking() {
        assert_eq!(scrub(""), "");
        assert_eq!(scrub("="), "=");
        assert_eq!(scrub("\"\" () ---"), "\"\" () ---");
        assert_eq!(scrub("   \n\n  "), "   \n\n  ");
    }
}
