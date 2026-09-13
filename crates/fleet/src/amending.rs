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
    amend, CheckEdit, CommandEdit, Edit, NewCheck, NewCommand, NewLink, NewNarrowing, NewPort,
    NotAmended, PortEdit,
};
use core_model::{AutoMerge, ReviewGate};
use ipc::{
    EditManifest, Instant, LinkDraft, ManifestEdit, ManifestEdited, NarrowingDraft, WireError,
    WireValue,
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
    ) -> Result<ManifestEdited, Refusal> {
        let file = self.manifest().path();
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
            .collect::<Result<Vec<Edit>, Unknown>>()
            .map_err(|unknown| self.unknown_word(unknown))?;
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
fn edit(wire: ManifestEdit) -> Result<Edit, Unknown> {
    let check = |name, edit| Edit::Check { name, edit };
    let command = |name, edit| Edit::Command { name, edit };
    let port = |name, edit| Edit::Port { name, edit };
    Ok(match wire {
        ManifestEdit::AddCheck { name, check: new } => check(
            name,
            CheckEdit::Add(NewCheck {
                run: new.run,
                requires: new.requires,
                when: new.when,
                narrow: new.narrow.map(narrowing),
            }),
        ),
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
    })
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
