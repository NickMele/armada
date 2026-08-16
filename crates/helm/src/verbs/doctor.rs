//! `armada doctor` — what this machine is missing or has drifted on.
//!
//! Read-only, and **safe to run in a shell prompt**: a warning alone does not
//! fail (`docs/commands/doctor.md`), so a guild three commits behind does not
//! make every prompt red.
//!
//! # The groups, in order
//!
//! | | Reports |
//! |---|---|
//! | **Tooling** | `git`, `claude`, a container runtime: present, and version |
//! | **Layout** | `~/.armada/` with its directories and `machine.yml` |
//! | **Guild drift** | behind, ahead of, or diverged from the remote, and by how many commits |
//! | **Fragments** | which of the three are still whatever import produced |
//! | **Projection** | whether the guild Claude Code reads is the guild you have |
//!
//! **Guild drift is the check that earns the command.** Two machines silently
//! diverging is the guild's main failure mode (`PHASES.md` §11), and it is the
//! one thing nothing else on the machine would ever tell you.
//!
//! **The projection group was withheld until there was a projector**, because a
//! `doctor` reporting `ok` for a check that ran nothing is the worse of the two
//! failures — a machine reported fine when nobody looked. It runs a survey,
//! which writes nothing, so the command stays read-only.
//!
//! # Offline is not a failure
//!
//! The drift check needs the network and degrades to `offline` without it,
//! which is a warning. A `doctor` that failed on a train would be a `doctor`
//! nobody runs on a train, which is where you most want it.

use armada_core::ctx::{Run, RunRequest};
use armada_core::disk;
use armada_core::envelope::{DoctorData, Envelope, Finding, Problem, Settled};
use armada_core::error::ArmadaError;
use armada_guild::layout::{self, Guild, DIRECTORIES};
use armada_guild::{machine, memory, projector, repo};
use armada_manifest::docker;
use std::path::Path;

use crate::verbs::guild::Where;
use crate::verbs::{preflight, Output};

/// Look at this machine and report.
///
/// `--fix` is accepted and refused by name: repairing a drifted guild is
/// `armada guild pull`, which the `→` line already names, and a flag that
/// silently did half of what it promised would be worse than one that says it
/// is not built.
pub fn run(runner: &impl Run, place: &Where) -> Result<Output, ArmadaError> {
    let guild = place.guild();
    // **Ordered so that a check's rows are contiguous**, because the render
    // draws one table per check (`render.rs`). `guild` reports drift and then
    // every fragment still as imported, and interleaving those with `manifest.db`
    // is what made a real reader ask which rows belonged together.
    let mut results = preflight::run(runner, &place.cwd, true).results;
    results.push(drone_argv(runner, &place.cwd, &place.armada_home));
    results.push(helm_argv(runner, &place.cwd, &place.armada_home));
    results.push(directories(&place.armada_home));
    results.extend(drift(runner, &guild));
    results.extend(fragments(&guild));
    results.extend(projection(place, &guild));
    results.push(store(&place.armada_home));
    // **Last, and beside the store, because they answer the same question.**
    // `manifest.db`'s reclaimable rows and Docker's reclaimable bytes are both
    // "what has quietly accumulated", and a reader who has just been told about
    // one is already asking about the other.
    results.extend(docker_disk(runner, &place.cwd));

    let status = DoctorData::verdict(&results);
    Ok(Output::Doctor(Box::new(Envelope::ok(
        "doctor",
        None,
        status,
        DoctorData {
            tally: DoctorData::tally(&results),
            headline: DoctorData::headline(&results),
            results,
        },
    ))))
}

/// **Would a Drone actually start?**
///
/// This check exists because of a specific failure, and it is worth stating in
/// full. Fleet's Drone argv was `claude --session-id <uuid> --print
/// --output-format stream-json <prompt>`, which Claude Code rejects at
/// argument-parse time: that combination requires `--verbose`. Every test passed
/// — the unit tests on the built vector, and an integration test running a stub
/// that recorded what `execve` received — while **no Drone had ever run**.
///
/// > **Asserting on argv proves you built the argv you intended. It does not
/// > prove the argv is accepted.** A suite that only makes the first claim is
/// > green on a program that never starts (`docs/traps.md`).
///
/// So this asks the binary, twice, and neither question costs a token:
///
/// 1. **Every flag the Drone uses is still offered**, read off `claude --help`.
///    That catches a flag renamed or removed by a new version — the general
///    shape of the failure this is a sample of.
/// 2. **The real argument validator accepts the real combination.** The Drone's
///    own argv is run with its prompt replaced by `--input-format stream-json`
///    and nothing on stdin: Claude Code validates every flag, starts the
///    session, gets EOF and exits **without making an API call**
///    ([`armada_core::fleet::drone::probe_argv`]). A usage error there is the
///    finding, verbatim.
///
/// **The residual, stated rather than hidden.** `claude --help` does not
/// document the `--verbose` requirement, and `--help` short-circuits before
/// validation — so nothing can enumerate combination rules in advance. Probe 2
/// covers the combination Armada actually uses, which is the one that matters,
/// and it would have caught the failure this check was written after.
///
/// **The brief is inside probe 2 rather than beside it**, and that is the point
/// of building the probe out of the real [`armada_core::fleet::drone::spawn_argv`]
/// rather than out of an approximation. A Drone's argv now carries a
/// multi-kilobyte `--append-system-prompt` value; the question *"does this
/// machine's `claude` accept that alongside `dontAsk`, two variadic tool lists
/// and `stream-json`"* is one only the real validator can answer, and it answers
/// it here for free. Probe 0 below is the one thing that must be settled before
/// the binary is asked anything at all.
fn drone_argv(runner: &impl Run, cwd: &Path, armada_home: &Path) -> Finding {
    use armada_core::fleet::drone as argv;

    // **Probe 3: the relay's flag, on a document that registers no hook**
    // (`020` §1). `--settings` is what carries a Drone's `Stop` hook, and its
    // disappearance is the quietest failure of the lot: every Job still spawns,
    // and none of them ever advances a step again. The document registers
    // nothing, because a probe that fired a relay would tick the fleet about an
    // exchange that never happened — and it is written under `~/.armada/`
    // rather than beside the caller, since `doctor` runs in somebody's
    // repository and must not leave a file in it.
    //
    // **Into a `~/.armada/` that already exists, and never creating one.**
    // `doctor` is a read verb, and a machine with no `~/.armada/` is a finding
    // three rows down — a probe that created the directory would answer the
    // question `directories` is asking, and `armada init`'s whole first-run
    // contract is that the directory is not there yet.
    //
    // **A path that will not take the file is not a finding either.** Failing
    // `doctor` over a scratch write would report a machine as broken for a
    // reason that has nothing to do with Claude Code; the probe simply runs
    // without the flag, and every other check stands.
    let no_hooks = armada_home.join(format!("probe-{}.json", argv::PROBE_SESSION));
    let registered = (armada_home.is_dir() && std::fs::write(&no_hooks, argv::NO_HOOKS).is_ok())
        .then(|| no_hooks.display().to_string());

    // **Probe 0: the brief cannot eat the flag behind it.** `--append-system-prompt`
    // takes a value, so a brief that were empty or flag-shaped would consume
    // `--permission-mode` — and the argv that results is still *accepted*. The
    // failure is a Drone with no posture, which is `docs/reserved/011`'s stall
    // arriving by a new route and reporting nothing. It is checked here rather
    // than only in a unit test because this is the vector this machine would
    // run, and because a probe whose flags had silently shifted by one would
    // then be validating a combination Armada does not use.
    // **The probe carries the MCP flags too, for the reason above**: a machine
    // that accepts `--mcp-config` is what makes a Drone able to report at all,
    // and a probe missing the flag would validate a combination Armada does not
    // run. Written beside the hooks document and on the same terms — into an
    // existing `~/.armada/` only, and a path that will not take the file simply
    // runs without it.
    let probe_mcp = armada_home.join(format!("probe-{}.mcp.json", argv::PROBE_SESSION));
    let attached = (armada_home.is_dir()
        && std::fs::write(&probe_mcp, armada_core::helm::mcp_json("armada")).is_ok())
    .then(|| probe_mcp.display().to_string());

    let probe = argv::probe_argv(registered.as_deref(), attached.as_deref());
    if let Some(at) = probe.iter().position(|word| word == argv::APPEND) {
        let carried = probe.get(at + 1).map(String::as_str).unwrap_or("");
        if carried.trim().is_empty() || carried.starts_with('-') {
            return Finding::needs(
                "drone argv",
                Problem::Missing,
                "the Drone's brief is empty, so `--append-system-prompt` would swallow \
                 the flag after it",
                "report this to Armada: a Drone would run with no posture",
            );
        }
    }

    let help = runner.call(
        &RunRequest::new(
            vec![argv::CLAUDE.to_string(), "--help".to_string()],
            cwd.to_path_buf(),
        )
        .timeout(PROBE),
    );
    let Ok(help) = help.and_then(|output| match output.ok() {
        true => Ok(output),
        false => Err(armada_core::ctx::SpawnError {
            program: argv::CLAUDE.to_string(),
            kind: armada_core::ctx::SpawnErrorKind::Other,
            message: "`--help` exited non-zero".to_string(),
        }),
    }) else {
        // A `claude` that will not answer `--help` is already reported by the
        // tooling check above; saying it twice would be noise.
        return Finding::needs(
            "drone argv",
            Problem::Offline,
            "`claude --help` did not answer, so the argv was not checked",
            "check `claude --help` runs",
        );
    };

    let missing: Vec<&str> = argv::FLAGS
        .iter()
        .copied()
        .filter(|flag| !help.stdout.contains(flag))
        .collect();
    if !missing.is_empty() {
        return Finding::needs(
            "drone argv",
            Problem::Missing,
            format!(
                "`claude` no longer offers {}: a Drone would not start",
                missing.join(", ")
            ),
            "pin an older claude, or report this to Armada",
        );
    }

    // The real validator, on the real combination — brief included.
    let Ok(output) = runner.call(&RunRequest::new(probe, cwd.to_path_buf()).timeout(PROBE)) else {
        return Finding::needs(
            "drone argv",
            Problem::Offline,
            "the argv probe would not run",
            "check `claude` runs",
        );
    };

    let said = format!("{}{}", output.stderr, output.stdout);
    if let Some(complaint) = said
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("Error:"))
    {
        return Finding::needs(
            "drone argv",
            Problem::Missing,
            format!("`claude` refuses the Drone's argv: {complaint}"),
            "armada cannot start a Drone until this is fixed",
        );
    }

    // **A turn would mean the probe cost something, and it must never.** No
    // `result` event is emitted when there is no input to answer; one appearing
    // says a future version began answering an empty conversation, and that is
    // reported rather than quietly paid for.
    if said.contains("total_cost_usd") {
        return Finding::needs(
            "drone argv",
            Problem::Partial,
            "the argv probe started a turn; it is meant to cost nothing",
            "report this to Armada: the doctor probe needs narrowing",
        );
    }

    // The probe's scratch document goes with it: it is a fixed name beside the
    // caller's directory, and leaving one behind on every `doctor` run would be
    // litter in somebody's repository.
    let _ = std::fs::remove_file(&no_hooks);

    Finding::settled(
        "drone argv",
        Settled::Ok,
        format!("{} flags accepted", argv::FLAGS.len()),
    )
}

/// The deadline on either half of the argv probe. Both are local; the probe
/// starts a session and exits at EOF, so anything approaching this is a wedged
/// binary or a hook that hangs.
const PROBE: std::time::Duration = std::time::Duration::from_secs(20);

/// **Would Helm actually start?**
///
/// The same question [`drone_argv`] asks, arriving at the other argv Armada
/// builds — and it is asked here for a sharper reason. A Drone that will not
/// start shows up as a Job that stalls, and `armada fleet ls` says so. A Helm
/// that will not start shows up as *nothing*, because Helm is how the reader
/// would have asked.
///
/// **Two questions, and neither opens a session or spends a token.**
///
/// 1. **Every flag the launch uses is still offered**, read off `claude --help`.
///    That is the shape of the failure Fleet already shipped once: an argv every
///    test agreed on and the binary rejects at argument-parse time
///    (`docs/traps.md`).
/// 2. **The monitor's plugin still validates**, via `claude plugin validate` —
///    the tool's own manifest checker, which reads a directory and starts
///    nothing. A plugin Claude Code will not load is an inbox with no live push
///    and no error anywhere; the `Stop` hook would still be a backstop, so the
///    failure is *quietly slower* rather than visible, which is exactly the kind
///    that survives for months.
///
/// **There is deliberately no session probe here.** The Drone's probe is safe
/// because a headless turn with closed stdin makes no API call; Helm is
/// interactive, and there is no equivalent that is provably free. A check that
/// might open the reader's orchestrator — against their account, on a machine
/// they were only asking about — is not worth the coverage.
///
/// The plugin half is **skipped rather than failed** when `armada helm` has
/// never run: there is nothing to validate, and a `doctor` that reported a
/// problem for a verb the reader has not used yet would be noise.
fn helm_argv(runner: &impl Run, cwd: &Path, armada_home: &Path) -> Finding {
    use armada_core::fleet::drone::CLAUDE;
    use armada_core::helm;

    let help = runner.call(
        &RunRequest::new(
            vec![CLAUDE.to_string(), "--help".to_string()],
            cwd.to_path_buf(),
        )
        .timeout(PROBE),
    );
    let Some(help) = help.ok().filter(|output| output.ok()) else {
        // Already reported by the tooling check above; saying it twice is noise.
        return Finding::needs(
            "helm argv",
            Problem::Offline,
            "`claude --help` did not answer, so the launch was not checked",
            "check `claude --help` runs",
        );
    };

    let missing: Vec<&str> = helm::FLAGS
        .iter()
        .copied()
        .filter(|flag| !help.stdout.contains(flag))
        .collect();
    if !missing.is_empty() {
        return Finding::needs(
            "helm argv",
            Problem::Missing,
            format!(
                "`claude` no longer offers {}: Helm would not start",
                missing.join(", ")
            ),
            "pin an older claude, or report this to Armada",
        );
    }

    let plugin = armada_home.join(helm::DIRECTORY).join("plugin");
    if !plugin.join(".claude-plugin/plugin.json").is_file() {
        return Finding::settled(
            "helm argv",
            Settled::Ok,
            format!(
                "{} flags accepted; run `armada helm` to wire the inbox; entering is {}",
                helm::FLAGS.len(),
                entering_word(armada_home)
            ),
        );
    }

    let validated = runner.call(
        &RunRequest::new(
            vec![
                CLAUDE.to_string(),
                "plugin".to_string(),
                "validate".to_string(),
                plugin.display().to_string(),
            ],
            cwd.to_path_buf(),
        )
        .timeout(PROBE),
    );
    match validated {
        Ok(output) if output.ok() => Finding::settled(
            "helm argv",
            Settled::Ok,
            // **The row says what was checked and whether entering is on**,
            // in that order, because a reader who saw only the first would
            // conclude `armada helm` opens a session either way.
            //
            // **It gives the state and not the reason**, which is the one
            // place that phrasing is not read from
            // [`ENTER_IS_OFF`](crate::verbs::helm::ENTER_IS_OFF) — and
            // deliberately. A `doctor` detail is a table cell about
            // sixty-five columns wide; the full refusal truncates mid-word,
            // and half a reason is worse than a short state. `armada helm
            // --help` and `armada helm enable --help` are the two surfaces a
            // reader reaches for next.
            format!(
                "{} flags accepted, monitor validates; entering is {}",
                helm::FLAGS.len(),
                entering_word(armada_home)
            ),
        ),
        Ok(output) => Finding::needs(
            "helm argv",
            Problem::Missing,
            format!(
                "`claude` will not load Helm's monitor: {}",
                complaint(&output).unwrap_or_else(|| "no reason given".to_string())
            ),
            "`armada helm` rewrites it; report this to Armada if it persists",
        ),
        Err(_) => Finding::needs(
            "helm argv",
            Problem::Offline,
            "`claude plugin validate` would not run",
            "check `claude` runs",
        ),
    }
}

/// `"on"` or `"off"` — the word this row's detail carries, read from the same
/// switch [`crate::verbs::helm::entering_is_off`] refuses against.
///
/// **A word, never the full sentence.** [`helm_argv`]'s two call sites explain
/// why: the column truncates a reason mid-word, and does not truncate a word
/// this short.
fn entering_word(armada_home: &Path) -> &'static str {
    match crate::machine::read(armada_home).enter {
        true => "on",
        false => "off",
    }
}

/// The first line of a refusal, whichever stream it came out on.
fn complaint(output: &armada_core::ctx::RunOutput) -> Option<String> {
    format!("{}{}", output.stderr, output.stdout)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

/// `~/.armada/` and its three directories.
///
/// **The check is named for the thing, not for the code that checks it.** It
/// used to be called `layout` and to report `no jobs, workspaces` — an
/// implementation word over a set difference, which told a real reader neither
/// what was missing nor why he should care. It is now named `~/.armada` and says
/// which paths are absent and what writes to them.
fn directories(armada_home: &Path) -> Finding {
    let home = crate::verbs::machine::shown(armada_home);
    if !armada_home.is_dir() {
        return Finding::needs(
            &home,
            Problem::Missing,
            format!("{home} is not there; nothing Armada keeps is on this machine"),
            "armada init",
        );
    }
    let missing: Vec<&str> = DIRECTORIES
        .iter()
        .copied()
        .filter(|directory| !armada_home.join(directory).is_dir())
        .collect();
    if missing.is_empty() {
        return Finding::settled(
            &home,
            Settled::Ok,
            format!("{}, all present", paths(&DIRECTORIES)),
        );
    }
    Finding::needs(
        &home,
        Problem::Missing,
        format!(
            "{} {} missing; {} {} there",
            paths(&missing),
            if missing.len() == 1 { "is" } else { "are" },
            written(&missing.iter().map(|d| layout::holds(d)).collect::<Vec<_>>()),
            if missing.len() == 1 { "goes" } else { "go" },
        ),
        // **`--force`, and it no longer means "replace the guild".** A re-run
        // recreates what is missing and leaves an existing guild alone
        // (`verbs/machine.rs`), which is what makes naming it here safe.
        "armada init --force",
    )
}

/// `jobs/ and workspaces/` — directory names, written as paths so a reader can
/// see they are directories.
fn paths(directories: &[&str]) -> String {
    written(
        &directories
            .iter()
            .map(|d| format!("{d}/"))
            .collect::<Vec<_>>(),
    )
}

/// `a`, `a and b`, `a, b and c`.
fn written(items: &[impl AsRef<str>]) -> String {
    let items: Vec<&str> = items.iter().map(AsRef::as_ref).collect();
    match items.split_last() {
        None => String::new(),
        Some((last, [])) => (*last).to_string(),
        Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
    }
}

/// How far this machine's guild is from its remote.
///
/// **The check that earns the command.** Two machines silently diverging is the
/// guild's main failure mode, and nothing else on the machine would tell you.
fn drift(runner: &impl Run, guild: &Guild) -> Vec<Finding> {
    if !guild.exists() {
        return vec![Finding::needs(
            GUILD,
            Problem::Missing,
            "there is no guild on this machine",
            "armada guild init",
        )];
    }
    let remote = match repo::remote(runner, guild.root()) {
        Ok(Some(remote)) => remote,
        // **Sync off is the documented default, not a problem.** `export` still
        // works, and a `doctor` that warned about it every day would train the
        // reader to skim the report.
        Ok(None) => {
            return vec![Finding::settled(
                GUILD,
                Settled::Ok,
                "no remote: sync off, export still works",
            )]
        }
        Err(error) => return vec![offline(&error)],
    };
    let _ = remote;

    let apart = match repo::fetch(runner, guild.root()) {
        Ok(apart) => apart,
        Err(error) => return vec![offline(&error)],
    };
    if apart.diverged() {
        return vec![Finding::needs(
            GUILD,
            Problem::Stale,
            apart.written(),
            "armada guild pull",
        )];
    }
    let mut found = Vec::new();
    if apart.behind > 0 {
        found.push(Finding::needs(
            GUILD,
            Problem::Stale,
            format!(
                "{} behind origin",
                crate::render::format::count(apart.behind, "commit")
            ),
            "armada guild pull",
        ));
    }
    if apart.ahead > 0 {
        found.push(Finding::needs(
            GUILD,
            Problem::Stale,
            format!(
                "{} not pushed",
                crate::render::format::count(apart.ahead, "commit")
            ),
            "armada guild push",
        ));
    }
    if found.is_empty() {
        found.push(Finding::settled(GUILD, Settled::Ok, "in step with origin"));
    }
    found
}

/// The check every guild row belongs to. One spelling, because the render groups
/// on it.
const GUILD: &str = "guild";

/// A drift check that could not run. **A warning, never a failure.**
fn offline(error: &ArmadaError) -> Finding {
    Finding::needs(
        GUILD,
        Problem::Offline,
        format!("could not reach the remote: {}", error.message),
        // Not nothing. "Could not be checked" without a next step reads as a
        // fault the reader has to diagnose, and it is usually a train.
        "reconnect, then armada doctor again",
    )
}

/// Which fragments are still whatever import produced.
///
/// **The half of `--defaults` that has to be reported.** A skipped interview
/// leaves a *working* guild, and the promise `PLAN.md` §13.4 makes is that
/// `doctor` names the fragments that are still a machine's reading of your
/// memory file rather than your own words. Without this, `--defaults` would
/// finish silently in a state that looks configured and is not — the exact
/// failure that section forbids.
fn fragments(guild: &Guild) -> Vec<Finding> {
    if !guild.exists() {
        return Vec::new();
    }
    memory::FRAGMENTS
        .iter()
        .filter_map(|name| {
            let body = std::fs::read_to_string(guild.path(name)).ok()?;
            // **Asked of the file rather than matched here.** `armada-guild`
            // writes these and owns what "still Armada's words" looks like; a
            // second reading of it in this file is how the two eventually
            // disagree about whether somebody has written his own voice.
            Some((name, memory::state(&body)?))
        })
        .map(|(name, state)| {
            Finding::needs(
                GUILD,
                Problem::Partial,
                format!("{name} {}", state.said()),
                // **A sentence rather than a command, and that is still a fix.**
                // There is no verb that writes this for you — `armada guild
                // edit` is not built — so the remedy is the path and what to do
                // with it. An earlier version left this `None` on the grounds
                // that prose is not a command, and the row reached a reader with
                // nothing to act on, which is the failure the rule is about.
                format!(
                    "write {} in your own words",
                    crate::verbs::machine::shown(&guild.path(name))
                ),
            )
        })
        .collect()
}

/// The check every projection row belongs to. Named for the directory it is
/// about, exactly as `~/.armada` is.
const CLAUDE: &str = "~/.claude";

/// **Is the guild Claude Code reads the guild you have?**
///
/// The group `docs/commands/doctor.md` reserved and could not report until
/// something projected. It runs a survey, which writes nothing: a check that had
/// to project in order to report on projection would be a check nobody could run
/// twice, and `doctor` is read-only.
///
/// **Nothing at all when there is no guild.** `drift` has already said so in the
/// group above, and a second row saying it again is a row a reader has to read
/// twice to learn nothing — the same reasoning that kept this group out of the
/// report entirely until a projector existed.
fn projection(place: &Where, guild: &Guild) -> Vec<Finding> {
    if !guild.exists() {
        return Vec::new();
    }
    let steps = projector::survey(guild, &place.claude_home, &place.armada_home);
    let mut found = Vec::new();

    // **Yours first, because it is the row that needs a person.** Out of date is
    // one command away; a file left alone is a decision only the reader can
    // make, and burying it under the count of what is stale is how it gets
    // missed.
    let yours: Vec<&str> = steps
        .iter()
        .filter(|step| step.is_yours())
        .map(|step| step.at.as_str())
        .collect();
    if !yours.is_empty() {
        found.push(Finding::needs(
            CLAUDE,
            Problem::Partial,
            format!(
                "{} yours rather than the guild's: {}",
                crate::render::format::count(yours.len(), "file"),
                a_few(&yours)
            ),
            "delete yours and run armada guild project, or keep it",
        ));
    }

    let behind = steps
        .iter()
        .filter(|step| step.writes() || step.deletes())
        .count();
    if behind > 0 {
        found.push(Finding::needs(
            CLAUDE,
            Problem::Stale,
            format!(
                "{} not what the guild says",
                crate::render::format::count(behind, "file")
            ),
            "armada guild project",
        ));
    }

    if found.is_empty() {
        // **A guild with nothing to project says so rather than reporting
        // `ok`.** An empty guild and a projected one are different facts, and a
        // row claiming the second when it means the first is the row this whole
        // group was withheld to avoid.
        found.push(Finding::settled(
            CLAUDE,
            Settled::Ok,
            if steps.is_empty() {
                "nothing in the guild to project".to_string()
            } else {
                format!(
                    "{} current",
                    crate::render::format::count(steps.len(), "file")
                )
            },
        ));
    }
    found
}

/// The first couple of names and how many more there are.
///
/// **Names, not just a number.** A row saying `4 files` leaves the reader with
/// nowhere to go; two paths and a `+2` fit on the line and say where to start.
fn a_few(items: &[&str]) -> String {
    const KEEP: usize = 2;
    if items.len() <= KEEP {
        return items.join(", ");
    }
    format!("{}, +{}", items[..KEEP].join(", "), items.len() - KEEP)
}

/// `manifest.db` — its size, table by table, and how much of it is
/// reclaimable.
///
/// **Guild may not name Manifest** (`ARCHITECTURE.md` §1.9), and `doctor` is
/// Helm's, so this is the one place the two are looked at together. It reads
/// through [`armada_manifest::db::Db::peek_stats`] rather than opening its own
/// connection: the store has an owner, and this asks it a question rather than
/// reimplementing it.
///
/// **Presence is not health** (`docs/reserved/009-smaller-things-raised-in-use.md`
/// item 1). A store that exists and holds four thousand dead rows used to read
/// the same as an empty one: both said `present`. The table breakdown comes off
/// `sqlite_master`, so a table `db.rs::migrate` adds shows up here without this
/// function changing — the same fix item 5 made for `armada --help`'s drifted
/// list.
fn store(armada_home: &Path) -> Finding {
    let path = armada_home.join("manifest.db");
    let withheld = machine::read(armada_home).withheld.len();
    if !path.is_file() {
        return Finding::settled(
            "manifest.db",
            Settled::Ok,
            "not created yet: `armada manifest init` makes it",
        );
    }

    let Some(stats) = armada_manifest::db::Db::peek_stats(&path) else {
        // Present but unreadable — the same failure `peek_namespace` already
        // tolerates. `armada manifest` reports a broken store when it actually
        // opens one; doctor says only that this could not be read from outside.
        return Finding::settled("manifest.db", Settled::Ok, "present, could not be read");
    };

    let tables = stats
        .tables
        .iter()
        .map(|(name, n)| format!("{name} {n}"))
        .collect::<Vec<_>>()
        .join(", ");
    let reclaimable = stats.reclaimable_workspaces + stats.reclaimable_owned;
    let mut detail = format!(
        "{tables}; {}",
        crate::render::format::count(reclaimable, "reclaimable row")
    );
    if withheld > 0 {
        detail.push_str(&format!(
            ", {} withheld from the guild",
            crate::render::format::count(withheld, "value")
        ));
    }
    Finding::settled("manifest.db", Settled::Ok, detail)
}

/// The name both disk rows carry, so the render draws them as one check.
const DOCKER_DISK: &str = "docker disk";

/// The deadline on every docker call this check makes.
///
/// **The docker CLI has no client-side timeout** (`docs/traps.md`), and a
/// `doctor` that hung against a wedged daemon would be a `doctor` nobody runs —
/// which is worse than one that reports nothing about disk. Ten seconds is
/// generous against the 0.34s these took with 171 volumes on the machine this
/// was measured on.
const DISK_PROBE: std::time::Duration = std::time::Duration::from_secs(10);

/// How much disk Docker is holding, **and how much of it is Armada's**.
///
/// This check exists because of a specific event: a macOS storage warning, on a
/// machine holding 171 local volumes and 12.0 GB, 100% reclaimable. `doctor`
/// already reports `manifest.db`'s row counts and the guild's drift, and this is
/// the same shape of fact — something accumulating quietly that nothing else on
/// the machine would ever mention.
///
/// # Two rows, because there are two remedies
///
/// | Row | Whose | What to run |
/// |---|---|---|
/// | the machine's | everything Docker holds | `docker volume prune`, which is **the reader's to run** |
/// | Armada's | what carries an Armada owner label | `armada manifest clean --all` |
///
/// **They are never summed.** On the measured machine every one of those 171
/// volumes was somebody else's compose work and **none carried an Armada
/// label**, so a single "reclaimable" figure would have pointed the reader at
/// Armada's verb for a problem Armada did not cause — or, reversed, at
/// `docker volume prune` for containers a live workspace is still using.
///
/// # Neither row is ever a warning
///
/// **Docker being absent or not running is normal, not a fault.** Most machines
/// running `armada doctor` are not running Docker at that moment, and a check
/// that warned every time would make the command unsafe in a shell prompt — the
/// property the module header says `doctor` exists to keep.
///
/// **Reclaimable disk is not a warning either**, which is the less obvious half.
/// It is a fact about the machine rather than drift Armada can fix: `clean`
/// removes a workspace's resources when that workspace is *dead*, so a live
/// worktree's 2 GB is legitimately held and a row calling it a problem would be
/// wrong every day the reader is working. The precedent is [`store`] directly
/// above, which reports `manifest.db`'s reclaimable row count as `ok` and puts
/// the command in the detail — a settled row may not carry a `remedy`, and the
/// number is on screen either way, which is the whole of what was asked for.
///
/// # Ownership is read from the daemon, never from `df -v`'s label string
///
/// `docker system df -v` carries a `Labels` field, and reading ownership out of
/// it would save a call. It is a **comma-joined string** (`docs/traps.md`), and
/// this file's neighbouring rule is that a delimiter a value can contain is one
/// that eventually attributes a resource to the wrong owner. So ownership comes
/// from `docker volume ls --filter label=…`, which the daemon answers, and
/// `df -v` contributes only the size, matched by name.
fn docker_disk(runner: &impl Run, cwd: &Path) -> Vec<Finding> {
    let ask = |argv: Vec<String>| -> Option<String> {
        runner
            .call(&RunRequest::new(argv, cwd.to_path_buf()).timeout(DISK_PROBE))
            .ok()
            .filter(|output| output.ok() && !output.timed_out)
            .map(|output| output.stdout)
    };

    // **One JSON object per line, and the template rather than the columns.** A
    // renamed field makes this exit non-zero; a renamed column would shift a
    // whitespace split by one and report a count as a size.
    let Some(stdout) = ask(argv(&["docker", "system", "df", "--format", "{{json .}}"])) else {
        // Absent, not running, or wedged — all three are ordinary, and none of
        // them is this machine's problem to fix right now.
        return vec![Finding::settled(
            DOCKER_DISK,
            Settled::Ok,
            "docker did not answer, so there is nothing to report",
        )];
    };

    let usage = disk::parse_summary(&stdout);
    if usage.rows.is_empty() && usage.unrecognised.is_empty() {
        // It answered and said nothing Armada could read. **Reported as read
        // rather than as empty**, because "no disk in use" and "the output
        // changed shape" are different facts and only one of them is good news.
        return vec![Finding::settled(
            DOCKER_DISK,
            Settled::Ok,
            "docker answered in a shape armada could not read",
        )];
    }

    let mut rows = vec![Finding::settled(
        DOCKER_DISK,
        Settled::Ok,
        machine_detail(&usage),
    )];

    // Nothing on the machine means nothing to attribute, and two more docker
    // calls to prove it.
    let volumes = usage
        .row(disk::DiskKind::Volumes)
        .map_or(0, |row| row.total);
    if volumes == 0 {
        rows.push(Finding::settled(
            DOCKER_DISK,
            Settled::Ok,
            "armada holds no volumes",
        ));
        return rows;
    }

    // **Both label namespaces, one call each.** Repeating `--filter label=…`
    // ands the conditions, so asking for both in one call matches nothing —
    // the same measured rule `docker::list_labelled` is built on, and the
    // reason a pre-M1 volume stays attributable for one release.
    let mut ours: Vec<String> = Vec::new();
    let mut asked = false;
    for label in [docker::LABEL_WORKSPACE, docker::LEGACY_LABEL_WORKSPACE] {
        let listed = ask(argv(&[
            "docker",
            "volume",
            "ls",
            "-q",
            "--filter",
            &format!("label={label}"),
        ]));
        if let Some(listed) = listed {
            asked = true;
            for name in listed.lines().map(str::trim).filter(|l| !l.is_empty()) {
                if !ours.iter().any(|seen| seen == name) {
                    ours.push(name.to_string());
                }
            }
        }
    }

    let sizes = ask(argv(&[
        "docker",
        "system",
        "df",
        "-v",
        "--format",
        "{{json .}}",
    ]))
    .map(|stdout| disk::parse_verbose_volumes(&stdout));

    rows.push(match (asked, sizes) {
        (true, Some(sizes)) => {
            let share = disk::split(&sizes, &ours);
            Finding::settled(DOCKER_DISK, Settled::Ok, armada_detail(&share))
        }
        // The enumeration is the half that decides whose a volume is, so
        // without it Armada says it does not know rather than guessing zero —
        // "armada holds none of it" is exactly the reassuring sentence that
        // must never be printed on a failed lookup.
        _ => Finding::settled(
            DOCKER_DISK,
            Settled::Ok,
            "armada's own share could not be read",
        ),
    });
    rows
}

/// `docker`'s argv, as owned strings.
fn argv(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| word.to_string()).collect()
}

/// The machine's row: **the number, and nothing else**.
///
/// **The remedy is deliberately not on this row.** At the 80 columns the render
/// is frozen at, a detail carrying both the figures and a command truncates
/// mid-word — and the half that gets cut is the command, which is the half that
/// is worth reading. So this row is the fact and [`armada_detail`] below is the
/// attribution plus the verb, which is also the order the reader asks in: how
/// much, then whose, then what to run.
fn machine_detail(usage: &disk::DiskUsage) -> String {
    let held: Vec<String> = usage
        .rows
        .iter()
        .filter(|row| row.total > 0)
        .map(|row| crate::render::format::count(row.total as usize, row.kind.word()))
        .collect();
    let held = match held.is_empty() {
        true => "nothing".to_string(),
        false => held.join(", "),
    };
    let mut detail = format!(
        "{} reclaimable in {held}",
        disk::format_maybe(usage.reclaimable())
    );
    // **A type this version has no case for is disk being under-reported**, and
    // the reader is the only one who can judge whether it matters. It goes last
    // because it is the part that may safely be truncated: it names a thing to
    // go and look at rather than a thing to do.
    if !usage.unrecognised.is_empty() {
        detail.push_str(&format!(
            "; docker also reported {}, which armada does not know",
            usage.unrecognised.join(", ")
        ));
    }
    detail
}

/// Armada's row: **whose it is, and therefore which verb**.
fn armada_detail(share: &disk::Share) -> String {
    if share.armada_count == 0 {
        // **The measured case, and the one worth spelling out.** A machine full
        // of volumes none of which are Armada's should say so plainly and hand
        // the reader the verb that will actually help — which is not one of
        // Armada's.
        return "none of it is armada's; `docker volume prune` is yours".to_string();
    }
    format!(
        "armada holds {}, {}; `armada manifest clean --all`",
        crate::render::format::count(share.armada_count, "volume"),
        disk::format_maybe(share.armada_bytes)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use armada_core::ctx::{RunOutput, RunRequest, SpawnError};
    use armada_core::envelope::Health;
    use std::path::PathBuf;

    struct Answers {
        divergence: &'static str,
        remote: bool,
    }

    impl Run for Answers {
        fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
            let argv: Vec<&str> = request.argv.iter().map(String::as_str).collect();
            let (ok, out) = match argv.as_slice() {
                ["git", "--version"] => (true, "git version 2.51.0\n"),
                ["claude", "--version"] => (true, "2.0.14\n"),
                ["docker", "--version"] => (false, ""),
                ["git", "remote", ..] if self.remote => (true, "git@example.com:me/guild.git\n"),
                ["git", "remote", ..] => (false, ""),
                ["git", "rev-list", ..] => (true, self.divergence),
                _ => (true, ""),
            };
            Ok(RunOutput {
                code: Some(if ok { 0 } else { 1 }),
                signal: None,
                stdout: out.to_string(),
                stderr: String::new(),
                timed_out: false,
            })
        }
    }

    /// A `claude` that answers `--help` with `flags` and the argv probe with
    /// `complaint`.
    struct Claude {
        flags: String,
        complaint: &'static str,
    }

    impl Claude {
        /// One that behaves the way the installed binary does.
        fn healthy() -> Claude {
            Claude {
                flags: armada_core::fleet::drone::FLAGS.join(" "),
                complaint: "",
            }
        }
    }

    impl Run for Claude {
        fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
            let argv: Vec<&str> = request.argv.iter().map(String::as_str).collect();
            match argv.as_slice() {
                ["claude", "--help"] => Ok(RunOutput {
                    code: Some(0),
                    signal: None,
                    stdout: self.flags.clone(),
                    stderr: String::new(),
                    timed_out: false,
                }),
                _ => Ok(RunOutput {
                    code: Some(1),
                    signal: None,
                    stdout: String::new(),
                    stderr: format!("{}\n", self.complaint),
                    timed_out: false,
                }),
            }
        }
    }

    /// A `claude` that records what it was asked and says nothing back.
    struct Watching(std::cell::RefCell<Vec<Vec<String>>>);

    impl Run for Watching {
        fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
            self.0.borrow_mut().push(request.argv.clone());
            Ok(RunOutput {
                code: Some(0),
                signal: None,
                stdout: armada_core::fleet::drone::FLAGS.join(" "),
                stderr: String::new(),
                timed_out: false,
            })
        }
    }

    /// A `claude` whose `--help` offers `flags`, and whose `plugin validate`
    /// answers `plugin_ok`.
    struct Helm {
        flags: String,
        plugin_ok: bool,
        /// Every argv it was given, so a test can prove no session was ever
        /// asked for.
        seen: std::cell::RefCell<Vec<Vec<String>>>,
    }

    impl Helm {
        fn healthy() -> Helm {
            Helm {
                flags: armada_core::helm::FLAGS.join(" "),
                plugin_ok: true,
                seen: std::cell::RefCell::new(Vec::new()),
            }
        }

        /// A later `claude` that has dropped one flag Armada's launch uses —
        /// the failure the whole check exists for, one flag at a time.
        fn without(flag: &str) -> Helm {
            Helm {
                flags: armada_core::helm::FLAGS
                    .iter()
                    .filter(|offered| **offered != flag)
                    .copied()
                    .collect::<Vec<_>>()
                    .join(" "),
                ..Helm::healthy()
            }
        }
    }

    impl Run for Helm {
        fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
            self.seen.borrow_mut().push(request.argv.clone());
            let argv: Vec<&str> = request.argv.iter().map(String::as_str).collect();
            match argv.as_slice() {
                ["claude", "--help"] => Ok(RunOutput {
                    code: Some(0),
                    signal: None,
                    stdout: self.flags.clone(),
                    stderr: String::new(),
                    timed_out: false,
                }),
                ["claude", "plugin", "validate", ..] => Ok(RunOutput {
                    code: Some(if self.plugin_ok { 0 } else { 1 }),
                    signal: None,
                    stdout: String::new(),
                    stderr: match self.plugin_ok {
                        true => String::new(),
                        false => "✖ monitors/monitors.json: not an array\n".to_string(),
                    },
                    timed_out: false,
                }),
                _ => panic!("`doctor` asked `claude` for {argv:?}"),
            }
        }
    }

    /// A `~/.armada/helm/plugin` that `armada helm` has already written.
    fn a_wired_machine() -> tempfile::TempDir {
        let home = tempfile::tempdir().unwrap();
        let manifest = home
            .path()
            .join(armada_core::helm::DIRECTORY)
            .join("plugin/.claude-plugin");
        std::fs::create_dir_all(&manifest).unwrap();
        std::fs::write(
            manifest.join("plugin.json"),
            armada_core::helm::plugin_json(),
        )
        .unwrap();
        home
    }

    /// **The check that would have caught the Drone's missing `--verbose`,
    /// arriving at Helm's argv.** Every flag the launch uses is held against
    /// what the installed binary says it offers.
    #[test]
    fn a_claude_that_offers_every_flag_the_launch_uses_is_ok() {
        let run = Helm::healthy();
        let finding = helm_argv(&run, Path::new("/tmp"), Path::new("/nonexistent"));
        assert_eq!(finding.status, Health::Ok);
        assert!(finding.detail.contains("7 flags accepted"), "{finding:?}");
        // **`--append-system-prompt` is one of the seven, and this is the whole
        // of the preflight for it.** It is what carries the reader's own
        // `voice.md`, `expectations.md` and `how-i-work.md` into the session; a
        // release that renamed it would otherwise surface as a Helm that will
        // not start at all, in the one session the reader cannot ask Armada
        // about — because Helm is how they ask.
        assert!(
            armada_core::helm::FLAGS.contains(&armada_core::helm::APPEND),
            "the flag that carries the reader's voice is not checked at all"
        );
    }

    /// **A `claude` that has dropped `--append-system-prompt` is reported by
    /// name.** Asserting the argv proves Armada built the string it meant to;
    /// this is the half that proves the binary still accepts it, and it is the
    /// reason the flag was added to `helm::FLAGS` rather than only to the argv.
    #[test]
    fn a_claude_that_no_longer_appends_a_system_prompt_is_a_finding() {
        let run = Helm::without(armada_core::helm::APPEND);
        let finding = helm_argv(&run, Path::new("/tmp"), Path::new("/nonexistent"));
        assert_eq!(finding.status, Health::Missing);
        assert!(
            finding.detail.contains(armada_core::helm::APPEND),
            "{finding:?}"
        );
    }

    /// **`doctor` never opens a Helm session, and this is the assertion that
    /// says so.** The Drone's probe is free because a headless turn with closed
    /// stdin makes no API call; Helm is interactive and has no such probe, so
    /// the only things `claude` may ever be asked here are `--help` and `plugin
    /// validate` — neither of which starts a session or spends a token.
    #[test]
    fn checking_helm_never_asks_claude_to_start_anything() {
        let home = a_wired_machine();
        let run = Helm::healthy();
        helm_argv(&run, Path::new("/tmp"), home.path());

        let asked = run.seen.borrow().clone();
        assert_eq!(asked.len(), 2, "{asked:?}");
        for argv in &asked {
            assert!(
                argv.as_slice() == ["claude", "--help"]
                    || argv[..3] == ["claude", "plugin", "validate"],
                "`doctor` ran {argv:?}, which is not a free question"
            );
            for spending in ["--session-id", "--resume", "--print", "--agent"] {
                assert!(
                    !argv.iter().any(|word| word == spending),
                    "`doctor` would open a session: {argv:?}"
                );
            }
        }
    }

    /// A flag the launch needs going missing is the failure this exists for: a
    /// Helm that will not start reports as nothing at all, because Helm is how
    /// the reader would have asked.
    #[test]
    fn a_flag_helm_needs_going_missing_is_a_finding() {
        let run = Helm {
            flags: armada_core::helm::FLAGS
                .iter()
                .filter(|flag| **flag != "--mcp-config")
                .copied()
                .collect::<Vec<_>>()
                .join(" "),
            ..Helm::healthy()
        };
        let finding = helm_argv(&run, Path::new("/tmp"), Path::new("/nonexistent"));
        assert_eq!(finding.status, Health::Missing);
        assert!(finding.detail.contains("--mcp-config"), "{finding:?}");
    }

    /// **A monitor Claude Code will not load is quietly slower rather than
    /// broken**, because the `Stop` hook still backs it up — which is exactly the
    /// kind of failure that survives for months if nothing reports it.
    #[test]
    fn a_monitor_claude_code_would_not_load_is_a_finding() {
        let home = a_wired_machine();
        let run = Helm {
            plugin_ok: false,
            ..Helm::healthy()
        };
        let finding = helm_argv(&run, Path::new("/tmp"), home.path());
        assert_eq!(finding.status, Health::Missing);
        assert!(finding.detail.contains("monitors.json"), "{finding:?}");
    }

    /// **`doctor` says entering is off, and the row still fits its column.**
    /// Both halves are the assertion: a reader who saw only *"6 flags accepted"*
    /// would conclude `armada helm` opens a session, and a row that says so and
    /// then truncates mid-word has told them nothing either.
    #[test]
    fn the_healthy_row_says_the_launch_is_checked_and_entering_is_off() {
        let home = a_wired_machine();
        let run = Helm::healthy();
        let finding = helm_argv(&run, Path::new("/tmp"), home.path());

        assert_eq!(finding.status, Health::Ok);
        assert!(finding.detail.contains("monitor validates"), "{finding:?}");
        assert!(finding.detail.contains("entering is off"), "{finding:?}");
        // The `DETAIL` column a `doctor` row draws into. Measured against the
        // real render: the full reason truncates here, which is why this row
        // carries the state and sends the reader to `armada helm --help`.
        assert!(
            finding.detail.len() <= 64,
            "the row truncates: {} columns — {finding:?}",
            finding.detail.len()
        );
    }

    /// **`doctor` tells the truth once this machine has said yes.** The same
    /// row, the same column budget, and the one word that changed — which is
    /// the whole of what `armada helm enable` is supposed to move.
    #[test]
    fn the_row_says_entering_is_on_once_this_machine_has_enabled_it() {
        let home = a_wired_machine();
        crate::machine::set_enter(home.path(), true).unwrap();
        let run = Helm::healthy();
        let finding = helm_argv(&run, Path::new("/tmp"), home.path());

        assert_eq!(finding.status, Health::Ok);
        assert!(finding.detail.contains("entering is on"), "{finding:?}");
        assert!(
            finding.detail.len() <= 64,
            "the row truncates: {} columns — {finding:?}",
            finding.detail.len()
        );
    }

    /// A machine that has never run `armada helm` has no monitor to validate,
    /// and is told to run the verb rather than reported as a problem.
    #[test]
    fn a_machine_that_has_never_run_helm_is_ok_and_says_what_to_run() {
        let run = Helm::healthy();
        let finding = helm_argv(&run, Path::new("/tmp"), Path::new("/nonexistent"));
        assert_eq!(finding.status, Health::Ok);
        assert!(finding.detail.contains("armada helm"), "{finding:?}");
        assert_eq!(run.seen.borrow().len(), 1, "it validated a plugin anyway");
    }

    /// **The probe is what `doctor` runs, and it must be the Drone's own
    /// argv.** A probe that validated an approximation is a probe that passes on
    /// a combination Armada never uses — which is the failure this check exists
    /// to catch, reintroduced one level up.
    #[test]
    fn the_probe_runs_the_drones_own_argv_and_not_an_approximation() {
        let run = Watching(std::cell::RefCell::new(Vec::new()));
        drone_argv(&run, Path::new("/tmp"), Path::new("/tmp"));

        let probe = run.0.borrow().last().cloned().expect("a probe ran");
        // **The relay's own settings, on the real path this check writes**
        // (`020` §1). Passing `None` here would have let `--settings` vanish
        // from the probe while `doctor` went on reporting the argv checked.
        let registered = Path::new("/tmp").join(format!(
            "probe-{}.json",
            armada_core::fleet::drone::PROBE_SESSION
        ));
        // **And the MCP config, on the real path this check writes**, for the
        // same reason: a probe that lost `--mcp-config` would still be reported
        // as the argv checked, while the flag that decides whether a Drone can
        // report at all went unexercised.
        let attached = Path::new("/tmp").join(format!(
            "probe-{}.mcp.json",
            armada_core::fleet::drone::PROBE_SESSION
        ));
        assert_eq!(
            probe,
            armada_core::fleet::drone::probe_argv(
                Some(&registered.display().to_string()),
                Some(&attached.display().to_string()),
            ),
            "the probe drifted from the Drone's own argv"
        );
        assert!(
            std::fs::read_to_string(&registered).is_err(),
            "the probe left its scratch document behind"
        );
    }

    /// **The brief goes through the real validator, and the posture survives
    /// it.**
    ///
    /// AGENTS.md: *"asserting on argv proves you built the string you meant, not
    /// that it works"*, and this is the half that is not an argv assertion — the
    /// vector this machine's `claude` is actually handed carries the
    /// multi-kilobyte system prompt, so the answer covers the combination and
    /// not just the flag. `armada_core` already pins the flag and the bytes; the
    /// thing that could only fail *here* is `claude` refusing that value beside
    /// two variadic tool lists.
    #[test]
    fn the_probe_the_validator_receives_carries_the_brief_and_still_has_a_posture() {
        use armada_core::fleet::drone;

        let run = Watching(std::cell::RefCell::new(Vec::new()));
        drone_argv(&run, Path::new("/tmp"), Path::new("/tmp"));
        let probe = run.0.borrow().last().cloned().expect("a probe ran");

        let at = probe
            .iter()
            .position(|word| word == drone::APPEND)
            .expect("the validator never saw the brief");
        // **Both halves of the one appended prompt reach the validator**: the
        // reporting contract (`docs/reserved/019`) and Armada's own skill
        // (`docs/reserved/008`), in that order. Asserted as a prefix and a
        // suffix rather than an equality, because the flag is singular and the
        // two are joined into one value.
        assert!(probe[at + 1].starts_with(drone::BRIEF), "{probe:?}");
        assert!(
            probe[at + 1].ends_with(armada_core::skill::BODY),
            "the validator never saw Armada's own skill"
        );
        // The flag behind it survived, which is the collision this probe is
        // extended to cover: a swallowed `--permission-mode` is a Drone with no
        // posture, and `docs/reserved/011` is what that costs.
        assert_eq!(probe[at + 2], "--permission-mode", "{probe:?}");
        assert!(probe.iter().any(|word| word == "dontAsk"), "{probe:?}");
    }

    /// **A brief that went empty is caught before the binary is asked
    /// anything.** The argv would still be accepted — `--append-system-prompt`
    /// would take `--permission-mode` as its value and Claude Code would not
    /// complain — so no probe against the real validator can find this one.
    #[test]
    fn a_brief_that_would_swallow_the_posture_is_a_finding() {
        // The shape, asserted against the real constant: if this ever became
        // empty, the guard above would fire and the row would stop being green.
        assert!(
            !armada_core::fleet::drone::BRIEF.trim().is_empty(),
            "an empty brief would make every Drone posture-less"
        );
        assert!(!armada_core::fleet::drone::BRIEF.starts_with('-'));
        // And the guard is wired: a healthy `claude` with a real brief is Ok,
        // which is the only branch that reaches the flag check below it.
        assert_eq!(
            drone_argv(&Claude::healthy(), Path::new("/tmp"), Path::new("/tmp")).status,
            Health::Ok
        );
    }

    /// **It carries no prompt, which is the whole reason it costs nothing.**
    /// Claude Code is told to read messages from stdin and given none, so it
    /// starts, gets EOF and exits without an API call.
    #[test]
    fn the_argv_probe_has_nothing_to_say_and_so_cannot_spend_a_token() {
        let run = Watching(std::cell::RefCell::new(Vec::new()));
        drone_argv(&run, Path::new("/tmp"), Path::new("/tmp"));
        let probe = run.0.borrow().last().cloned().unwrap();

        assert_eq!(
            &probe[probe.len() - 2..],
            ["--input-format", "stream-json"],
            "without stream-json input the probe would answer a prompt"
        );
        let real = armada_core::fleet::drone::spawn_argv(
            armada_core::fleet::drone::PROBE_SESSION,
            "a prompt",
            &armada_core::fleet::drone::Posture::default(),
            Some("/s.json"),
            Some("/m.json"),
        );
        assert!(
            !probe.iter().any(|word| word == "a prompt"),
            "the probe carries a prompt, so it would run a turn"
        );
        assert_eq!(
            probe.len(),
            real.len() + 1,
            "the probe should be the real argv less its prompt, plus two words"
        );
    }

    #[test]
    fn a_claude_that_offers_every_flag_and_refuses_nothing_is_ok() {
        let finding = drone_argv(&Claude::healthy(), Path::new("/tmp"), Path::new("/tmp"));
        assert_eq!(finding.status, Health::Ok);
        assert!(finding.remedy.is_none());
    }

    /// **The general shape of the failure**: a new version renames or drops a
    /// flag, and every Job spawns into a Drone that dies on a usage error.
    #[test]
    fn a_flag_the_drone_needs_going_missing_is_a_finding() {
        let finding = drone_argv(
            &Claude {
                flags: "--session-id --resume --print --output-format --model".to_string(),
                ..Claude::healthy()
            },
            Path::new("/tmp"),
            Path::new("/tmp"),
        );
        assert_eq!(finding.status, Health::Missing);
        assert!(finding.detail.contains("--verbose"), "{}", finding.detail);
        assert!(finding.remedy.is_some(), "a finding without a remedy");
    }

    /// **The Drone's own `--append-system-prompt`, and this is the whole of the
    /// preflight for it** (`docs/reserved/008`).
    ///
    /// It carries Armada's own skill into every headless turn — *do not edit
    /// `armada.yml`, propose it* — and a `claude` that dropped the flag would
    /// take every Job with it, because the flag is in the argv unconditionally.
    /// Asserting the argv proves Armada built the string it meant to; this is
    /// the half that proves the binary still accepts it, which is the rule the
    /// `--verbose` trap wrote.
    #[test]
    fn a_claude_that_no_longer_appends_a_system_prompt_takes_every_drone_with_it() {
        assert!(
            armada_core::fleet::drone::FLAGS.contains(&armada_core::fleet::drone::APPEND),
            "the flag that carries Armada's own skill is not checked at all"
        );
        let finding = drone_argv(
            &Claude {
                flags: armada_core::fleet::drone::FLAGS
                    .iter()
                    .filter(|flag| **flag != armada_core::fleet::drone::APPEND)
                    .copied()
                    .collect::<Vec<_>>()
                    .join(" "),
                ..Claude::healthy()
            },
            Path::new("/tmp"),
            Path::new("/tmp"),
        );
        assert_eq!(finding.status, Health::Missing);
        assert!(
            finding.detail.contains(armada_core::fleet::drone::APPEND),
            "{}",
            finding.detail
        );
        // Inverted once: the same probe against a `claude` that still offers it
        // is `Ok`, so the finding is about the missing flag rather than about
        // the fixture.
        assert_eq!(
            drone_argv(&Claude::healthy(), Path::new("/tmp"), Path::new("/tmp")).status,
            Health::Ok
        );
    }

    /// **The specific failure this check was added after.** The flags all exist
    /// and the combination is refused; only asking the validator finds it.
    #[test]
    fn a_combination_the_cli_refuses_is_a_finding_even_when_every_flag_exists() {
        let finding = drone_argv(
            &Claude {
                complaint:
                    "Error: When using --print, --output-format=stream-json requires --verbose",
                ..Claude::healthy()
            },
            Path::new("/tmp"),
            Path::new("/tmp"),
        );
        assert_eq!(finding.status, Health::Missing);
        assert!(finding.detail.contains("--verbose"), "{}", finding.detail);
    }

    /// **A probe that ran a turn is a finding, because it must never cost
    /// anything.** No `result` event is emitted when there is no input to
    /// answer; one appearing means a future version began answering an empty
    /// conversation, and a diagnostic that quietly started spending would be
    /// worse than one that does not run.
    #[test]
    fn a_probe_that_started_a_turn_is_reported_rather_than_paid_for() {
        struct Spending;
        impl Run for Spending {
            fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
                let helping = request.argv.iter().any(|word| word == "--help");
                Ok(RunOutput {
                    code: Some(0),
                    signal: None,
                    stdout: match helping {
                        true => armada_core::fleet::drone::FLAGS.join(" "),
                        false => r#"{"type":"result","total_cost_usd":0.02}"#.to_string(),
                    },
                    stderr: String::new(),
                    timed_out: false,
                })
            }
        }
        let finding = drone_argv(&Spending, Path::new("/tmp"), Path::new("/tmp"));
        assert_eq!(finding.status, Health::Partial, "a spend was reported ok");
        assert!(
            finding.detail.contains("cost nothing"),
            "{}",
            finding.detail
        );
        // A warning rather than a failure: the flags are fine, and `doctor` has
        // to stay safe to run in a shell prompt.
        assert!(!finding.status.is_failure());
    }

    /// A `claude` that will not answer at all degrades to a warning: the tooling
    /// check above already reports it, and saying it twice is noise.
    #[test]
    fn a_claude_that_will_not_answer_help_degrades_rather_than_failing() {
        struct Absent;
        impl Run for Absent {
            fn call(&self, _: &RunRequest) -> Result<RunOutput, SpawnError> {
                Err(SpawnError {
                    program: "claude".to_string(),
                    kind: armada_core::ctx::SpawnErrorKind::NotFound,
                    message: "No such file or directory".to_string(),
                })
            }
        }
        let finding = drone_argv(&Absent, Path::new("/tmp"), Path::new("/tmp"));
        assert_eq!(finding.status, Health::Offline);
        assert!(
            !finding.status.is_failure(),
            "a missing claude failed twice"
        );
    }

    fn machine_with_a_guild() -> (tempfile::TempDir, Where) {
        let home = tempfile::tempdir().unwrap();
        let armada_home = home.path().join(".armada");
        for directory in DIRECTORIES {
            std::fs::create_dir_all(armada_home.join(directory)).unwrap();
        }
        std::fs::create_dir_all(armada_home.join("guild/.git")).unwrap();
        for fragment in memory::FRAGMENTS {
            std::fs::write(armada_home.join("guild").join(fragment), "mine\n").unwrap();
        }
        let place = Where {
            armada_home,
            cwd: home.path().to_path_buf(),
            claude_home: home.path().join(".claude"),
        };
        (home, place)
    }

    fn data(output: &Output) -> DoctorData {
        match output {
            Output::Doctor(envelope) => envelope.data.clone(),
            _ => panic!("not a doctor"),
        }
    }

    fn find<'a>(data: &'a DoctorData, check: &str, detail: &str) -> Option<&'a Finding> {
        data.results
            .iter()
            .find(|row| row.check == check && row.detail.contains(detail))
    }

    /// **A guild that has not been projected is reported, and the row names the
    /// command.** This is the group `doctor.md` reserved and could not report
    /// until a projector existed.
    #[test]
    fn a_guild_that_is_not_on_claude_codes_load_path_is_stale() {
        let (home, place) = machine_with_a_guild();
        std::fs::create_dir_all(home.path().join(".armada/guild/skills/onboard-repo")).unwrap();
        std::fs::write(
            home.path()
                .join(".armada/guild/skills/onboard-repo/SKILL.md"),
            "# onboard\n",
        )
        .unwrap();

        let findings = projection(&place, &place.guild());
        let stale = findings
            .iter()
            .find(|row| row.check == CLAUDE)
            .expect("no projection row");
        assert_eq!(stale.status, Health::Stale);
        assert!(stale.detail.contains("1 file"), "{}", stale.detail);
        assert_eq!(stale.remedy.as_deref(), Some("armada guild project"));
        assert!(
            !place.claude_home.exists(),
            "the survey wrote something, and `doctor` is read-only"
        );
    }

    /// **A file you edited is reported as yours, not as out of date**, and it is
    /// the first row, because it is the one only a person can decide about.
    #[test]
    fn a_file_you_edited_is_reported_as_yours() {
        let (home, place) = machine_with_a_guild();
        let guild = place.guild();
        std::fs::create_dir_all(guild.path("subagents")).unwrap();
        std::fs::write(guild.path("subagents/helm.md"), "# helm\n").unwrap();
        armada_guild::projector::project(&guild, &place.claude_home, &place.armada_home).unwrap();
        std::fs::write(place.claude_home.join("agents/helm.md"), "# mine\n").unwrap();
        let _ = home;

        let findings = projection(&place, &guild);
        assert_eq!(findings[0].status, Health::Partial);
        assert!(
            findings[0].detail.contains("agents/helm.md"),
            "{}",
            findings[0].detail
        );
        assert!(findings[0].remedy.is_some());
    }

    /// **A guild with nothing in it says so rather than reporting `ok`.** An
    /// empty guild and a projected one are different facts, and a row claiming
    /// the second when it means the first is what this group was withheld to
    /// avoid.
    #[test]
    fn a_guild_with_nothing_to_project_says_that_rather_than_ok() {
        let (_home, place) = machine_with_a_guild();
        let findings = projection(&place, &place.guild());
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].status, Health::Ok);
        assert!(findings[0].detail.contains("nothing in the guild"));
    }

    /// **No row at all when there is no guild.** The group above has already
    /// said so, and a second row saying it again is one a reader has to read
    /// twice to learn nothing.
    #[test]
    fn a_machine_with_no_guild_gets_no_projection_row() {
        let home = tempfile::tempdir().unwrap();
        let place = Where {
            armada_home: home.path().join(".armada"),
            cwd: home.path().to_path_buf(),
            claude_home: home.path().join(".claude"),
        };
        assert!(projection(&place, &place.guild()).is_empty());
    }

    /// **The check that earns the command**, with the remedy that fixes it.
    #[test]
    fn a_guild_behind_its_remote_is_stale_and_names_the_command_that_fixes_it() {
        let (_home, place) = machine_with_a_guild();
        let output = run(
            &Answers {
                divergence: "3\t0\n",
                remote: true,
            },
            &place,
        )
        .unwrap();
        let data = data(&output);
        let stale = find(&data, "guild", "3 commits behind origin").expect("no drift row");
        assert_eq!(stale.status, Health::Stale);
        assert_eq!(stale.remedy.as_deref(), Some("armada guild pull"));
    }

    /// **A warning alone does not fail**, so `doctor` stays safe in a shell
    /// prompt — and `missing` is what does fail.
    #[test]
    fn drift_warns_and_a_missing_tool_fails() {
        let (_home, place) = machine_with_a_guild();
        let output = run(
            &Answers {
                divergence: "3\t0\n",
                remote: true,
            },
            &place,
        )
        .unwrap();
        // `docker` is missing in this fixture, so the run does fail — and it is
        // the missing tool that did it, not the stale guild.
        assert_eq!(output.exit_code(), 0, "doctor answers; it does not gate");
        let data = data(&output);
        assert!(
            data.tally.iter().any(|f| f.contains("missing")),
            "{:?}",
            data.tally
        );
        assert!(
            data.tally.iter().any(|f| f.contains("warning")),
            "{:?}",
            data.tally
        );
        assert_eq!(
            DoctorData::verdict(&data.results),
            armada_core::error::Status::Failed
        );
    }

    /// **Sync off is not a problem.** It is the documented default and `export`
    /// still works; warning about it daily would train the reader to skim.
    #[test]
    fn a_guild_with_no_remote_is_ok_rather_than_a_warning() {
        let (_home, place) = machine_with_a_guild();
        let data = data(
            &run(
                &Answers {
                    divergence: "0\t0\n",
                    remote: false,
                },
                &place,
            )
            .unwrap(),
        );
        let row = find(&data, "guild", "no remote").expect("no guild row");
        assert_eq!(row.status, Health::Ok);
    }

    /// **The half of `--defaults` that has to be reported.** Without this, a
    /// skipped interview finishes silently in a state that looks configured and
    /// is not.
    #[test]
    fn a_fragment_still_as_imported_is_reported_by_name() {
        let (_home, place) = machine_with_a_guild();
        std::fs::write(
            place.guild().path("voice.md"),
            format!("<!-- {} imported -->\n", memory::MARKER),
        )
        .unwrap();
        let data = data(
            &run(
                &Answers {
                    divergence: "0\t0\n",
                    remote: false,
                },
                &place,
            )
            .unwrap(),
        );
        let row = find(&data, "guild", "voice.md still as imported").expect("no fragment row");
        assert_eq!(row.status, Health::Partial);
        assert!(
            find(&data, "guild", "expectations.md still as imported").is_none(),
            "a fragment the person wrote was reported as still imported"
        );
    }

    /// A machine that has never run `armada init` is told the one thing that
    /// helps.
    #[test]
    fn a_machine_with_no_armada_home_says_to_run_init() {
        let home = tempfile::tempdir().unwrap();
        let place = Where {
            armada_home: home.path().join(".armada"),
            cwd: home.path().to_path_buf(),
            claude_home: home.path().join(".claude"),
        };
        let data = data(
            &run(
                &Answers {
                    divergence: "0\t0\n",
                    remote: false,
                },
                &place,
            )
            .unwrap(),
        );
        let row = find(&data, "~/.armada", "is not there").expect("no directories row");
        assert_eq!(row.status, Health::Missing);
        assert_eq!(row.remedy.as_deref(), Some("armada init"));
    }

    /// **`layout` and `no jobs, workspaces` said nothing a reader could use.**
    /// The check is named for the thing it checks, and the detail names the
    /// paths and what writes to them.
    #[test]
    fn a_missing_directory_is_named_as_a_path_and_says_what_writes_there() {
        let (_home, place) = machine_with_a_guild();
        std::fs::remove_dir_all(place.armada_home.join("jobs")).unwrap();
        std::fs::remove_dir_all(place.armada_home.join("workspaces")).unwrap();
        let data = data(
            &run(
                &Answers {
                    divergence: "0\t0\n",
                    remote: false,
                },
                &place,
            )
            .unwrap(),
        );
        let row = find(&data, "~/.armada", "missing").expect("no directories row");
        assert_eq!(
            row.detail,
            "jobs/ and workspaces/ are missing; Jobs and worktrees go there"
        );
        assert_eq!(row.remedy.as_deref(), Some("armada init --force"));
    }

    /// **A `→` line for every row that asks the reader to do something**, not
    /// only for the missing ones. `missing ~/.armada` and both `partial guild`
    /// rows reached a real reader with nothing to act on, which is the failure
    /// `Finding::needs` now makes unrepresentable — this holds it against a real
    /// pass as well as against the type.
    #[test]
    fn every_row_that_needs_action_carries_the_fix_for_it() {
        let (_home, place) = machine_with_a_guild();
        std::fs::remove_dir_all(place.armada_home.join("jobs")).unwrap();
        for fragment in memory::FRAGMENTS {
            std::fs::write(
                place.guild().path(fragment),
                format!("<!-- {} imported -->\n", memory::MARKER),
            )
            .unwrap();
        }
        let data = data(
            &run(
                &Answers {
                    divergence: "3\t0\n",
                    remote: true,
                },
                &place,
            )
            .unwrap(),
        );
        let needy = data
            .results
            .iter()
            .filter(|row| row.status.needs_action())
            .count();
        assert!(needy >= 5, "the case did not produce problems: {data:?}");
        for row in &data.results {
            assert_eq!(
                row.status.needs_action(),
                row.remedy.is_some(),
                "`{}` reports {:?} with remedy {:?}",
                row.check,
                row.status,
                row.remedy
            );
        }
    }

    /// The directories row names `~/.armada` and never a real home directory.
    #[test]
    fn no_row_carries_a_real_home_directory() {
        let (_home, place) = machine_with_a_guild();
        let data = data(
            &run(
                &Answers {
                    divergence: "0\t0\n",
                    remote: false,
                },
                &place,
            )
            .unwrap(),
        );
        let row = find(&data, "~/.armada", "all present").expect("no directories row");
        assert_eq!(row.detail, "guild/, jobs/ and workspaces/, all present");
        let _ = PathBuf::new();
    }

    /// **Presence is not health.** A fresh `manifest.db` used to read as
    /// `present`, indistinguishable from a store nobody had looked inside. Now
    /// the row names every table `sqlite_master` has, so a table added later
    /// shows up without this file changing.
    #[test]
    fn manifest_db_present_reports_a_table_by_table_count() {
        let home = tempfile::tempdir().unwrap();
        armada_manifest::db::Db::open(home.path()).unwrap();

        let finding = store(home.path());
        assert_eq!(finding.status, Health::Ok);
        assert!(finding.detail.contains("meta 1"), "{}", finding.detail);
        assert!(
            finding.detail.contains("workspaces 0"),
            "{}",
            finding.detail
        );
        assert!(finding.detail.contains("owned 0"), "{}", finding.detail);
        assert!(finding.detail.contains("leases 0"), "{}", finding.detail);
        assert!(
            finding.detail.contains("0 reclaimable rows"),
            "{}",
            finding.detail
        );
    }

    /// **The fact worth surfacing**: the ownership store's whole purpose is
    /// reclaiming what is no longer owned, and this is what tells a reader how
    /// much of it is stale. A workspace whose directory is gone, and what it
    /// owns, both count.
    #[test]
    fn manifest_db_counts_a_gone_workspace_and_what_it_owns_as_reclaimable() {
        let home = tempfile::tempdir().unwrap();
        let mut db = armada_manifest::db::Db::open(home.path()).unwrap();
        db.claim_block(
            &armada_core::id::WorkspaceId::from_stored("aaaaaaaa"),
            &home.path().join("gone"),
            None,
            None,
            armada_core::ports::PORT_BASE,
            "t",
        )
        .unwrap();
        db.record_owned(&armada_core::registry::OwnedRow {
            workspace: armada_core::id::WorkspaceId::from_stored("aaaaaaaa"),
            kind: armada_core::registry::OwnedKind::Container,
            reference: "c1".to_string(),
            boot_id: None,
            pid_started_at: None,
            component: None,
        })
        .unwrap();

        let finding = store(home.path());
        assert!(
            finding.detail.contains("2 reclaimable rows"),
            "{}",
            finding.detail
        );
    }

    /// `a`, `a and b`, `a, b and c` — the shape every detail here is built from.
    #[test]
    fn a_list_is_written_the_way_a_sentence_writes_one() {
        assert_eq!(written(&["a"]), "a");
        assert_eq!(written(&["a", "b"]), "a and b");
        assert_eq!(written(&["a", "b", "c"]), "a, b and c");
        let none: [&str; 0] = [];
        assert_eq!(written(&none), "");
    }

    // ------------------------------------------------------------ docker disk

    /// A docker daemon that answers whatever the test wants it to, and records
    /// what it was asked.
    ///
    /// **Every reply here is real output** measured on darwin against Docker
    /// 29.6.2 — the 171 volumes and 12.01GB are the machine this check was
    /// written for. A fake carrying invented output would prove only that the
    /// parser reads the fake.
    struct Daemon {
        summary: Option<&'static str>,
        owned: Option<&'static str>,
        verbose: Option<&'static str>,
        seen: std::cell::RefCell<Vec<Vec<String>>>,
    }

    impl Daemon {
        fn measured() -> Daemon {
            Daemon {
                summary: Some(MEASURED_SUMMARY),
                // The measured machine: every one of the 171 was somebody
                // else's compose work, and none carried an Armada label.
                owned: Some(""),
                verbose: Some(MEASURED_VERBOSE),
                seen: std::cell::RefCell::new(Vec::new()),
            }
        }
    }

    impl Run for Daemon {
        fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
            self.seen.borrow_mut().push(request.argv.clone());
            let argv: Vec<&str> = request.argv.iter().map(String::as_str).collect();
            let reply = match argv.as_slice() {
                ["docker", "system", "df", "-v", ..] => self.verbose,
                ["docker", "system", "df", ..] => self.summary,
                ["docker", "volume", "ls", ..] => self.owned,
                _ => Some(""),
            };
            Ok(RunOutput {
                code: Some(if reply.is_some() { 0 } else { 1 }),
                signal: None,
                stdout: reply.unwrap_or_default().to_string(),
                stderr: String::new(),
                timed_out: false,
            })
        }
    }

    const MEASURED_SUMMARY: &str = concat!(
        r#"{"Active":"0","Reclaimable":"4.171MB (100%)","Size":"4.171MB","TotalCount":"1","Type":"Images"}"#,
        "\n",
        r#"{"Active":"0","Reclaimable":"0B","Size":"0B","TotalCount":"0","Type":"Containers"}"#,
        "\n",
        r#"{"Active":"0","Reclaimable":"12.01GB (100%)","Size":"12.01GB","TotalCount":"171","Type":"Local Volumes"}"#,
        "\n",
        r#"{"Active":"0","Reclaimable":"0B","Size":"0B","TotalCount":"0","Type":"Build Cache"}"#,
    );

    const MEASURED_VERBOSE: &str = concat!(
        r#"{"Images":[],"Containers":[],"BuildCache":[],"Volumes":["#,
        r#"{"Name":"someone-elses_pgdata","Size":"12.01GB"},"#,
        r#"{"Name":"armada-a3f91c02_pgdata","Size":"79.02MB"}]}"#,
    );

    fn details(runner: &impl Run) -> Vec<String> {
        docker_disk(runner, Path::new("/srv/repo"))
            .into_iter()
            .map(|finding| finding.detail)
            .collect()
    }

    /// **The measured machine, and the row he would have seen.** 171 volumes,
    /// 12.0 GB reclaimable — and the second row saying none of it is Armada's,
    /// which is what sends him to the right verb.
    #[test]
    fn the_measured_machine_reports_its_numbers_and_says_none_of_it_is_armadas() {
        let rows = details(&Daemon::measured());
        assert_eq!(rows.len(), 2, "{rows:#?}");
        assert!(rows[0].contains("171 volumes"), "{}", rows[0]);
        assert!(rows[0].contains("12.0 GB reclaimable"), "{}", rows[0]);
        assert!(rows[1].contains("none of it is armada's"), "{}", rows[1]);
        assert!(rows[1].contains("docker volume prune"), "{}", rows[1]);
    }

    /// **The two remedies never appear on one row**, because acting on the wrong
    /// one either deletes somebody's database or leaves the disk full.
    #[test]
    fn the_two_remedies_never_appear_on_one_row() {
        let rows = details(&Daemon::measured());
        assert!(!rows[0].contains("armada manifest clean"), "{}", rows[0]);
        assert!(!rows[0].contains("docker volume prune"), "{}", rows[0]);
        assert!(!rows[1].contains("armada manifest clean"), "{}", rows[1]);
    }

    /// Armada holding something says so, with the verb that releases what is
    /// dead — the inverse of the measured case above.
    #[test]
    fn a_volume_armada_owns_is_attributed_to_armada_with_its_own_remedy() {
        let daemon = Daemon {
            owned: Some("armada-a3f91c02_pgdata\n"),
            ..Daemon::measured()
        };
        let rows = details(&daemon);
        assert!(rows[1].contains("1 volume"), "{}", rows[1]);
        assert!(rows[1].contains("79.0 MB"), "{}", rows[1]);
        assert!(
            rows[1].contains("armada manifest clean --all"),
            "{}",
            rows[1]
        );
    }

    /// **Docker being absent or not running is normal, not a fault.** One row,
    /// `ok`, and `doctor` still passes — a check that warned every run on a
    /// machine not running Docker would make the command unusable in a prompt.
    #[test]
    fn a_docker_that_does_not_answer_is_one_ok_row_and_never_a_warning() {
        let daemon = Daemon {
            summary: None,
            ..Daemon::measured()
        };
        let rows = docker_disk(&daemon, Path::new("/srv/repo"));
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].status, Health::Ok);
        assert!(rows[0].detail.contains("did not answer"), "{rows:#?}");
        assert_eq!(DoctorData::verdict(&rows), armada_core::error::Status::Ok);
    }

    /// The whole check is read-only. **Inverted once**: if any argv here ever
    /// mutates, this is the test that says so.
    #[test]
    fn nothing_this_check_runs_can_remove_anything() {
        let daemon = Daemon::measured();
        docker_disk(&daemon, Path::new("/srv/repo"));
        for argv in daemon.seen.borrow().iter() {
            assert!(
                !argv.iter().any(|word| matches!(
                    word.as_str(),
                    "rm" | "prune" | "remove" | "-f" | "--force" | "down"
                )),
                "doctor must never mutate: {argv:?}"
            );
        }
    }

    /// **Both namespaces, one call each.** Repeating `--filter label=…` ands the
    /// conditions, so a single call asking for both matches nothing — and a
    /// pre-M1 volume would be silently attributed to nobody.
    #[test]
    fn ownership_is_asked_in_both_label_namespaces() {
        let daemon = Daemon::measured();
        docker_disk(&daemon, Path::new("/srv/repo"));
        let filters: Vec<String> = daemon
            .seen
            .borrow()
            .iter()
            .filter(|argv| argv.get(1).map(String::as_str) == Some("volume"))
            .filter_map(|argv| argv.last().cloned())
            .collect();
        assert_eq!(
            filters,
            vec![
                "label=armada.workspace".to_string(),
                "label=char.workspace".to_string()
            ]
        );
    }

    /// **Ownership is never read out of `df -v`'s label string.** It is
    /// comma-joined, and a delimiter a value can contain eventually attributes a
    /// volume to the wrong owner — so the daemon is asked instead.
    #[test]
    fn ownership_comes_from_the_daemon_and_not_from_the_verbose_label_field() {
        // `df -v` claims the volume is Armada's; the daemon says it is not.
        let daemon = Daemon {
            verbose: Some(concat!(
                r#"{"Volumes":[{"Name":"someone-elses_pgdata","Size":"12.01GB","#,
                r#""Labels":"armada.workspace=a3f91c02"}]}"#,
            )),
            owned: Some(""),
            ..Daemon::measured()
        };
        assert!(details(&daemon)[1].contains("none of it is armada's"));
    }

    /// **The reassuring sentence must never be printed on a failed lookup.** A
    /// lookup that did not happen is *unknown*, and reporting it as "none of it
    /// is armada's" would be a claim nothing checked.
    #[test]
    fn an_ownership_lookup_that_failed_says_so_rather_than_claiming_none() {
        let daemon = Daemon {
            owned: None,
            ..Daemon::measured()
        };
        let rows = details(&daemon);
        assert_eq!(rows[1], "armada's own share could not be read");
        assert!(!rows[1].contains("none of it is armada's"));
    }

    /// A machine with no volumes at all does not spend two more docker calls
    /// proving Armada owns none of them.
    #[test]
    fn a_machine_with_no_volumes_asks_nothing_further() {
        let daemon = Daemon {
            summary: Some(
                r#"{"Active":"0","Reclaimable":"0B","Size":"0B","TotalCount":"0","Type":"Local Volumes"}"#,
            ),
            ..Daemon::measured()
        };
        let rows = details(&daemon);
        assert_eq!(rows[1], "armada holds no volumes");
        assert!(!daemon
            .seen
            .borrow()
            .iter()
            .any(|argv| argv.get(1).map(String::as_str) == Some("volume")));
    }

    /// **A type this version has no case for is disk being under-reported.**
    /// Naming it is the only way the reader can judge whether it matters.
    #[test]
    fn a_type_docker_grew_since_this_shipped_is_named_on_the_row() {
        let daemon = Daemon {
            summary: Some(concat!(
                r#"{"Active":"0","Reclaimable":"0B","Size":"0B","TotalCount":"0","Type":"Local Volumes"}"#,
                "\n",
                r#"{"Active":"0","Reclaimable":"1GB","Size":"1GB","TotalCount":"3","Type":"Snapshots"}"#,
            )),
            ..Daemon::measured()
        };
        assert!(
            details(&daemon)[0].contains("Snapshots"),
            "{:#?}",
            details(&daemon)
        );
    }

    /// Output that changed shape entirely reads as *could not be read*, never as
    /// an empty machine — one of those is good news and the other is not.
    #[test]
    fn output_in_a_shape_armada_cannot_read_is_not_reported_as_an_empty_machine() {
        let daemon = Daemon {
            summary: Some("TYPE  TOTAL  ACTIVE  SIZE  RECLAIMABLE\n"),
            ..Daemon::measured()
        };
        let rows = details(&daemon);
        assert_eq!(rows.len(), 1);
        assert!(rows[0].contains("could not read"), "{}", rows[0]);
    }

    /// Every docker call this check makes carries Armada's own deadline. The
    /// docker CLI has none of its own, and a `doctor` that hangs is one nobody
    /// runs.
    #[test]
    fn every_docker_call_this_check_makes_has_a_deadline() {
        struct Deadlines(std::cell::RefCell<Vec<Option<std::time::Duration>>>);
        impl Run for Deadlines {
            fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
                self.0.borrow_mut().push(request.timeout);
                Ok(RunOutput {
                    code: Some(0),
                    signal: None,
                    stdout: MEASURED_SUMMARY.to_string(),
                    stderr: String::new(),
                    timed_out: false,
                })
            }
        }
        let runner = Deadlines(std::cell::RefCell::new(Vec::new()));
        docker_disk(&runner, Path::new("/srv/repo"));
        let seen = runner.0.borrow();
        assert!(!seen.is_empty());
        assert!(seen.iter().all(|t| *t == Some(DISK_PROBE)), "{seen:?}");
    }
}
