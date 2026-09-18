//! Assembling the command a narrowed Check runs, from what the Manifest
//! declared and what the worktree changed.
//!
//! **Here rather than in `fleet`, because this file and [`run`](crate::run)
//! have to agree about quoting.** A `run` string is split into a program and
//! its arguments by a quote-aware splitter with no shell behind it, so a path
//! holding a space is one argument only if this puts quotes where that splitter
//! expects to take them off. Two crates deciding that is two crates that can
//! disagree, and the way they would disagree is a Check reporting a failure
//! about a file nobody named.
//!
//! **It knows nothing about Cargo, and that is the design.** `-p ipc` from
//! `crates/ipc/src/lib.rs` is two facts: that packages live under `crates`,
//! which the Manifest writes as `under`, and that a package is named to this
//! command as `-p <name>`, which the Manifest writes as `each`. A runner
//! holding either would be wrong about the first `pnpm` repository it met.

use core_model::Narrowing;

/// What a Check's narrowing came to for one change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Narrowed {
    /// Run this instead of the Check's own command.
    To(String),
    /// Nothing this change touched feeds the narrowing, so the Check has
    /// nothing to say about the work. **Not a pass and not a failure** — the
    /// caller records it as a Check that was not run.
    Nothing,
    /// Run the Check's own command, whole. Both the ordinary case — a Check
    /// that declares no `narrow` — and the one path this cannot spell as an
    /// argument, which is the safe direction: whole is never less than narrow.
    Whole,
}

/// The command a narrowed run of this Check should use.
///
/// `changed` is the worktree's own diff, read by Fleet rather than named by the
/// Drone. Values come back **sorted and deduplicated**: `-p ipc -p ipc` costs a
/// second package resolution and buys nothing, and a report that reads
/// differently on two identical runs is a report a Drone cannot compare.
///
/// A change that derives only to excluded values answers [`Narrowed::Nothing`],
/// which is the truth: the Check has nothing to say about work the whole run
/// would not have measured either.
pub fn narrowed(narrowing: Option<&Narrowing>, changed: &[String]) -> Narrowed {
    let Some(narrowing) = narrowing else {
        return Narrowed::Whole;
    };
    let mut values: Vec<&str> = Vec::new();
    for path in changed {
        if !narrowing
            .from()
            .is_none_or(|from| from.matches_any(std::slice::from_ref(path)))
        {
            continue;
        }
        let Some(value) = value(narrowing, path) else {
            continue;
        };
        // **After the derivation, because it names values and not paths.** A
        // whole run that already excludes something excludes it for a reason,
        // and a narrowed run that pulled it back in would show a Drone a bar
        // the gate deliberately does not apply to it.
        if narrowing.except().iter().any(|dropped| dropped == value) {
            continue;
        }
        // A path this cannot spell as one argument abandons the whole
        // narrowing rather than being dropped from the list. Dropping it would
        // run a Check over a subset the Drone was never told was a subset,
        // which is the one way this call could lie about what it measured.
        if !spellable(value) {
            return Narrowed::Whole;
        }
        values.push(value);
    }
    values.sort_unstable();
    values.dedup();
    if values.is_empty() {
        return Narrowed::Nothing;
    }
    let mut command = String::from(narrowing.run());
    for value in values {
        command.push(' ');
        command.push_str(&narrowing.each().replace("{}", &quoted(value)));
    }
    Narrowed::To(command)
}

/// What one changed path contributes, or [`None`] where it contributes nothing.
///
/// `under` names a directory whose child is the value — `crates` turns
/// `crates/ipc/src/lib.rs` into `ipc` — and a path outside that directory, or
/// one that is the directory itself with no child, yields nothing rather than a
/// value nobody meant.
fn value<'p>(narrowing: &Narrowing, path: &'p str) -> Option<&'p str> {
    let Some(under) = narrowing.under() else {
        return Some(path);
    };
    let under = under.trim_end_matches('/');
    let rest = path.strip_prefix(under)?.strip_prefix('/')?;
    match rest.split('/').next() {
        Some(child) if !child.is_empty() => Some(child),
        _ => None,
    }
}

/// Whether a value can be handed to [`run`](crate::run)'s splitter as one
/// argument. A quote of either kind cannot: the splitter takes quotes off and
/// has no escape, so there is no spelling that survives the round trip.
fn spellable(value: &str) -> bool {
    !value.contains('\'') && !value.contains('"')
}

/// A value as one argument. Quoted only where it holds whitespace, so the
/// command a report prints is the command a person would have typed.
fn quoted(value: &str) -> String {
    match value.contains(char::is_whitespace) {
        true => format!("\"{value}\""),
        false => value.to_string(),
    }
}

/// The command that runs one test by name, or `None` where the name cannot be
/// one argument. The name is a Drone's, so it gets the guard a narrowed value
/// gets and no other. #999.
pub fn one_test(run: &str, test: &str) -> Option<String> {
    let test = test.trim();
    if test.is_empty() || !spellable(test) {
        return None;
    }
    Some(run.replace("{}", &quoted(test)))
}

/// The command a runner's `run_changed` shape makes for these files.
///
/// `None` where the template has nowhere to put them, where a path cannot be
/// one argument, or where no path was given — all three are a runner that has
/// nothing narrower to say about this change, and the caller runs the Check
/// whole rather than running something that names no file.
///
/// **`{pkg}` is the Check's, `{files}` is the Drone's.** The first is a fact
/// about where this Check runs and is substituted whether the Check declared
/// one or not; a template naming it against a Check that declares none has
/// nothing to put there, and that is the `None` above rather than an empty
/// argument in the middle of a command.
pub fn run_changed(template: &str, pkg: Option<&str>, files: &[String]) -> Option<String> {
    if !template.contains("{files}") {
        return None;
    }
    let mut named: Vec<&str> = files
        .iter()
        .map(|path| path.trim())
        .filter(|path| !path.is_empty())
        .collect();
    named.sort_unstable();
    named.dedup();
    if named.is_empty() || !named.iter().all(|path| spellable(path)) {
        return None;
    }
    let command = match (template.contains("{pkg}"), pkg) {
        (true, None) => return None,
        (true, Some(pkg)) if !spellable(pkg) => {
            let _ = pkg;
            return None;
        }
        (true, Some(pkg)) => template.replace("{pkg}", &quoted(pkg)),
        (false, _) => template.to_string(),
    };
    let spelled = named
        .iter()
        .map(|path| quoted(path))
        .collect::<Vec<String>>()
        .join(" ");
    Some(command.replace("{files}", &spelled))
}

#[cfg(test)]
mod tests {
    use super::run_changed;

    const VITEST: &str = "pnpm --dir {pkg} exec vitest related {files} --run";

    fn paths(of: &[&str]) -> Vec<String> {
        of.iter().map(|path| path.to_string()).collect()
    }

    #[test]
    fn the_package_and_the_files_are_both_put_in() {
        assert_eq!(
            run_changed(VITEST, Some("packages/screens"), &paths(&["src/a.ts"])),
            Some("pnpm --dir packages/screens exec vitest related src/a.ts --run".to_string())
        );
    }

    /// Sorted and deduplicated for `narrowed`'s reason: a report that reads
    /// differently on two identical runs is one nobody can compare.
    #[test]
    fn the_files_come_back_sorted_and_deduplicated() {
        assert_eq!(
            run_changed(
                VITEST,
                Some("p"),
                &paths(&["src/b.ts", "src/a.ts", "src/b.ts"])
            ),
            Some("pnpm --dir p exec vitest related src/a.ts src/b.ts --run".to_string())
        );
    }

    #[test]
    fn a_path_holding_a_space_is_one_argument() {
        let made =
            run_changed(VITEST, Some("p"), &paths(&["src/two words.ts"])).expect("it narrows");
        assert!(made.contains("\"src/two words.ts\""), "{made}");
    }

    /// Every one of these is the same answer to the caller — the Check runs
    /// whole — and none of them is a command naming nothing.
    #[test]
    fn nothing_to_say_is_none_and_never_a_command_without_files() {
        assert_eq!(run_changed(VITEST, Some("p"), &[]), None, "no files");
        assert_eq!(
            run_changed(VITEST, None, &paths(&["a.ts"])),
            None,
            "no package for a template that names one"
        );
        assert_eq!(
            run_changed("pnpm test", Some("p"), &paths(&["a.ts"])),
            None,
            "a template with nowhere to put them"
        );
        assert_eq!(
            run_changed(VITEST, Some("p"), &paths(&["src/it's.ts"])),
            None,
            "a path the splitter cannot be handed as one argument"
        );
    }

    /// A runner rooted at the repository declares no `{pkg}`, and a Check
    /// driven by it needs none.
    #[test]
    fn a_template_naming_no_package_needs_none() {
        assert_eq!(
            run_changed("pytest {files}", None, &paths(&["t/a.py"])),
            Some("pytest t/a.py".to_string())
        );
    }
}
