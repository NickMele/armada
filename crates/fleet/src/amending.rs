//! A form's edits to `armada.yml`, placed and written — Journey 9, *Editing*,
//! and `#721`.
//!
//! **The file is read before anything is placed.** Edits drawn from a reading
//! the disk no longer holds are refused exactly as a save is, before they are
//! applied to text nobody saw. [`editing::save`] then compares again at the
//! write, so the one guard is never skipped.
//!
//! **What a form produces always loads.** `config::amend` hands back text only
//! once `Manifest::parse` accepted it, and a refusal carries every fault. That
//! is the opposite of `save_manifest_file`, which writes work in progress on
//! purpose — a form has no half-typed state to put down.
//!
//! [`editing::save`]: crate::editing::save

use std::fs;
use std::io;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use config::{
    amend, CheckEdit, CommandEdit, Edit, EvidenceEdit, NewCheck, NewCommand, NewEvidence, NewLink,
    NewNarrowing, NewPort, NotAmended, PortEdit,
};
use core_model::{AutoMerge, Covers, ReviewGate};
use ipc::{
    CheckDraft, CommandDraft, EditManifest, EvidenceDraft, Instant, LinkDraft, ManifestDeclared,
    ManifestEdit, ManifestEdited, NamedCheck, NamedCommand, NamedPort, NarrowingDraft, PolicyWords,
    PortDraft, WireError, WireValue,
};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::editing::{save, NotSaved};

/// An edit names a Check, Command or port the file does not declare, or adds
/// one it already does. **A 422**: the form was drawn from another reading, and
/// the answer is to read the file again. `key` names where it looked.
const MANIFEST_EDIT_MISNAMED: &str = "fleet.manifest_edit_misnamed";
/// The file at `key` is written in a shape a form does not edit without
/// touching more than the key. **A 422**, and the file view is the recourse.
const MANIFEST_EDIT_UNPLACEABLE: &str = "fleet.manifest_edit_unplaceable";
/// The edits placed and the result would not load. **A 422 carrying `faults`**
/// as `[key, fault]` pairs, and nothing is written.
const MANIFEST_EDIT_REFUSED: &str = "fleet.manifest_edit_refused";

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// `edit_manifest` — the edits spliced into the file they were drawn from,
    /// parsed, and written through [`save`].
    pub(crate) fn edit_manifest_file(
        &self,
        asked: EditManifest,
        served: &crate::repositories::Served,
    ) -> Result<ManifestEdited, Refusal> {
        let file = served.manifest().path();
        let path = file.display().to_string();
        match fs::read_to_string(file) {
            Ok(on_disk) if on_disk == asked.read => {}
            Ok(on_disk) => {
                return Err(self.refusal(Adrift::ManifestMovedUnderTheEdit {
                    path,
                    on_disk: Some(on_disk),
                }))
            }
            Err(cause) if cause.kind() == io::ErrorKind::NotFound => {
                return Err(self.refusal(Adrift::ManifestMovedUnderTheEdit {
                    path,
                    on_disk: None,
                }))
            }
            Err(cause) => return Err(self.refusal(Adrift::ManifestUnreadable { path, cause })),
        }
        let edits = asked
            .edits
            .into_iter()
            .map(edit)
            .collect::<Result<Vec<Vec<Edit>>, Unknown>>()
            .map_err(|unknown| self.unknown_word(unknown))?
            .concat();
        let amended = amend(file, &asked.read, &edits).map_err(|why| self.not_amended(why))?;
        save(file, &asked.read, amended.text()).map_err(|why| {
            self.refusal(match why {
                NotSaved::Unreadable(cause) => Adrift::ManifestUnreadable {
                    path: path.clone(),
                    cause,
                },
                NotSaved::Moved(on_disk) => Adrift::ManifestMovedUnderTheEdit {
                    path: path.clone(),
                    on_disk,
                },
                NotSaved::Unwritable(cause) => Adrift::ManifestUnwritable {
                    path: path.clone(),
                    cause,
                },
            })
        })?;
        Ok(ManifestEdited {
            path,
            at: Instant::from(&self.now()),
            text: amended.text().to_string(),
            declared: Some(declared_in(amended.manifest())),
        })
    }

    fn not_amended(&self, why: NotAmended) -> Refusal {
        let said = why.to_string();
        Refusal::Unacceptable(match why {
            NotAmended::Misnamed { key, .. } => {
                WireError::raised(MANIFEST_EDIT_MISNAMED, said, self.run_id())
                    .with_field("key", WireValue::Str(key))
            }
            NotAmended::Unplaceable { key, .. } => {
                WireError::raised(MANIFEST_EDIT_UNPLACEABLE, said, self.run_id())
                    .with_field("key", WireValue::Str(key))
            }
            NotAmended::Refused(load) => {
                let faults = load
                    .refusals()
                    .iter()
                    .map(|refusal| fault(&refusal.key, refusal.fault.to_string()))
                    .collect();
                WireError::raised(MANIFEST_EDIT_REFUSED, said, self.run_id())
                    .with_field("faults", WireValue::List(faults))
            }
        })
    }

    /// A policy word the file would refuse, refused as the file would be.
    fn unknown_word(&self, unknown: Unknown) -> Refusal {
        let said = format!("`{}` is not a value `{}` takes", unknown.word, unknown.key);
        Refusal::Unacceptable(
            WireError::raised(MANIFEST_EDIT_REFUSED, said.clone(), self.run_id())
                .with_field("faults", WireValue::List(vec![fault(unknown.key, said)])),
        )
    }
}

fn fault(key: &str, said: String) -> WireValue {
    WireValue::List(vec![WireValue::Str(key.to_string()), WireValue::Str(said)])
}

/// A word the wire carried that the policy it names does not take.
struct Unknown {
    key: &'static str,
    word: String,
}

/// The wire's edit as `config`'s. **Closed on both sides**, so an edit the form
/// can send is an edit the writer knows.
fn edit(wire: ManifestEdit) -> Result<Vec<Edit>, Unknown> {
    let check = |name, edit| Edit::Check { name, edit };
    let command = |name, edit| Edit::Command { name, edit };
    let port = |name, edit| Edit::Port { name, edit };
    Ok(vec![match wire {
        // A Check is declared without its exit code, which is then set on it.
        ManifestEdit::AddCheck { name, check: new } => {
            let code = new.expect_exit_code;
            let added = check(
                name.clone(),
                CheckEdit::Add(NewCheck {
                    run: new.run,
                    requires: new.requires,
                    when: new.when,
                    narrow: new.narrow.map(narrowing),
                }),
            );
            return Ok(match code {
                0 => vec![added],
                code => vec![added, check(name, CheckEdit::ExpectExitCode(code))],
            });
        }
        ManifestEdit::SetCheckExpectExitCode {
            name,
            expect_exit_code,
        } => check(name, CheckEdit::ExpectExitCode(expect_exit_code)),
        ManifestEdit::SetBase { base } => Edit::Base(base),
        ManifestEdit::AddEvidence { evidence } => Edit::Evidence(EvidenceEdit::Add(NewEvidence {
            serve: evidence.serve,
            ready: evidence.ready,
            run: evidence.run,
            frames: evidence.frames,
            never: evidence.never,
        })),
        ManifestEdit::RemoveEvidence => Edit::Evidence(EvidenceEdit::Remove),
        ManifestEdit::SetEvidenceServe { serve } => Edit::Evidence(EvidenceEdit::Serve(serve)),
        ManifestEdit::SetEvidenceReady { ready } => Edit::Evidence(EvidenceEdit::Ready(ready)),
        ManifestEdit::SetEvidenceRun { run } => Edit::Evidence(EvidenceEdit::Run(run)),
        ManifestEdit::SetEvidenceFrames { frames } => Edit::Evidence(EvidenceEdit::Frames(frames)),
        ManifestEdit::SetEvidenceNever { never } => Edit::Evidence(EvidenceEdit::Never(never)),
        ManifestEdit::SetAfterMergeChecks { checks } => Edit::AfterMergeChecks(checks),
        ManifestEdit::SetSetupRequires { requires } => Edit::SetupRequires(requires),
        ManifestEdit::SetQuietAfterSeconds {
            quiet_after_seconds,
        } => Edit::QuietAfterSeconds(quiet_after_seconds),
        ManifestEdit::SetPokeLimit { poke_limit } => Edit::PokeLimit(poke_limit),
        ManifestEdit::SetExcludePaths { exclude_paths } => Edit::ExcludePaths(exclude_paths),
        ManifestEdit::RemoveCheck { name } => check(name, CheckEdit::Remove),
        ManifestEdit::SetCheckRun { name, run } => check(name, CheckEdit::Run(run)),
        ManifestEdit::SetCheckRequires { name, requires } => {
            check(name, CheckEdit::Requires(requires))
        }
        ManifestEdit::SetCheckWhen { name, when } => check(name, CheckEdit::When(when)),
        ManifestEdit::SetCheckNarrow { name, narrow } => {
            check(name, CheckEdit::Narrow(narrow.map(narrowing)))
        }
        ManifestEdit::AddCommand { name, command: new } => command(
            name,
            CommandEdit::Add(NewCommand {
                run: new.run,
                destructive: new.destructive,
                serve: new.serve,
                ready: new.ready,
                links: new.links.into_iter().map(link).collect(),
            }),
        ),
        ManifestEdit::RemoveCommand { name } => command(name, CommandEdit::Remove),
        ManifestEdit::SetCommandRun { name, run } => command(name, CommandEdit::Run(run)),
        ManifestEdit::SetCommandDestructive { name, destructive } => {
            command(name, CommandEdit::Destructive(destructive))
        }
        ManifestEdit::SetCommandServe { name, serve } => command(name, CommandEdit::Serve(serve)),
        ManifestEdit::SetCommandReady { name, ready } => command(name, CommandEdit::Ready(ready)),
        ManifestEdit::SetCommandLinks { name, links } => command(
            name,
            CommandEdit::Links(links.into_iter().map(link).collect()),
        ),
        ManifestEdit::AddPort { name, port: new } => port(
            name,
            PortEdit::Add(NewPort {
                container: new.container,
                env: new.env,
            }),
        ),
        ManifestEdit::RemovePort { name } => port(name, PortEdit::Remove),
        ManifestEdit::SetPortContainer { name, container } => {
            port(name, PortEdit::Container(container))
        }
        ManifestEdit::SetPortEnv { name, env } => port(name, PortEdit::Env(env)),
        ManifestEdit::SetAutoMerge { auto_merge } => Edit::AutoMerge(
            auto_merge
                .map(|word| {
                    AutoMerge::from_written(&word).ok_or(Unknown {
                        key: "auto_merge",
                        word,
                    })
                })
                .transpose()?,
        ),
        ManifestEdit::SetReviewGate { review_gate } => Edit::ReviewGate(
            review_gate
                .map(|word| {
                    ReviewGate::from_written(&word).ok_or(Unknown {
                        key: "review_gate",
                        word,
                    })
                })
                .transpose()?,
        ),
        ManifestEdit::SetCostCapMicrosPerJob {
            cost_cap_micros_per_job,
        } => Edit::CostCapMicrosPerJob(cost_cap_micros_per_job),
        ManifestEdit::SetTurnCapPerJob { turn_cap_per_job } => {
            Edit::TurnCapPerJob(turn_cap_per_job)
        }
    }])
}

fn narrowing(wire: NarrowingDraft) -> NewNarrowing {
    NewNarrowing {
        run: wire.run,
        each: wire.each,
        from: wire.from,
        under: wire.under,
        except: wire.except,
    }
}

fn link(wire: LinkDraft) -> NewLink {
    NewLink {
        url: wire.url,
        name: wire.name,
    }
}

/// What a Manifest declares, in the drafts a form sends back — so a key the
/// form reads is a key it can write, and nothing it draws is a second spelling.
pub(crate) fn declared_in(manifest: &config::Manifest) -> ManifestDeclared {
    let patterns = |covers: Option<&Covers>| {
        covers.map_or_else(Vec::new, |covers| {
            covers
                .patterns()
                .iter()
                .map(|pattern| pattern.as_str().to_string())
                .collect()
        })
    };
    let checks = manifest
        .checks_as_written()
        .iter()
        .filter_map(|name| {
            let check = manifest.check(name)?;
            Some(NamedCheck {
                name: name.clone(),
                check: CheckDraft {
                    run: check.run().to_string(),
                    expect_exit_code: check.expect_exit_code(),
                    requires: check
                        .requires()
                        .iter()
                        .map(|required| required.name().to_string())
                        .collect(),
                    when: patterns(check.when()),
                    narrow: check.narrow().map(|narrow| NarrowingDraft {
                        run: narrow.run().to_string(),
                        each: narrow.each().to_string(),
                        from: patterns(narrow.from()),
                        under: narrow.under().map(str::to_string),
                        except: narrow.except().to_vec(),
                    }),
                },
            })
        })
        .collect();
    let mut commands: Vec<NamedCommand> = manifest
        .command_names()
        .into_iter()
        .filter_map(|name| {
            let command = manifest.command(&name)?;
            Some(NamedCommand {
                command: CommandDraft {
                    run: Some(command.run().to_string()),
                    destructive: command.is_destructive(),
                    serve: None,
                    ready: None,
                    links: Vec::new(),
                },
                name,
            })
        })
        .collect();
    commands.extend(manifest.server_names().into_iter().filter_map(|name| {
        let server = manifest.server(&name)?;
        Some(NamedCommand {
            command: CommandDraft {
                run: server.run().map(str::to_string),
                destructive: server.is_destructive(),
                serve: Some(server.serve().to_string()),
                ready: server.ready().map(str::to_string),
                links: server
                    .links()
                    .iter()
                    .map(|link| LinkDraft {
                        url: link.url().to_string(),
                        name: link.name().map(str::to_string),
                    })
                    .collect(),
            },
            name,
        })
    }));
    commands.sort_by(|one, other| one.name.cmp(&other.name));
    let ports = manifest
        .port_names()
        .into_iter()
        .filter_map(|name| {
            let port = manifest.port(&name)?;
            Some(NamedPort {
                port: PortDraft {
                    container: port.container(),
                    env: port.env().map(str::to_string),
                },
                name,
            })
        })
        .collect();
    ManifestDeclared {
        id: Some(manifest.id().as_str().to_string()),
        version: Some(manifest.version()),
        checks,
        commands,
        ports,
        auto_merge: PolicyWords {
            written: manifest.auto_merge().as_written().to_string(),
            offered: AutoMerge::ALL
                .iter()
                .map(|word| word.as_written().to_string())
                .collect(),
        },
        review_gate: PolicyWords {
            written: manifest.review_gate().as_written().to_string(),
            offered: ReviewGate::ALL
                .iter()
                .map(|word| word.as_written().to_string())
                .collect(),
        },
        cost_cap_micros_per_job: manifest.cost_cap_micros(),
        turn_cap_per_job: manifest.turn_cap(),
        base: manifest.base().map(str::to_string),
        evidence: manifest.harness().map(|harness| EvidenceDraft {
            serve: harness.serve().map(str::to_string),
            ready: harness.ready().map(str::to_string),
            run: harness.run().to_string(),
            frames: harness.frames().as_str().to_string(),
            never: harness.never().to_vec(),
        }),
        after_merge_checks: manifest
            .proved_after_a_merge()
            .iter()
            .map(|check| check.label().to_string())
            .collect(),
        setup_requires: manifest
            .prepared_by()
            .iter()
            .map(|preparation| preparation.name().to_string())
            .collect(),
        quiet_after_seconds: manifest.quiet_after_seconds(),
        poke_limit: manifest.poke_limit(),
        exclude_paths: manifest
            .exclude_paths()
            .iter()
            .map(|path| path.as_str().to_string())
            .collect(),
    }
}
