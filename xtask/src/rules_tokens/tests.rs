//! The rule's own negative tests, and the emitter assertion behind them.
//!
//! A rule that has never been shown to fail asserts nothing. This one answers a
//! defect every other check passed for eight months, so the first thing proved
//! here is the exact file that shipped: a `--breakpoint-*` aliasing a token.

use super::*;
use crate::tokens::Token;
use crate::tokens_emit::theme_css;
use crate::Finding;

/// A tree with one stylesheet in it, under a temporary root.
fn findings(name: &str, rel: &str, css: &str) -> Vec<String> {
    let root = std::env::temp_dir().join(format!("armada-tokens-{name}"));
    let _ = fs::remove_dir_all(&root);
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("a parent")).expect("a directory");
    fs::write(&path, css).expect("a stylesheet");
    // The rule fails on a missing theme too, and that is not what these prove.
    let theme = root.join(THEME);
    fs::create_dir_all(theme.parent().expect("a parent")).expect("a directory");
    if !theme.is_file() {
        fs::write(&theme, "@theme {\n}\n").expect("a theme");
    }
    let report = no_media_query_resolves_through_a_custom_property(&root);
    let _ = fs::remove_dir_all(&root);
    report
        .findings
        .iter()
        .map(|f| match f {
            Finding::Fail(what) | Finding::Warn(what) => what.clone(),
        })
        .collect()
}

/// One token, as the reader would have produced it.
fn token(name: &str, value: &str) -> Token {
    Token {
        name: name.to_string(),
        value: value.to_string(),
        file: "spacing.css".to_string(),
        alias_of: None,
        note: None,
    }
}

#[test]
fn a_breakpoint_that_aliases_a_token_fails() {
    let found = findings(
        "aliased",
        "packages/tokens/tokens.theme.css",
        "@theme inline {\n  --breakpoint-narrow: var(--layout-breakpoint-narrow);\n}\n",
    );
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("--breakpoint-narrow"), "{found:?}");
}

#[test]
fn a_hand_written_media_query_reading_a_token_fails() {
    let found = findings(
        "handwritten",
        "packages/components/src/Thing.css",
        "@media (max-width: var(--window-floor)) {\n  .thing { display: none; }\n}\n",
    );
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("Thing.css:1"), "{found:?}");
}

/// The false positive the rule would otherwise report against the file it is
/// about: `tokens.theme.css` explains this defect in prose, `@media` and
/// `var()` and all.
#[test]
fn prose_about_the_defect_is_not_the_defect() {
    let found = findings(
        "prose",
        "packages/components/src/Thing.css",
        "/* Tailwind writes it into @media (width >= …), where var() is not\n   \
         legal at all. */\n.thing { display: none; }\n",
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_literal_breakpoint_and_a_literal_media_query_pass() {
    let found = findings(
        "literal",
        "packages/tokens/tokens.theme.css",
        "@theme {\n  --breakpoint-*: initial;\n  --breakpoint-narrow: 880px;\n}\n\
         @media (width >= 880px) {\n  .thing { display: none; }\n}\n",
    );
    assert!(found.is_empty(), "{found:?}");
}

/// The emitter half. `Slot::Breakpoint` is what makes the rule above unable to
/// fire against a generated file, and this is the assertion that it does.
#[test]
fn the_emitter_writes_a_breakpoint_as_a_literal_in_a_plain_theme() {
    let css = theme_css(&[
        token("--layout-breakpoint", "1100px"),
        token("--layout-breakpoint-narrow", "880px"),
    ])
    .expect("a theme");

    assert!(css.contains("--breakpoint-wide: 1100px;"), "{css}");
    assert!(css.contains("--breakpoint-narrow: 880px;"), "{css}");
    // Not in the `inline` block, which is the whole difference.
    let plain = css.rfind("@theme {").expect("a plain @theme");
    let inline = css.find("@theme inline {").expect("an inline @theme");
    assert!(plain > inline, "{css}");
    assert!(css[plain..].contains("--breakpoint-narrow"), "{css}");
}

/// The emitter refuses the shape that shipped, rather than emitting it for the
/// rule to catch later. A generated file nobody can hand-edit is only as safe
/// as the generator.
#[test]
fn the_emitter_refuses_a_breakpoint_that_aliases_a_token() {
    let err = theme_css(&[token("--layout-breakpoint", "var(--something-else)")])
        .expect_err("an aliased breakpoint");
    assert!(err.contains("media feature"), "{err}");
}
