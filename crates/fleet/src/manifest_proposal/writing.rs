//! A proposal as `armada.yml`: edits to empty text through `config`'s one writer, so a
//! proposal and the Manifest form cannot disagree about what the file looks like.

use std::io;

use config::{
    amend, CheckEdit, CommandEdit, Edit, NewCheck, NewCommand, NewPort, NewRunner, PortEdit,
};
use core_model::{AutoMerge, ReviewGate};
use ipc::{Instant, ManifestFault, ManifestRefused, ManifestSaved, PolicyKey, Provenance};

use super::Draft;
use crate::editing::{create, NotCreated};

/// Why Write did not land.
#[derive(Debug)]
pub(crate) enum NotWritten {
    Written,
    Refused(ManifestRefused),
    Appeared(Option<String>),
    Unwritable(io::Error),
}

impl Draft {
    /// The file Write would put down, or every fault that stops it loading.
    pub fn text(&self) -> Result<String, ManifestRefused> {
        let edits = self.edits()?;
        amend(&self.path, "", &edits)
            .map(|done| done.text().to_string())
            .map_err(refused)
    }

    /// Put the file down, only where it loads and nothing is at the path.
    pub(crate) fn write(&mut self, at: Instant) -> Result<ManifestSaved, NotWritten> {
        if self.written.is_some() {
            return Err(NotWritten::Written);
        }
        let text = self.text().map_err(NotWritten::Refused)?;
        create(&self.path, &text).map_err(|why| match why {
            NotCreated::Appeared(on_disk) => NotWritten::Appeared(on_disk),
            NotCreated::Unwritable(cause) => NotWritten::Unwritable(cause),
        })?;
        let saved = ManifestSaved {
            path: self.path.display().to_string(),
            at,
        };
        self.written = Some(saved.clone());
        Ok(saved)
    }

    fn edits(&self) -> Result<Vec<Edit>, ManifestRefused> {
        let mut edits = vec![Edit::Version(1), Edit::Id(self.id.value.clone())];
        edits.extend(self.ports.iter().map(|port| Edit::Port {
            name: port.name.clone(),
            edit: PortEdit::Add(NewPort {
                container: port.container.map(u32::from),
                env: port.env.clone(),
            }),
        }));
        edits.extend(self.checks.iter().map(|check| Edit::Check {
            name: check.name.clone(),
            edit: CheckEdit::Add(NewCheck {
                run: check.run.clone(),
                requires: check.requires.clone(),
                when: Vec::new(),
                narrow: None,
                runner: check.runner.as_ref().map(|runner| NewRunner {
                    name: runner.name.clone(),
                    dir: runner.dir.clone(),
                }),
            }),
        }));
        edits.extend(self.commands.iter().map(|command| Edit::Command {
            name: command.name.clone(),
            edit: CommandEdit::Add(NewCommand {
                run: Some(command.run.clone()),
                destructive: command.destructive,
                serve: None,
                ready: None,
                links: Vec::new(),
            }),
        }));
        if let Some(setup) = &self.setup {
            edits.push(Edit::SetupRequires(setup.requires.clone()));
        }
        // A default row is an absent key, so only a pinned policy becomes an edit.
        for row in self
            .policy
            .iter()
            .filter(|row| row.provenance != Provenance::Default)
        {
            let (key, edit) = match row.key {
                PolicyKey::AutoMerge => (
                    "auto_merge",
                    AutoMerge::from_written(&row.value).map(|word| Edit::AutoMerge(Some(word))),
                ),
                PolicyKey::ReviewGate => (
                    "review_gate",
                    ReviewGate::from_written(&row.value).map(|word| Edit::ReviewGate(Some(word))),
                ),
            };
            let Some(edit) = edit else {
                let said = format!("`{}` is not a value `{key}` takes", row.value);
                return Err(ManifestRefused {
                    summary: said.clone(),
                    faults: vec![ManifestFault {
                        key: key.to_string(),
                        fault: said,
                    }],
                });
            };
            edits.push(edit);
        }
        Ok(edits)
    }
}

/// Every fault the writer found, at its key.
fn refused(why: config::NotAmended) -> ManifestRefused {
    let faults = match &why {
        config::NotAmended::Refused(load) => load
            .refusals()
            .iter()
            .map(|one| ManifestFault {
                key: one.key.clone(),
                fault: one.fault.to_string(),
            })
            .collect(),
        config::NotAmended::Misnamed { key, .. } | config::NotAmended::Unplaceable { key, .. } => {
            vec![ManifestFault {
                key: key.clone(),
                fault: why.to_string(),
            }]
        }
    };
    ManifestRefused {
        summary: why.to_string(),
        faults,
    }
}
