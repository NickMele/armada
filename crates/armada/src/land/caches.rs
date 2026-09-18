//! What is cached per base commit, across every turn gated against it:
//! `main`'s own `verify-foundations` report, and which Checks a red turn
//! found already failing there. `scripts/land`'s `base_foundations` and
//! `checks_on_the_base`.

use std::collections::BTreeMap;
use std::path::Path;

use super::armada_cli::check;
use super::codec;
use super::dir::StateDir;
use super::env::Env;
use super::gate::{a_report, already_red_on_base};
use super::prepare::seed;
use super::shell::run;
use super::stop::Stopped;
use super::worktree::{reused_keeping, LandWorktree};

/// `base`'s own `verify-foundations` output, raw — read from the cache
/// where it holds a genuine report, taken fresh into the `base` worktree
/// otherwise. The caller reads it through [`super::gate::foundations_delta`],
/// which does its own filtering to `FAIL`/`missing:` lines.
///
/// **Cached only once read as a report.** A half-written or crashed run
/// cached under a base sha would read `main`'s own failures as new for
/// every branch after it — [`a_report`] is what a cache entry is validated
/// against before it is trusted, on read as much as on write.
pub fn base_foundations(
    repo: &Path,
    state: &StateDir,
    base: &str,
    env: &Env,
    logs: &Path,
) -> Result<String, Stopped> {
    let cached_path = state.foundations_report_path(base);
    if let Ok(held) = std::fs::read_to_string(&cached_path) {
        if let Some(body) = held.strip_prefix("report\n") {
            return Ok(body.to_string());
        }
    }

    let keep = env.keep_refs();
    let at = reused_keeping(repo, LandWorktree::Base, base, logs, &keep)
        .map_err(|why| Stopped::stopped(why.to_string()))?;
    seed(repo, &at, env, logs)?;
    let log = logs.join("foundations-base.log");
    let argv: Vec<&str> = env.foundations.iter().map(String::as_str).collect();
    let ran = run(&argv, &at, None, Some(&log))?;
    let said = ran.combined();
    if !a_report(&said, exit_code(&ran)) {
        return Err(Stopped::stopped(format!(
            "{base_branch} itself is broken: `{}` exited {} there and named no failing rule, so \
             nothing can be gated against it and this branch is not at fault. Fix {base_branch} \
             and land that first; see foundations-base.log",
            env.foundations.join(" "),
            exit_code(&ran),
            base_branch = env.base,
        )));
    }
    write_atomically(&cached_path, &format!("report\n{said}"))?;
    Ok(said)
}

/// Of `names`, the ones already known to fail on `base` itself — running
/// whichever of them the per-base cache has not already answered, and
/// answering nothing about a Check nobody has asked this yet.
///
/// **Only ever asked about a Check that just failed**, so the cost lands on
/// a failing turn and on nothing else.
pub fn checks_on_the_base(
    repo: &Path,
    state: &StateDir,
    base: &str,
    names: &[String],
    env: &Env,
    logs: &Path,
) -> Result<Vec<String>, Stopped> {
    let cache_path = state.checks_cache_path(base);
    let mut known: BTreeMap<String, bool> = codec::read("check verdicts on the base", &cache_path)
        .map_err(|why| Stopped::stopped(why.to_string()))?
        .unwrap_or_default();
    let unknown: Vec<&String> = names
        .iter()
        .filter(|name| !known.contains_key(*name))
        .collect();
    if !unknown.is_empty() {
        let keep = env.keep_refs();
        let at = reused_keeping(repo, LandWorktree::Base, base, logs, &keep)
            .map_err(|why| Stopped::stopped(why.to_string()))?;
        seed(repo, &at, env, logs)?;
        super::prepare::setup(&at, env, logs)?;
        for name in unknown {
            let log = logs.join(format!("{name}-on-{}.log", env.base));
            let ran = check(&env.armada, &at, name, &log)?;
            known.insert(name.clone(), ran.passed);
        }
        codec::write(&cache_path, &known).map_err(|why| Stopped::stopped(why.to_string()))?;
    }
    Ok(already_red_on_base(names, &known))
}

fn exit_code(ran: &super::shell::Ran) -> i32 {
    ran.status_code()
}

fn write_atomically(path: &Path, body: &str) -> Result<(), Stopped> {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{}.tmp", std::process::id()));
    let staging = path.with_file_name(name);
    std::fs::write(&staging, body)
        .and_then(|_| std::fs::rename(&staging, path))
        .map_err(|why| Stopped::stopped(format!("{} could not be written: {why}", path.display())))
}
