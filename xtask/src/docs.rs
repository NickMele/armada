//! Shared loading of the documentation corpus.
//!
//! Every check works from the same set of files and the same fenced-block
//! extraction, so both live here rather than being reimplemented per check —
//! two extractors that disagree would make a green lint meaningless.

use std::path::{Path, PathBuf};

/// The corpus, by the short name a cross-reference uses to attribute a `§`.
///
/// The first six are ordered as `ARCHITECTURE.md` §2.8 orders precedence, so
/// reports read in the order the documents actually rank. `docs/reserved/*`
/// follows: one reserved design per file (docs/PLAN.md §15.3), none of which
/// define a `§`-numbered heading of their own, so their order does not matter
/// to xref resolution — only that they are read at all.
pub const FILES: &[(&str, &str)] = &[
    ("traps.md", "docs/traps.md"),
    ("ARCHITECTURE.md", "docs/ARCHITECTURE.md"),
    ("PLAN.md", "docs/PLAN.md"),
    ("PHASES.md", "docs/PHASES.md"),
    ("AGENTS.md", "AGENTS.md"),
    ("README.md", "README.md"),
    ("reserved/README.md", "docs/reserved/README.md"),
    (
        "001-raised-items-need-identity.md",
        "docs/reserved/001-raised-items-need-identity.md",
    ),
    ("002-tasks.md", "docs/reserved/002-tasks.md"),
    (
        "003-bridge-command-centre.md",
        "docs/reserved/003-bridge-command-centre.md",
    ),
    (
        "004-guild-inventory.md",
        "docs/reserved/004-guild-inventory.md",
    ),
    (
        "005-inbox-label-not-identity.md",
        "docs/reserved/005-inbox-label-not-identity.md",
    ),
    (
        "006-guild-has-no-way-to-learn.md",
        "docs/reserved/006-guild-has-no-way-to-learn.md",
    ),
    (
        "007-scanner-should-propose.md",
        "docs/reserved/007-scanner-should-propose.md",
    ),
    (
        "008-armada-injects-its-own-skills.md",
        "docs/reserved/008-armada-injects-its-own-skills.md",
    ),
    (
        "009-smaller-things-raised-in-use.md",
        "docs/reserved/009-smaller-things-raised-in-use.md",
    ),
    (
        "010-armada-records-its-own-failures.md",
        "docs/reserved/010-armada-records-its-own-failures.md",
    ),
];

pub struct Doc {
    /// Short name — `PLAN.md`. What a `§` reference attributes against.
    pub name: &'static str,
    /// Repo-relative path — `docs/PLAN.md`. What a report cites.
    pub rel: &'static str,
    pub text: String,
}

impl Doc {
    pub fn lines(&self) -> std::str::Lines<'_> {
        self.text.lines()
    }
}

pub fn load(root: &Path) -> Result<Vec<Doc>, String> {
    FILES
        .iter()
        .map(|(name, rel)| {
            let path: PathBuf = root.join(rel);
            std::fs::read_to_string(&path)
                .map(|text| Doc { name, rel, text })
                .map_err(|e| format!("{rel}: {e}"))
        })
        .collect()
}

/// One fenced code block.
pub struct Block {
    /// Info string after the fence, lowercased. Empty when the fence had none.
    pub lang: String,
    /// 1-indexed line of the block's first *body* line, for citation.
    pub start: usize,
    pub body: String,
}

/// Extract fenced blocks, including blockquoted ones.
///
/// A fence inside a `>` blockquote has its marker stripped from every body
/// line — the plan uses blockquotes for asides that still contain real YAML,
/// and leaving the `>` in place makes every one of them a parse failure.
///
/// A closing fence must match the opening run length and carry no info string,
/// which is what lets a ```` ```` ```` block legally contain ``` ```` ```.
///
/// A block preceded by `<!-- doclint: skip ... -->` is dropped. Some blocks in
/// this corpus are deliberately not parseable — a fragment shown without its
/// envelope, a yes/no pair that repeats a key, `<placeholders>` in a shell
/// line. The marker makes that a decision with a reason attached rather than a
/// finding everyone learns to scroll past, which is how a lint stops working.
pub fn blocks(text: &str) -> Vec<Block> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let Some(open) = parse_fence(lines[i]) else {
            i += 1;
            continue;
        };
        let skipped = i > 0 && lines[i - 1].trim_start().starts_with("<!-- doclint: skip");
        let mut body = Vec::new();
        let mut j = i + 1;
        while j < lines.len() {
            match parse_fence(lines[j]) {
                Some(close) if close.ticks == open.ticks && close.lang.is_empty() => break,
                _ => {
                    body.push(strip_quote(lines[j], &open.prefix));
                    j += 1;
                }
            }
        }
        if !skipped {
            out.push(Block {
                lang: open.lang.to_ascii_lowercase(),
                start: i + 2, // 1-indexed, body starts one line past the fence
                body: body.join("\n"),
            });
        }
        i = j + 1;
    }
    out
}

struct Fence {
    prefix: String,
    ticks: usize,
    lang: String,
}

fn parse_fence(line: &str) -> Option<Fence> {
    // Leading whitespace and blockquote markers, then a run of >= 3 backticks,
    // then an optional info string and nothing else.
    let mut idx = 0;
    let bytes = line.as_bytes();
    while idx < bytes.len() && (bytes[idx] == b' ' || bytes[idx] == b'\t' || bytes[idx] == b'>') {
        idx += 1;
    }
    let prefix = line[..idx].to_string();
    let rest = &line[idx..];
    let ticks = rest.bytes().take_while(|b| *b == b'`').count();
    if ticks < 3 {
        return None;
    }
    let lang = rest[ticks..].trim();
    // An info string with a space is a fence with attributes, not a language.
    if lang.contains(char::is_whitespace) {
        return None;
    }
    Some(Fence {
        prefix,
        ticks,
        lang: lang.to_string(),
    })
}

fn strip_quote(line: &str, prefix: &str) -> String {
    if !prefix.contains('>') {
        return line.to_string();
    }
    // Drop the same leading whitespace-and-`>` run the opening fence had, plus
    // the single space that conventionally follows it.
    let mut idx = 0;
    let bytes = line.as_bytes();
    while idx < bytes.len() && (bytes[idx] == b' ' || bytes[idx] == b'\t' || bytes[idx] == b'>') {
        idx += 1;
    }
    line[idx..].to_string()
}

/// A finding. Every check emits these; `main` decides the exit code.
pub struct Finding {
    pub file: String,
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for Finding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}  {}", self.file, self.line, self.message)
    }
}
