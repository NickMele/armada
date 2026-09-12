//! Which words of a `run` line name something in the repository's build files,
//! for the tools this read knows.
//!
//! `pnpm` and `npm` name a script in a `package.json`; `cargo` names an alias
//! in `.cargo/config.toml` and packages in the workspace. Every other program
//! is recorded as not followed — **a tool this read has never heard of reads as
//! not followed, plainly, and never as clean.**
//!
//! # A tool's own subcommand is not a missing script
//!
//! `pnpm install` names no script, and neither does `cargo build`. So each
//! tool carries the subcommands it answers for itself, and a word on that list
//! that is not also a script lands as not followed. **A word on neither list is
//! `gone`**, which is the case this module exists for: `pnpm typecheck` after
//! `typecheck` left `package.json`. A subcommand missing from a list below is a
//! false `gone` on the one repository that uses it, and the fix is a word here.
//!
//! **`cargo` is the opposite way round.** A cargo word that is neither built in
//! nor an alias is an external subcommand — `cargo nextest` is `cargo-nextest`
//! on the machine — so it is not followed and never `gone`. The consequence is
//! stated rather than hidden: an alias that is deleted moves its row from
//! checked to not followed, not to `gone`, because nothing in the repository
//! can tell a deleted alias from a plugin that was always meant to be installed.
//!
//! # Where a flag stops the read
//!
//! Before the script word, a flag this read does not know might take a value,
//! and that value would then be read as the script. So an unknown flag there
//! ends the follow with the flag recorded, rather than guessing and risking a
//! false `gone`. `--name=value` is always safe to step over.

use super::declared::Repository;
use super::{named_path, Found};

/// What `pnpm` answers for itself, without a script of that name.
const PNPM_OWN: &[&str] = &[
    "add",
    "approve-builds",
    "audit",
    "bin",
    "cat-file",
    "cat-index",
    "catalog",
    "config",
    "create",
    "dedupe",
    "deploy",
    "doctor",
    "env",
    "fetch",
    "find-hash",
    "help",
    "i",
    "ignored-builds",
    "import",
    "init",
    "install",
    "install-test",
    "it",
    "licenses",
    "link",
    "list",
    "ln",
    "ls",
    "outdated",
    "pack",
    "patch",
    "patch-commit",
    "patch-remove",
    "prune",
    "publish",
    "rb",
    "rebuild",
    "remove",
    "rm",
    "root",
    "sbom",
    "self-update",
    "server",
    "setup",
    "start",
    "store",
    "t",
    "test",
    "un",
    "uninstall",
    "unlink",
    "up",
    "update",
    "version",
    "why",
];

/// What `npm` answers for itself, without a script of that name.
const NPM_OWN: &[&str] = &[
    "access",
    "adduser",
    "audit",
    "bugs",
    "cache",
    "ci",
    "completion",
    "config",
    "dedupe",
    "deprecate",
    "diff",
    "dist-tag",
    "docs",
    "doctor",
    "edit",
    "explain",
    "explore",
    "find-dupes",
    "fund",
    "help",
    "hook",
    "i",
    "init",
    "install",
    "install-ci-test",
    "install-test",
    "it",
    "link",
    "ll",
    "login",
    "logout",
    "ls",
    "org",
    "outdated",
    "owner",
    "pack",
    "ping",
    "pkg",
    "prefix",
    "profile",
    "prune",
    "publish",
    "query",
    "rebuild",
    "repo",
    "restart",
    "root",
    "sbom",
    "search",
    "shrinkwrap",
    "star",
    "stars",
    "start",
    "stop",
    "t",
    "team",
    "test",
    "token",
    "uninstall",
    "unpublish",
    "unstar",
    "update",
    "version",
    "view",
    "whoami",
];

/// Flags either package manager takes before the script word, with no value.
const PACKAGE_MANAGER_BARE_FLAGS: &[&str] = &[
    "--aggregate-output",
    "--frozen-lockfile",
    "--if-present",
    "--offline",
    "--parallel",
    "--prefer-offline",
    "--recursive",
    "--silent",
    "--stream",
    "--workspace-root",
    "-r",
    "-s",
];

/// What `cargo` answers for itself.
const CARGO_OWN: &[&str] = &[
    "add",
    "b",
    "bench",
    "build",
    "c",
    "check",
    "clean",
    "clippy",
    "config",
    "d",
    "doc",
    "fetch",
    "fix",
    "fmt",
    "generate-lockfile",
    "help",
    "info",
    "init",
    "install",
    "locate-project",
    "login",
    "logout",
    "metadata",
    "miri",
    "new",
    "owner",
    "package",
    "pkgid",
    "publish",
    "r",
    "remove",
    "report",
    "rm",
    "run",
    "rustc",
    "rustdoc",
    "search",
    "t",
    "test",
    "tree",
    "uninstall",
    "update",
    "vendor",
    "verify-project",
    "version",
    "yank",
];

/// Flags cargo takes before its subcommand, with no value.
const CARGO_BARE_FLAGS: &[&str] = &[
    "--frozen",
    "--locked",
    "--offline",
    "--quiet",
    "--verbose",
    "-q",
    "-v",
    "-vv",
];

/// Follow one line's program into the build files it would consult.
pub(super) fn follow(
    program: &str,
    arguments: &[String],
    repo: &mut Repository,
    found: &mut Found,
) {
    // A program that is itself a repository path was looked for already.
    if named_path(program).is_some() {
        return;
    }
    match program {
        "pnpm" => package_manager(program, PNPM_OWN, arguments, repo, found),
        "npm" => package_manager(program, NPM_OWN, arguments, repo, found),
        "cargo" => cargo(arguments, repo, found),
        _ => found.unfollowed(program, "not a tool this read follows"),
    }
}

fn package_manager(
    tool: &str,
    own: &[&str],
    arguments: &[String],
    repo: &mut Repository,
    found: &mut Found,
) {
    let mut dir = String::new();
    let mut words = arguments.iter();
    let word = loop {
        let Some(word) = words.next() else {
            return found.unfollowed(tool, "names no script");
        };
        match word.as_str() {
            "-C" | "--dir" | "--prefix" => match words.next() {
                Some(value) => dir = value.clone(),
                None => return found.unfollowed(word, "a directory flag with no directory"),
            },
            "--filter" | "-F" | "--workspace" => {
                let value = words.next().unwrap_or(word);
                return found.unfollowed(
                    value,
                    "a workspace filter names a package by name, and this read follows a directory",
                );
            }
            // pnpm's `-w` is the workspace root and takes nothing; npm's takes a
            // package name, which is the filter case above.
            "-w" if tool == "pnpm" => dir.clear(),
            "-w" => {
                let value = words.next().unwrap_or(word);
                return found.unfollowed(
                    value,
                    "a workspace filter names a package by name, and this read follows a directory",
                );
            }
            flag if flag.starts_with("--dir=") || flag.starts_with("--prefix=") => {
                dir = flag
                    .split_once('=')
                    .map(|(_, v)| v.to_string())
                    .unwrap_or_default();
            }
            flag if flag.starts_with("--filter=") || flag.starts_with("--workspace=") => {
                return found.unfollowed(
                    flag,
                    "a workspace filter names a package by name, and this read follows a directory",
                );
            }
            flag if flag.starts_with("--") && flag.contains('=') => {}
            flag if PACKAGE_MANAGER_BARE_FLAGS.contains(&flag) => {}
            flag if flag.starts_with('-') => {
                return found.unfollowed(
                    flag,
                    "a flag this read does not know, so the word after it may be its value",
                );
            }
            _ => break word,
        }
    };

    let (script, explicit) = match word.as_str() {
        "run" | "run-script" => match words.find(|one| !one.starts_with('-')) {
            Some(script) => (script, true),
            None => return found.unfollowed(word, "names no script"),
        },
        "exec" | "dlx" | "x" => {
            let binary = words.find(|one| !one.starts_with('-')).unwrap_or(word);
            return found.unfollowed(
                binary,
                "run from node_modules, which is installed rather than committed",
            );
        }
        _ => (word, false),
    };

    if !dir.is_empty() {
        // The trailing `/` makes any directory a path-shaped word, so what is
        // left for `named_path` to refuse is absolute, `..`, or a pattern.
        if named_path(&format!("{dir}/")).is_none() {
            return found.unfollowed(&dir, "a directory outside the checkout");
        }
        if !repo.checkout().join(&dir).is_dir() {
            // With a `/` the path rule has already called it missing.
            if !dir.contains('/') {
                found.missing(dir.clone());
            }
            return found.unfollowed(script, "its package directory is missing");
        }
    }

    let manifest = match dir.is_empty() {
        true => "package.json".to_string(),
        false => format!("{dir}/package.json"),
    };
    let present = repo.checkout().join(&manifest).is_file();
    let Some(scripts) = repo.scripts(&dir) else {
        return match present {
            true => found.unfollowed(script, &format!("{manifest} could not be read")),
            false => found.unfollowed(
                script,
                &format!("no {manifest}; {tool} would look further up, and this read does not"),
            ),
        };
    };

    if scripts.contains(script.as_str()) {
        found.found();
    } else if !explicit && own.contains(&script.as_str()) {
        found.unfollowed(
            script,
            &format!("{tool}'s own subcommand, not a script this repository declares"),
        );
    } else {
        found.missing(format!("{manifest}: scripts.{script}"));
    }
}

fn cargo(arguments: &[String], repo: &mut Repository, found: &mut Found) {
    let mut words = arguments.iter().peekable();
    let mut subcommand = None;
    while let Some(word) = words.next() {
        match word.as_str() {
            flag if flag.starts_with('+') => {}
            flag if CARGO_BARE_FLAGS.contains(&flag) => {}
            flag if flag.starts_with("--") && flag.contains('=') => {}
            flag if flag.starts_with('-') => {
                found.unfollowed(
                    flag,
                    "a flag this read does not know, so the word after it may be its value",
                );
                break;
            }
            _ => {
                subcommand = Some(word);
                break;
            }
        }
    }

    let mut packages: Vec<String> = Vec::new();
    if let Some(sub) = subcommand {
        let aliases = repo.aliases().cloned();
        match aliases.as_ref().and_then(|table| table.get(sub.as_str())) {
            Some(expansion) => {
                found.found();
                let expanded: Vec<String> =
                    expansion.split_whitespace().map(str::to_string).collect();
                packages.extend(named_packages(&expanded, found));
            }
            None if CARGO_OWN.contains(&sub.as_str()) => {
                found.unfollowed(sub, "cargo's own subcommand");
            }
            None if aliases.is_none() => {
                found.unfollowed(sub, ".cargo/config.toml could not be read");
            }
            None => found.unfollowed(
                sub,
                "not an alias this repository declares, so an external cargo subcommand \
                 installed on the machine",
            ),
        }
    }
    packages.extend(named_packages(arguments, found));

    for package in packages {
        match repo.members() {
            None => found.unfollowed(&package, "the workspace's members could not be read"),
            Some(members) if members.contains(&package) => found.found(),
            Some(_) => found.missing(format!("Cargo.toml: workspace member {package}")),
        }
    }
}

/// Every package a cargo line names with `-p` or `--package`.
///
/// A line that points cargo at another manifest names packages in *that*
/// workspace, which is not the one this read has, so it follows none of them.
fn named_packages(words: &[String], found: &mut Found) -> Vec<String> {
    if let Some(elsewhere) = words.iter().find(|one| one.starts_with("--manifest-path")) {
        found.unfollowed(
            elsewhere,
            "another manifest's workspace, which this read does not follow",
        );
        return Vec::new();
    }
    let mut named = Vec::new();
    let mut words = words.iter();
    while let Some(word) = words.next() {
        let value = match word.as_str() {
            "-p" | "--package" => words.next().cloned(),
            flag if flag.starts_with("--package=") => {
                flag.split_once('=').map(|(_, v)| v.to_string())
            }
            _ => None,
        };
        let Some(value) = value else { continue };
        // `name@version` names the package `name`.
        let name = value.split('@').next().unwrap_or(&value).to_string();
        if name.contains(['*', '?', '[']) {
            found.unfollowed(&value, "a package pattern, which cargo matches for itself");
            continue;
        }
        named.push(name);
    }
    named
}
