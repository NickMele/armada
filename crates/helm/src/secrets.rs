//! Secret resolution, **the half that holds a value** — and therefore the only
//! place in Armada that does.
//!
//! [`armada_core::secrets`] decides *which* provider answers *which* reference
//! and with what argv. This runs it, keeps what it printed for the lifetime of
//! one process, injects it into the children that were granted it, and scrubs it
//! out of everything Armada writes. `ARCHITECTURE.md` §1.8's *"resolution happens
//! in the shell"* is this file.
//!
//! # Where resolution happens, which is the whole of `013`
//!
//! **In the parent, before the run detaches** (`PLAN.md` §4.7,
//! [`docs/reserved/013`](../../../docs/reserved/013-secrets-must-resolve-before-the-run-detaches.md)).
//! `armada manifest check --detach` hands the run to a `setsid`'d child that has
//! no terminal, so a provider invoked there cannot prompt: `op` would wait on a
//! biometric nobody is looking at and the run would hang until its ceiling
//! rather than fail. The obvious design — resolve in the run loop, at
//! `Action::Spawn`, where the value is needed — is correct for an attached run,
//! wrong for a detached one, and **fails only in the detached case**, which is
//! the shape that survives review and ships.
//!
//! So `check` resolves once, in the caller's terminal, whether or not it is
//! about to detach, and the detached child never invokes a provider at all
//! ([`inherit`] is what it does instead).
//!
//! # How a value reaches the detached child without touching disk
//!
//! **Down the child's stdin, written and closed before this process exits.**
//! `PLAN.md` §4.7 rule 5 rules out the two easier answers and this is the third:
//!
//! | channel | why not |
//! |---|---|
//! | a file the child reads | `ARCHITECTURE.md` §1.8 — a plaintext token in a file Armada created is the `.env` file §4.7 exists to eliminate. It is also readable for as long as it exists, by anyone, and a crash between write and unlink leaves it forever |
//! | Armada's own environment | §4.5 inherits the parent environment wholesale to every child, so this grants **every** secret to **every** child and voids per-entry grants entirely. `ps` exposes it on some platforms |
//! | argv | world-readable through `ps`, and §4.7 forbids it outright |
//! | **the child's stdin** | a pipe pair that exists between two processes and nowhere else; the write end closes before this process returns, so the child sees EOF and no third party ever holds either end |
//!
//! The mechanism is already there: [`armada_core::ctx::RunRequest::with_stdin`]
//! exists for the compose driver, for precisely this reason — *"the transformed
//! document goes on stdin so it is never written to disk"* — and
//! `ProcessGroup::spawn` writes it and drops the handle before returning.
//! Nothing new is unsafe, no descriptor is inherited past the `exec`, and the
//! detached child's own children get `Stdio::null()` on stdin because their
//! requests carry no `stdin`, so the payload cannot travel a second hop.
//!
//! **The one residual, stated rather than papered over.** A pipe holds about
//! 64 KiB before a write blocks, so a parent whose payload exceeded that would
//! wait for the child to start reading. The child reads it as the first thing it
//! does, and secrets are not 64 KiB, so this is a bound rather than a hazard —
//! but it is why the read happens before the child does any other work.
//!
//! # What a resolved value is allowed to touch
//!
//! One thing: the environment of a child whose own `secrets:` named it
//! ([`Vault::granted`]). Everything else Armada emits about the run goes through
//! [`Vault::mask`] first, and the three records that capture argv and
//! environment — the run's log files, `~/.armada/failures.jsonl` and the ring
//! buffer — never receive one in the first place, because the dispatch record
//! writes [`armada_core::schedule::EnvDelta::names`] and the argv never carried
//! it. `crates/helm/tests/secrets.rs` plants a value and greps all of them.

use armada_core::ctx::{Run, RunRequest, SpawnErrorKind, StdioMode};
use armada_core::error::{ArmadaError, ErrClass};
use armada_core::secrets::Call;
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

/// How long one provider gets before Armada gives up on it.
///
/// **Generous, because the expensive providers are the interactive ones.** `op`
/// against a locked vault waits for a person to find a hardware key, and a
/// ceiling tight enough to be brisk for `cat` would turn 1Password into an
/// intermittent failure — which is the exact outcome §4.7 exists to avoid. Two
/// minutes is long enough for a person and still an answer rather than a hang,
/// which is the property `013` asks for: *"a clear error naming which secret and
/// which provider — not a hang."*
pub const PROVIDER_TIMEOUT_MS: u64 = 120_000;

/// The resolved secrets of one run, in memory, for the lifetime of one process.
///
/// **`PLAN.md` §4.7: never cached to disk, always cached in memory.** A run
/// granting the same secret to twenty checks resolves it once — that is
/// [`armada_core::secrets::plan`]'s doing, since it plans over a set — and this
/// holds the answers until the process exits, at which point they are gone with
/// it.
///
/// **No `Serialize`, and a `Debug` that prints names.** The derives are the
/// leak: `#[derive(Debug)]` puts every value into any `{:?}` anyone ever writes,
/// including one inside an error message, and `Serialize` puts them in `--json`
/// the first time this type is nested in a payload by accident. The one way a
/// value leaves this type deliberately is [`Vault::handoff`], which is named for
/// what it is and has exactly one caller.
#[derive(Default, Clone)]
pub struct Vault {
    values: BTreeMap<String, String>,
}

impl std::fmt::Debug for Vault {
    /// Names, never values — the same rule [`armada_guild::secrets::Withheld`]
    /// enforces by having nowhere to put one.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Vault")
            .field("names", &self.values.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl Vault {
    /// Whether anything was resolved at all.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// The names this vault holds, sorted. **Never the values.**
    pub fn names(&self) -> Vec<&str> {
        self.values.keys().map(String::as_str).collect()
    }

    /// The subset one entry's `secrets:` grant names.
    ///
    /// **The per-entry grant is applied here and only here** (`PLAN.md` §4.7
    /// rule 5). A check with no grant gets an empty map, which is what makes
    /// *"a sibling check sees no secret in its environment"* true by
    /// construction rather than by review.
    pub fn granted(&self, names: &[String]) -> BTreeMap<String, String> {
        armada_core::secrets::granted(&self.values, names)
    }

    /// Every resolved value in `text`, replaced by [`crate::redact::MASK`].
    ///
    /// **Value-level, and applied before serialization** (`PLAN.md` §4.7 rule
    /// 2). Filtering serialized output fails the moment a value contains `"` or
    /// `\`, because the serializer has already escaped it — Armada's own encoder
    /// defeating Armada's own filter. So this runs over the raw captured bytes,
    /// on the way into the log and into the failure journal.
    ///
    /// **This is exact-match scrubbing and it complements
    /// [`crate::redact::scrub`] rather than replacing it.** `redact` knows what
    /// a credential *looks* like and catches values Armada never resolved;
    /// this knows exactly what this run resolved and catches values that look
    /// like nothing at all — `hunter2` is a real password and no shape test will
    /// ever say so.
    ///
    /// **Longest first**, so a value that contains another does not leave the
    /// shorter one's suffix behind.
    pub fn mask(&self, text: &str) -> String {
        if self.values.is_empty() {
            return text.to_string();
        }
        let mut ordered: Vec<&String> = self.values.values().collect();
        ordered.sort_by_key(|value| std::cmp::Reverse(value.len()));
        let mut out = text.to_string();
        for value in ordered {
            // **An empty value is skipped**, because `replace("")` matches
            // between every character and would rewrite the whole text into
            // masks. `resolve` refuses an empty value, so this is a guard
            // against a future caller rather than a live case.
            if !value.is_empty() {
                out = out.replace(value.as_str(), crate::redact::MASK);
            }
        }
        out
    }

    /// The wire form handed to a detached child, on its stdin.
    ///
    /// **JSON, because the escaping has to be somebody's job.** A secret may
    /// contain a newline, a tab or a `"` — a PEM private key contains all
    /// three — and a line-oriented `NAME=value` format would silently truncate
    /// it at the first one, injecting a *partial* credential that fails in a way
    /// nobody could diagnose. `serde_json` already owns that problem.
    ///
    /// **The one deliberate exit from this type**, named for what it does so
    /// that a reviewer can grep for it and find exactly one caller: the
    /// `--detach` handoff in [`crate::verbs::check`]. It goes to a pipe and
    /// never to a file — see the module header.
    pub fn handoff(&self) -> String {
        serde_json::to_string(&self.values).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Read a handoff back, in the detached child.
///
/// **The child never invokes a provider, whatever this returns.** An empty or
/// absent payload gives an empty vault and the run then fails on the grant it
/// cannot satisfy, which is the honest answer: the parent had the terminal, so a
/// child that reached for `op` here would be the exact hang `013` was written
/// about.
///
/// Malformed input is an empty vault rather than an error, for the reason
/// [`armada_core::run::nesting`] gives about `ARMADA_RUN_ID`: Armada writes this
/// and Armada reads it back, so anything else means something in between
/// rewrote it, and the forgiving answer is the one that cannot be used to make
/// a run fail from outside.
pub fn inherit(payload: &str) -> Vault {
    Vault {
        values: serde_json::from_str(payload).unwrap_or_default(),
    }
}

/// Run every provider, in the terminal this process still has.
///
/// **Failure is reported and never quoted** (`PLAN.md` §4.7 rule 3): when a
/// provider fails there is no resolved value registered to scrub against, so a
/// chatty provider — `set -x`, `--debug` — would leak through a path
/// structurally incapable of redaction. Its stderr is *inherited* rather than
/// captured, which means Armada never holds it and so cannot repeat it even by
/// accident; the person at the terminal sees it live, which is where a prompt
/// and a *"vault is locked"* belong anyway. What Armada says is the provider's
/// name, its exit code and a fixed message.
pub fn resolve<R: Run>(
    run: &R,
    cwd: &Path,
    calls: &[Call],
    timeout_ms: u64,
) -> Result<Vault, ArmadaError> {
    let mut values = BTreeMap::new();
    for call in calls {
        values.insert(call.name.clone(), invoke(run, cwd, call, timeout_ms)?);
    }
    Ok(Vault { values })
}

fn invoke<R: Run>(
    run: &R,
    cwd: &Path,
    call: &Call,
    timeout_ms: u64,
) -> Result<String, ArmadaError> {
    let request = RunRequest::new(call.argv.clone(), cwd.to_path_buf())
        .timeout(Duration::from_millis(timeout_ms))
        // **Armada's own session, deliberately.** Every other `RunRequest` asks
        // for `setsid` so one `killpg` reaches the whole tree; a provider must
        // not, because `setsid` creates a session with **no controlling
        // terminal** — and a provider that cannot open `/dev/tty` cannot prompt,
        // which is the failure this whole file exists to prevent. It is a short,
        // awaited, foreground call, so there is no tree to reach for.
        .session(false)
        .stdio(StdioMode::CaptureStdout);

    let output = run.call(&request).map_err(|error| ArmadaError {
        // The same split `check`'s spawn makes: a program that is not there is
        // a config the caller has to edit, anything else is the machine.
        class: match error.kind {
            SpawnErrorKind::NotFound | SpawnErrorKind::PermissionDenied => ErrClass::BadConfig,
            SpawnErrorKind::Other => ErrClass::Environment,
        },
        r#where: format!("secrets.{}", call.name),
        message: format!(
            "`{}` could not be resolved: the `{}` provider would not start (`{}`)",
            call.name, call.provider, error.program
        ),
        next_action: Some(format!(
            "check `secret_providers.{}.cmd` — the program has to be on PATH",
            call.provider
        )),
    })?;

    if output.timed_out {
        return Err(ArmadaError {
            class: ErrClass::Timeout,
            r#where: format!("secrets.{}", call.name),
            message: format!(
                "`{}` could not be resolved: the `{}` provider did not answer within {}s",
                call.name,
                call.provider,
                timeout_ms / 1_000
            ),
            next_action: Some(format!(
                "run `secret_providers.{}.cmd` by hand — it is probably waiting for input Armada cannot give it",
                call.provider
            )),
        });
    }

    let code = output
        .code
        .unwrap_or_else(|| 128 + output.signal.unwrap_or(0));
    if code != 0 {
        return Err(ArmadaError {
            class: ErrClass::ToolFailed,
            r#where: format!("secrets.{}", call.name),
            message: format!(
                "`{}` could not be resolved: the `{}` provider exited {code}",
                call.name, call.provider
            ),
            // **No `{}` holding the provider's output.** Rule 3, and the reason
            // stderr was inherited above: there is nothing here to interpolate.
            next_action: Some(format!(
                "run `secret_providers.{}.cmd` by hand to see what it said",
                call.provider
            )),
        });
    }

    // **One trailing newline goes and nothing else does.** `echo`, `op read`
    // and `security -w` all end with one and none of them means it; `trim()`
    // instead would silently corrupt a secret whose first or last character is
    // a space, which is legal and which nobody would ever diagnose.
    let value = output
        .stdout
        .strip_suffix('\n')
        .map(|text| text.strip_suffix('\r').unwrap_or(text))
        .unwrap_or(&output.stdout)
        .to_string();

    if value.is_empty() {
        return Err(ArmadaError {
            class: ErrClass::ToolFailed,
            r#where: format!("secrets.{}", call.name),
            message: format!(
                "`{}` could not be resolved: the `{}` provider succeeded and printed nothing",
                call.name, call.provider
            ),
            // A variable set to `""` reads as configured and is not — the same
            // failure `armada_guild::secrets::without` refuses to create by
            // dropping a key rather than blanking it.
            next_action: Some(format!(
                "check the reference under `secrets.{}` names something that exists",
                call.name
            )),
        });
    }

    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use armada_core::ctx::{RunOutput, SpawnError};
    use std::cell::RefCell;

    /// A provider bench: what each argv answers, and what was asked.
    struct Providers {
        answers: Vec<Result<RunOutput, SpawnError>>,
        asked: RefCell<Vec<RunRequest>>,
    }

    impl Providers {
        fn new(answers: Vec<Result<RunOutput, SpawnError>>) -> Self {
            Providers {
                answers,
                asked: RefCell::new(Vec::new()),
            }
        }
    }

    impl Run for Providers {
        fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
            let index = self.asked.borrow().len();
            self.asked.borrow_mut().push(request.clone());
            match self.answers.get(index) {
                Some(Ok(output)) => Ok(output.clone()),
                Some(Err(error)) => Err(error.clone()),
                None => panic!("a provider was invoked more times than the bench expected"),
            }
        }
    }

    fn printed(text: &str) -> Result<RunOutput, SpawnError> {
        Ok(RunOutput {
            code: Some(0),
            signal: None,
            stdout: text.to_string(),
            stderr: String::new(),
            timed_out: false,
        })
    }

    fn call(name: &str) -> Call {
        Call {
            name: name.to_string(),
            provider: "op".to_string(),
            argv: vec!["op".to_string(), "read".to_string(), "op://a/b".to_string()],
        }
    }

    fn vault(pairs: &[(&str, &str)]) -> Vault {
        Vault {
            values: pairs
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
        }
    }

    /// The happy path, and the two properties of the request that decide
    /// whether a prompting provider can prompt.
    #[test]
    fn a_provider_keeps_the_terminal_and_gives_up_only_its_stdout() {
        let bench = Providers::new(vec![printed("secret-value\n")]);
        let resolved = resolve(&bench, Path::new("/scratch"), &[call("T")], 1_000).unwrap();
        assert_eq!(resolved.granted(&["T".to_string()])["T"], "secret-value");

        let asked = bench.asked.borrow();
        assert_eq!(asked[0].argv, vec!["op", "read", "op://a/b"]);
        assert!(
            !asked[0].new_session,
            "setsid gives the provider no controlling terminal, so it can never prompt"
        );
        assert_eq!(
            asked[0].stdio,
            StdioMode::CaptureStdout,
            "stdin and stderr must stay the terminal's; only stdout is Armada's"
        );
        assert_eq!(asked[0].timeout, Some(Duration::from_millis(1_000)));
    }

    /// One trailing newline, because every provider ends with one and none of
    /// them means it. Nothing else is touched.
    #[test]
    fn exactly_one_trailing_newline_is_taken_off() {
        for (printed_text, want) in [
            ("value\n", "value"),
            ("value\r\n", "value"),
            ("value", "value"),
            ("value\n\n", "value\n"),
            (" padded \n", " padded "),
            ("multi\nline\n", "multi\nline"),
        ] {
            let bench = Providers::new(vec![printed(printed_text)]);
            let resolved = resolve(&bench, Path::new("/scratch"), &[call("T")], 1_000).unwrap();
            assert_eq!(
                resolved.granted(&["T".to_string()])["T"],
                want,
                "{printed_text:?}"
            );
        }
    }

    /// **Rule 3.** A chatty failing provider gets named and quoted nowhere.
    #[test]
    fn a_failing_provider_is_named_and_never_quoted() {
        let leak = "+ op read op://Private/AWS/root\nghp_EXAMPLE_NOT_A_REAL_CREDENTIAL_000001";
        let bench = Providers::new(vec![Ok(RunOutput {
            code: Some(3),
            signal: None,
            stdout: leak.to_string(),
            stderr: leak.to_string(),
            timed_out: false,
        })]);
        let error = resolve(&bench, Path::new("/scratch"), &[call("T")], 1_000).unwrap_err();

        assert_eq!(error.class, ErrClass::ToolFailed);
        assert!(error.message.contains("`T`"), "{}", error.message);
        assert!(error.message.contains("`op`"), "{}", error.message);
        assert!(error.message.contains('3'), "{}", error.message);
        let whole = format!("{error:?}");
        assert!(
            !whole.contains("ghp_") && !whole.contains("op://Private"),
            "the provider's output reached the error: {whole}"
        );
    }

    /// A provider that would prompt where it cannot is a **timeout with a
    /// name**, not a hang — which is the whole of `013`'s third requirement.
    #[test]
    fn a_provider_that_never_answers_times_out_naming_the_secret() {
        let bench = Providers::new(vec![Ok(RunOutput {
            code: None,
            signal: Some(9),
            stdout: String::new(),
            stderr: String::new(),
            timed_out: true,
        })]);
        let error = resolve(&bench, Path::new("/scratch"), &[call("T")], 5_000).unwrap_err();
        assert_eq!(error.class, ErrClass::Timeout);
        assert!(error.message.contains("`T`"), "{}", error.message);
        assert!(error.message.contains("`op`"), "{}", error.message);
        assert!(error.message.contains("5s"), "{}", error.message);
    }

    /// A provider that exits 0 and prints nothing has resolved nothing. A
    /// variable set to `""` reads as configured and is not.
    #[test]
    fn a_provider_that_prints_nothing_is_a_failure_and_not_an_empty_secret() {
        let bench = Providers::new(vec![printed("\n")]);
        let error = resolve(&bench, Path::new("/scratch"), &[call("T")], 1_000).unwrap_err();
        assert!(error.message.contains("printed nothing"), "{error:?}");
    }

    /// A provider that is not installed says so against the config key that
    /// names it.
    #[test]
    fn a_provider_that_is_not_installed_is_bad_config_naming_the_scheme() {
        let bench = Providers::new(vec![Err(SpawnError {
            program: "op".to_string(),
            kind: SpawnErrorKind::NotFound,
            message: "no such file".to_string(),
        })]);
        let error = resolve(&bench, Path::new("/scratch"), &[call("T")], 1_000).unwrap_err();
        assert_eq!(error.class, ErrClass::BadConfig);
        assert!(
            error
                .next_action
                .as_deref()
                .is_some_and(|next| next.contains("secret_providers.op.cmd")),
            "{error:?}"
        );
    }

    /// **Least privilege at the last moment.** A check with no grant gets
    /// nothing, and a check with one gets only its own.
    #[test]
    fn a_grant_hands_over_its_own_names_and_no_others() {
        let vault = vault(&[("A", "value-a"), ("B", "value-b")]);
        assert!(vault.granted(&[]).is_empty(), "an ungranted entry got one");
        assert_eq!(
            vault.granted(&["B".to_string()]).into_iter().collect::<Vec<_>>(),
            vec![("B".to_string(), "value-b".to_string())]
        );
    }

    /// The value-level scrubber, on a value **no shape test would ever
    /// catch** — which is why it exists beside `redact::scrub` rather than
    /// instead of it.
    #[test]
    fn a_resolved_value_is_masked_even_when_it_looks_like_nothing() {
        let plain = "hunter2";
        assert!(
            !armada_guild::secrets::value_is_credential_shaped(plain),
            "the fixture has to be a value the shape detector misses"
        );
        let vault = vault(&[("A", plain)]);
        let masked = vault.mask("logging in with hunter2 now\nhunter2 again\n");
        assert!(!masked.contains(plain), "{masked}");
        assert_eq!(masked.lines().count(), 2, "line structure survives: {masked}");
        assert!(masked.contains(crate::redact::MASK), "{masked}");
    }

    /// A value containing another must not leave the shorter one's remainder
    /// behind — which is what ordering by length buys.
    #[test]
    fn a_value_that_contains_another_is_masked_whole() {
        let vault = vault(&[("SHORT", "abc"), ("LONG", "abcdef")]);
        assert_eq!(vault.mask("abcdef"), crate::redact::MASK);
    }

    /// An empty vault is a pass-through, and an empty value cannot rewrite the
    /// text into masks.
    #[test]
    fn nothing_resolved_changes_nothing() {
        assert_eq!(Vault::default().mask("untouched"), "untouched");
        assert_eq!(vault(&[("A", "")]).mask("untouched"), "untouched");
    }

    /// **The type cannot leak through `{:?}`.** The derive would have put every
    /// value into any format string anyone ever writes, including one inside an
    /// error.
    #[test]
    fn debugging_a_vault_prints_names_and_never_values() {
        let printed = format!("{:?}", vault(&[("A", "value-a")]));
        assert!(printed.contains('A'), "{printed}");
        assert!(!printed.contains("value-a"), "{printed}");
    }

    /// The handoff survives every character a line-oriented format would eat.
    #[test]
    fn the_handoff_round_trips_a_value_with_newlines_quotes_and_equals() {
        let awkward = "-----BEGIN KEY-----\nline\"two\"\tthree=four\n-----END KEY-----";
        let before = vault(&[("PEM", awkward), ("PLAIN", "x")]);
        let after = inherit(&before.handoff());
        assert_eq!(after.granted(&["PEM".to_string()])["PEM"], awkward);
        assert_eq!(after.names(), vec!["PEM", "PLAIN"]);
    }

    /// A child handed nothing gets an empty vault rather than an error — and a
    /// child handed nonsense gets the same, because Armada wrote it and Armada
    /// read it back, so anything else was rewritten in between.
    #[test]
    fn an_absent_or_malformed_handoff_is_an_empty_vault_and_never_a_provider_call() {
        for payload in ["", "{}", "not json", "[1,2,3]", "null"] {
            assert!(inherit(payload).is_empty(), "{payload:?}");
        }
    }
}
