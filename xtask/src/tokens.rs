//! The design token pipeline: `packages/tokens/src/*.css` in, three checked-in
//! outputs out, and a check that fails when they drift apart.
//!
//! **The CSS is the authority on values.** Notion is the authority on the
//! decision and the reasoning; a row that disagrees with the CSS is stale. So
//! this generator reads the CSS and never the other way round — seeded from
//! Notion instead, every later regeneration would diff against the wrong thing.
//!
//! Two rules make the pipeline safe to leave alone:
//!
//! - **Cascade order is read from `styles.css`**, not hard-coded here. The
//!   import list *is* the cascade, and it is the only place it is declared.
//! - **An unclassified file or token fails.** Adding a source without saying
//!   what it is, or a token whose name matches no entry in [`THEME`], stops the
//!   check rather than being guessed at. Closure by collection, the same shape
//!   `verify-error-codes` and the vendor-literal rule use.
//!
//! `tokens.css` is a verbatim concatenation rather than a re-emission. Several
//! tokens carry the argument for a decision made one way and then reversed —
//! `--step-failed` and `--focus-ring` in particular — and those comments are
//! the only record of why a value is what it is. Concatenating means they
//! survive by construction instead of by care.

use std::fs;
use std::path::Path;

/// What a file listed in `styles.css` is.
enum Source {
    /// Declares custom properties. Goes through the generator.
    Tokens,
    /// Consumes custom properties and declares none — a stylesheet, not a
    /// token file. `base.css` is the only one, and the reason it is named here
    /// rather than skipped by a rule is that a rule would also skip the next
    /// file somebody adds by mistake.
    Stylesheet,
}

/// Every file `styles.css` may import, and what it is. An import not on this
/// list fails the check.
const SOURCES: &[(&str, Source)] = &[
    ("fonts.css", Source::Tokens),
    ("colors.css", Source::Tokens),
    ("status.css", Source::Tokens),
    ("typography.css", Source::Tokens),
    ("spacing.css", Source::Tokens),
    ("motion.css", Source::Tokens),
    ("semantic.css", Source::Tokens),
    ("base.css", Source::Stylesheet),
];

/// Where a token lands in the Tailwind theme.
///
/// The theme key is the token name, so a class name and a token name are the
/// same string and a mismatch is unspellable. `CssOnly` is for values that are
/// not utilities — a composite shorthand, or a constant a layout reads.
pub enum Slot {
    /// `theme.<group>.<name>`, value `var(--token)`, the key derived from the
    /// token name by stripping its prefix.
    Group(&'static str),
    /// `theme.<group>.<key>`, where the derived key would read badly
    /// (`shadow-shadow-overlay`) or the token means something the prefix does
    /// not say (a floor is a `minHeight`, not a `height`).
    Named(&'static str, &'static str),
    /// One half of a `fontSize` tuple, paired with its `--leading-*`.
    FontSize,
    /// The other half. Consumed by its `--text-*`, never emitted alone.
    Leading,
    /// Reachable from CSS only, with the reason it is not a utility.
    CssOnly(&'static str),
}

/// Name-to-slot, longest match first. **An exact name beats a prefix**, which
/// is what resolves the one collision in the set: `--text-sm` is a font size
/// and `--text-body` is a colour, and no prefix rule can tell them apart.
pub const THEME: &[(&str, Slot)] = &[
    // Exact names, checked first.
    ("--text-body", Slot::Group("colors.text")),
    ("--text-label", Slot::Group("colors.text")),
    ("--text-meta", Slot::Group("colors.text")),
    ("--text-on-accent", Slot::Group("colors.text")),
    ("--accent", Slot::Named("colors.accent", "DEFAULT")),
    ("--ease", Slot::Named("transitionTimingFunction", "DEFAULT")),
    ("--tooltip-delay", Slot::Named("transitionDelay", "tooltip")),
    ("--gap-section", Slot::Named("spacing", "section")),
    ("--shadow-overlay", Slot::Named("boxShadow", "overlay")),
    ("--tracking-caps", Slot::Named("letterSpacing", "caps")),
    ("--palette-width", Slot::Named("maxWidth", "palette")),
    ("--palette-max-height", Slot::Named("maxHeight", "palette")),
    // The Job row is padding-driven: 84px is a floor a wrapping title grows
    // past, so it is a min-height and `h-row-stacked` must not exist.
    ("--h-row-stacked", Slot::Named("minHeight", "row-stacked")),
    ("--sidebar-default", Slot::Named("spacing", "sidebar")),
    ("--sidebar-min", Slot::Named("minWidth", "sidebar")),
    ("--sidebar-max", Slot::Named("maxWidth", "sidebar")),
    ("--sidebar-rail", Slot::Named("spacing", "rail")),
    // Composites and constants. Not classes, on purpose.
    ("--focus-ring", Slot::CssOnly("an outline shorthand, not a scale value")),
    ("--focus-ring-offset", Slot::CssOnly("paired with --focus-ring")),
    ("--row-focus-bar", Slot::CssOnly("a border shorthand, not a scale value")),
    ("--window-floor", Slot::CssOnly("the window's hard floor, read by main")),
    ("--layout-breakpoint", Slot::CssOnly("a media query bound, not a utility")),
    // Prefixes.
    ("--bg-", Slot::Group("colors.bg")),
    ("--fg-", Slot::Group("colors.fg")),
    ("--border-", Slot::Group("borderColor")),
    ("--accent-", Slot::Group("colors.accent")),
    ("--diff-", Slot::Group("colors.diff")),
    ("--status-", Slot::Group("colors.status")),
    ("--step-", Slot::Group("colors.step")),
    ("--verdict-", Slot::Group("colors.verdict")),
    ("--surface-", Slot::Group("colors.surface")),
    ("--edge-", Slot::Group("borderColor")),
    ("--space-", Slot::Group("spacing")),
    ("--h-", Slot::Group("height")),
    ("--pad-", Slot::Group("padding")),
    ("--radius-", Slot::Group("borderRadius")),
    ("--text-", Slot::FontSize),
    ("--leading-", Slot::Leading),
    ("--weight-", Slot::Group("fontWeight")),
    ("--font-", Slot::Group("fontFamily")),
    ("--duration-", Slot::Group("transitionDuration")),
];

/// One declared custom property, with the comment that explains it.
pub struct Token {
    pub name: String,
    pub value: String,
    pub file: String,
    /// The token this resolves to, when the value is a bare `var(--x)`.
    /// Recorded rather than flattened: an alias is the indirection, and
    /// resolving it at generation time throws away the thing it is for.
    pub alias_of: Option<String>,
    pub note: Option<String>,
}

/// Read `styles.css`, then every token file it names, in cascade order.
pub fn read(root: &Path) -> Result<(Vec<String>, Vec<Token>), String> {
    let src = root.join("packages/tokens/src");
    let styles = fs::read_to_string(src.join("styles.css"))
        .map_err(|_| "packages/tokens/src/styles.css — the cascade is not here".to_string())?;

    // The path an import carries is the design project's, not ours —
    // `tokens/colors.css` there, a sibling here. Only the file name and the
    // order matter, and taking the name means a refetch needs no hand edit.
    let mut order = Vec::new();
    for line in styles.lines() {
        let Some(rest) = line.trim().strip_prefix("@import \"") else { continue };
        let Some(path) = rest.split('"').next() else { continue };
        let Some(name) = path.rsplit('/').next() else { continue };
        order.push(name.to_string());
    }
    if order.is_empty() {
        return Err("styles.css names no imports — the cascade cannot be read".into());
    }

    let mut sources = Vec::new();
    let mut tokens = Vec::new();
    for name in &order {
        let Some((_, kind)) = SOURCES.iter().find(|(n, _)| n == name) else {
            return Err(format!(
                "{name} is imported by styles.css and classified nowhere — \
                 add it to SOURCES in xtask/src/tokens.rs saying what it is"
            ));
        };
        let text = fs::read_to_string(src.join(name))
            .map_err(|_| format!("packages/tokens/src/{name} — imported and not present"))?;
        if let Source::Tokens = kind {
            tokens.extend(parse(&text, name));
            sources.push(name.clone());
        } else if text.contains("--") && declares_a_property(&text) {
            return Err(format!(
                "{name} is classified as a stylesheet and declares a custom property — \
                 reclassify it or move the declaration"
            ));
        }
    }

    // `@import url(...)` must precede every rule in a stylesheet, so a font
    // import anywhere but the first file makes the concatenation invalid CSS.
    for name in sources.iter().skip(1) {
        let text = fs::read_to_string(src.join(name)).unwrap_or_default();
        if text.contains("@import url(") {
            return Err(format!(
                "{name} carries an @import url() and is not first in the cascade — \
                 the concatenated tokens.css would be invalid"
            ));
        }
    }
    Ok((sources, tokens))
}

fn declares_a_property(text: &str) -> bool {
    text.lines().any(|l| {
        let t = l.trim_start();
        t.starts_with("--") && t.contains(':')
    })
}

/// Pull `--name: value;` out of `:root` blocks, keeping the comment that
/// explains each one. Declarations inside `@media` are overrides rather than
/// declarations and are carried by the concatenation, not listed here.
///
/// **Comments are stripped before anything is read as code.** These files
/// argue with themselves in prose, and a comment line reasoning about
/// `--fg-subtle reaches 4.58:1` parses as a declaration to anything that looks
/// for `--name:` without knowing where the comment is. It did.
fn parse(text: &str, file: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut in_media = false;
    let mut in_comment = false;
    let mut pending: Option<String> = None;

    for raw in text.lines() {
        let (code, comment) = split_comment(raw, &mut in_comment);
        let line = code.trim();
        // The nearest comment above a declaration is the one that explains it,
        // so a later comment replaces an earlier unconsumed one.
        if let Some(text) = comment {
            pending = Some(text);
        }
        if line.starts_with("@media") {
            in_media = true;
        }

        let opens = line.matches('{').count();
        let closes = line.matches('}').count();

        // A file's header comment explains the file, not the first token in it.
        // Only the first block, though: `status.css` opens three `:root`s, and
        // the comment above the second and third is the argument for what
        // follows, not a header.
        if depth == 0 && opens > 0 && out.is_empty() {
            pending = None;
        }

        if depth > 0 && !in_media {
            for decl in line.split(';') {
                let decl = decl.trim();
                if !decl.starts_with("--") {
                    continue;
                }
                let Some((name, rest)) = decl.split_once(':') else { continue };
                let value = rest.trim().to_string();
                if value.is_empty() {
                    continue;
                }
                let alias_of = value
                    .strip_prefix("var(")
                    .and_then(|v| v.strip_suffix(')'))
                    .filter(|v| v.starts_with("--") && !v.contains(','))
                    .map(str::to_string);
                out.push(Token {
                    name: name.trim().to_string(),
                    value,
                    file: file.to_string(),
                    alias_of,
                    note: pending.take(),
                });
            }
        }

        depth = depth + opens - closes.min(depth + opens);
        if depth == 0 {
            in_media = false;
        }
    }
    out
}

/// Split one line into its code and its comment text, carrying `/* … */`
/// across lines. Only the line a comment **opens** on yields a note — a note
/// is a label, and the argument stays where it was written. Without that, the
/// last line of a nine-line comment becomes the token's explanation.
fn split_comment(raw: &str, in_comment: &mut bool) -> (String, Option<String>) {
    let mut code = String::new();
    let mut note: Option<String> = None;
    let continuing = *in_comment;
    let mut rest = raw;

    loop {
        if *in_comment {
            match rest.find("*/") {
                Some(end) => {
                    if !continuing && note.is_none() {
                        note = Some(first_sentence(&rest[..end]));
                    }
                    *in_comment = false;
                    rest = &rest[end + 2..];
                }
                None => {
                    if !continuing && note.is_none() {
                        note = Some(first_sentence(rest));
                    }
                    return (code, note.filter(|n| !n.is_empty()));
                }
            }
        } else {
            match rest.find("/*") {
                Some(start) => {
                    code.push_str(&rest[..start]);
                    *in_comment = true;
                    rest = &rest[start + 2..];
                }
                None => {
                    code.push_str(rest);
                    return (code, note.filter(|n| !n.is_empty()));
                }
            }
        }
    }
}

fn first_sentence(line: &str) -> String {
    line.trim_start_matches("/*")
        .trim_end_matches("*/")
        .trim()
        .trim_end_matches('.')
        .to_string()
}

/// The three outputs, as (path, contents).
pub fn outputs(root: &Path) -> Result<Vec<(String, String)>, String> {
    use crate::tokens_emit::{tailwind_theme, tokens_css, tokens_json};
    let (sources, tokens) = read(root)?;
    if tokens.is_empty() {
        return Err("no tokens parsed — the sources are present and declare nothing".into());
    }
    Ok(vec![
        ("packages/tokens/tokens.css".into(), tokens_css(root, &sources)),
        ("packages/tokens/tokens.json".into(), tokens_json(&tokens)),
        ("packages/tokens/tailwind.theme.js".into(), tailwind_theme(&tokens)?),
    ])
}
