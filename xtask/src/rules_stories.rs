//! The Storybook half of the gate: one rule, reading every story against the
//! directory that holds it.
//!
//! `components.toml` was a hand-maintained list of what had been built, and it
//! drifted both ways at once — three rows naming a component with no story,
//! twelve stories no row named. Storybook replaced it because a story imports
//! the component the app imports and so cannot disagree with the app. But
//! nothing was checking Storybook, so deleting the registry moved the authority
//! and left the guard behind.
//!
//! The title-to-path transform is stated in
//! `.claude/skills/armada-components/SKILL.md`. It is what the design side
//! reads to find a component, and this is the only thing that checks it holds.
//!
//! **No TS parser, and the gate keeps no dependencies** — so this reads the one
//! shape a story file has: a `const meta` object with a quoted `title`.

use std::fs;
use std::path::Path;

use crate::{files_with_ext, Report};

/// Where the component library lives. Storybook's own glob is
/// `../src/**/*.stories.tsx` from `.storybook/`, which is this directory.
const ROOT: &str = "packages/components/src";

/// Where the transform and the directory layout are written down.
const SKILL: &str = ".claude/skills/armada-components/SKILL.md";

/// Every story's title is its path, and every component has a story.
///
/// Three things, and each was a way the deleted registry drifted:
///
/// - **A title maps to its directory under the transform.** That is what makes
///   a name resolve to a file with no lookup table, which is the whole reason
///   Storybook can be the registry.
/// - **A component directory has a story**, or the registry does not know it
///   exists.
/// - **A story has a component beside it**, or nothing outside Storybook can
///   render what the registry claims — the exact state three screens were in
///   before they were lifted into components taking props.
pub fn every_story_names_its_own_path(root: &Path) -> Report {
    let mut report = Report::new("every story's title is its path, and every component has one");

    let src = root.join(ROOT);
    if !src.is_dir() {
        report.fail(format!(
            "{ROOT} — the component library the stories live in"
        ));
        return report;
    }

    for group in dirs_in(&src) {
        for component in dirs_in(&src.join(&group)) {
            check(&src, &group, &component, &mut report);
        }
    }
    stories_are_in_a_component_directory(root, &src, &mut report);
    report
}

/// One component directory: its story, its component file, and whether the
/// title in the first agrees with the name of the directory.
fn check(src: &Path, group: &str, component: &str, report: &mut Report) {
    let on_disk = src.join(group).join(component);
    let dir = format!("{ROOT}/{group}/{component}");
    let story = format!("{dir}/{component}.stories.tsx");

    if !on_disk.join(format!("{component}.tsx")).is_file() {
        report.fail(format!(
            "{dir}/ — a directory with no `{component}.tsx`. \
             Storybook says what exists, so a story with no component behind it \
             claims something nothing outside Storybook can render. {SKILL}"
        ));
    }

    let Ok(text) = fs::read_to_string(on_disk.join(format!("{component}.stories.tsx"))) else {
        report.fail(format!(
            "{dir}/ — a component with no `{component}.stories.tsx`. \
             A component the registry does not draw is one nobody can see drift. {SKILL}"
        ));
        return;
    };

    let Some((title, line)) = meta_title(&text) else {
        report.fail(format!(
            "{story} — no quoted `title` in its `const meta`. \
             The title is where the component's real name lives. {SKILL}"
        ));
        return;
    };

    let Some((stated_group, name)) = title.split_once('/') else {
        report.fail(format!(
            "{story}:{line} — title \"{title}\" names no group. \
             A title is `<Group>/<Name>`, and `{group}/` is the group here. {SKILL}"
        ));
        return;
    };

    if name.contains('/') {
        report.fail(format!(
            "{story}:{line} — title \"{title}\" nests below its group. \
             A title is `<Group>/<Name>`, because the path is two levels. {SKILL}"
        ));
        return;
    }

    if !stated_group.eq_ignore_ascii_case(group) {
        report.fail(format!(
            "{story}:{line} — title \"{title}\" is in group `{stated_group}`, \
             and the directory is `{group}/`. {SKILL}"
        ));
    }

    let want = pascal(name);
    if want != component {
        report.fail(format!(
            "{story}:{line} — title \"{title}\" is `{want}` under the transform, \
             and the directory is `{component}/`. Rename one to the other. {SKILL}"
        ));
    }
}

/// A story anywhere but `<group>/<Name>/<Name>.stories.tsx` has no directory to
/// be checked against, so it is invisible to the rule above while still being
/// picked up by Storybook's glob.
fn stories_are_in_a_component_directory(root: &Path, src: &Path, report: &mut Report) {
    for path in files_with_ext(root, src, &["tsx"]) {
        let Some(rest) = path.strip_prefix(&format!("{ROOT}/")) else {
            continue;
        };
        let parts: Vec<&str> = rest.split('/').collect();
        let Some(stem) = parts.last().and_then(|n| n.strip_suffix(".stories.tsx")) else {
            continue;
        };
        if parts.len() == 3 && parts[1] == stem {
            continue;
        }
        report.fail(format!(
            "{path} — a story outside `<group>/{stem}/{stem}.stories.tsx`. \
             Storybook loads it and no directory checks it. {SKILL}"
        ));
    }
}

/// The `title` of a story file's `const meta`, and the line it is on.
///
/// Bounded to the `const meta` block rather than taking the first match,
/// because a story's args carry `title` too: `TheShell` has one at the same
/// indentation as the meta's, one line under a fixture.
fn meta_title(text: &str) -> Option<(String, usize)> {
    let mut inside = false;
    for (n, raw) in text.lines().enumerate() {
        if !inside {
            inside = raw.starts_with("const meta") && raw.trim_end().ends_with('{');
            continue;
        }
        if raw.starts_with('}') {
            return None;
        }
        if let Some(rest) = raw.trim_start().strip_prefix("title:") {
            return quoted(rest).map(|t| (t, n + 1));
        }
    }
    None
}

/// The first double-quoted string in `s`. No escape handling — a title
/// carrying a quote is not a thing, and the mismatch it would produce names
/// both sides rather than passing silently.
fn quoted(s: &str) -> Option<String> {
    let (_, rest) = s.split_once('"')?;
    let (inner, _) = rest.split_once('"')?;
    Some(inner.to_string())
}

/// A story's human name as the directory that holds it: split on every
/// non-alphanumeric, capitalise each word, concatenate. `Job row (stacked)` is
/// `JobRowStacked`; `A failed job — a dead end, read as one` is
/// `AFailedJobADeadEndReadAsOne`.
///
/// **Only a word's first letter changes.** Lowercasing the rest would make
/// `StatusBar` into `Statusbar`; leaving it makes `kbd` into `Kbd`. Those two
/// titles are what pin the transform down.
///
/// **ASCII, because the output has to be a directory name.** A non-ASCII letter
/// splits the word rather than surviving into it, so a title carrying one fails
/// and names both sides — which is the point. Silently dropping it is how a
/// rule widens itself.
fn pascal(name: &str) -> String {
    let mut out = String::new();
    for word in name.split(|c: char| !c.is_ascii_alphanumeric()) {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_ascii_uppercase());
            out.push_str(chars.as_str());
        }
    }
    out
}

/// Every directory directly under `dir`, sorted. A missing directory yields
/// nothing — absence is reported by the rule, not by a panic.
fn dirs_in(dir: &Path) -> Vec<String> {
    let mut found = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            if !entry.path().is_dir() {
                continue;
            }
            if let Some(name) = entry.file_name().to_str() {
                found.push(name.to_string());
            }
        }
    }
    found.sort();
    found
}

#[cfg(test)]
mod tests;
