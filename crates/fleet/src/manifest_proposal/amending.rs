//! A person's edit to a proposal, and where it moves a line's provenance. A value is not
//! refused here: a proposal passes through wrong states, and Write refuses.

use ipc::{
    Band, PolicyKey, ProposalEdit, ProposedCheck, ProposedCommand, ProposedPort, ProposedSetup,
    Provenance,
};

use super::{Draft, AUTO_MERGE_DEFAULT, REVIEW_GATE_DEFAULT};

/// Why an edit was not applied.
#[derive(Debug, PartialEq, Eq)]
pub enum NotAmended {
    /// A move or a removal named a line the proposal does not have.
    NoSuchLine { band: Option<Band>, name: String },
    /// Moving would drop a key the other registry has no place for.
    NotMovable { name: String, holds: &'static str },
}

impl std::fmt::Display for NotAmended {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotAmended::NoSuchLine {
                band: Some(band),
                name,
            } => write!(
                out,
                "this proposal has no `{name}` under `{}`",
                spelled(*band)
            ),
            NotAmended::NoSuchLine { band: None, name } => {
                write!(out, "this proposal has no Check or Command named `{name}`")
            }
            NotAmended::NotMovable { name, holds } => write!(
                out,
                "`{name}` carries `{holds}`, which the other registry has no key for, so it was \
                 not moved. Clear `{holds}` first"
            ),
        }
    }
}

impl std::error::Error for NotAmended {}

fn spelled(band: Band) -> &'static str {
    match band {
        Band::Ports => "ports",
        Band::Checks => "checks",
        Band::Commands => "commands",
    }
}

/// Added stays added, because no file ever said it.
fn touched(was: &Provenance) -> Provenance {
    match was {
        Provenance::AddedDuringSetup => Provenance::AddedDuringSetup,
        _ => Provenance::EditedDuringSetup,
    }
}

impl Draft {
    /// Apply one edit. An edit that changes nothing moves nothing.
    pub fn amend(&mut self, edit: ProposalEdit) -> Result<(), NotAmended> {
        match edit {
            ProposalEdit::Id { id } => {
                if self.id.value != id {
                    self.id.provenance = touched(&self.id.provenance);
                    self.id.value = id;
                }
            }
            ProposalEdit::Port {
                name,
                container,
                env,
            } => {
                let provenance = Provenance::AddedDuringSetup;
                let put = ProposedPort {
                    name,
                    container,
                    env,
                    provenance,
                };
                put_line(
                    &mut self.ports,
                    put,
                    |one| &one.name,
                    |one| &mut one.provenance,
                );
            }
            ProposalEdit::Check {
                name,
                run,
                requires,
            } => {
                let provenance = Provenance::AddedDuringSetup;
                let put = ProposedCheck {
                    name,
                    run,
                    requires,
                    provenance,
                };
                put_line(
                    &mut self.checks,
                    put,
                    |one| &one.name,
                    |one| &mut one.provenance,
                );
            }
            ProposalEdit::Command {
                name,
                run,
                destructive,
            } => {
                let provenance = Provenance::AddedDuringSetup;
                let put = ProposedCommand {
                    name,
                    run,
                    destructive,
                    provenance,
                };
                put_line(
                    &mut self.commands,
                    put,
                    |one| &one.name,
                    |one| &mut one.provenance,
                );
            }
            ProposalEdit::Setup { requires } => self.set_setup(requires),
            ProposalEdit::Policy { key, value } => self.set_policy(key, value),
            ProposalEdit::Move { name } => self.moved(name)?,
            ProposalEdit::Remove { band, name } => {
                let removed = match band {
                    Band::Ports => take(&mut self.ports, |one| one.name == name).is_some(),
                    Band::Checks => take(&mut self.checks, |one| one.name == name).is_some(),
                    Band::Commands => take(&mut self.commands, |one| one.name == name).is_some(),
                };
                if !removed {
                    return Err(NotAmended::NoSuchLine {
                        band: Some(band),
                        name,
                    });
                }
            }
        }
        Ok(())
    }

    fn set_setup(&mut self, requires: Vec<String>) {
        self.setup = match (self.setup.take(), requires.is_empty()) {
            (_, true) => None,
            (Some(was), false) if was.requires == requires => Some(was),
            (Some(was), false) => Some(ProposedSetup {
                requires,
                provenance: touched(&was.provenance),
            }),
            (None, false) => Some(ProposedSetup {
                requires,
                provenance: Provenance::AddedDuringSetup,
            }),
        };
    }

    fn set_policy(&mut self, key: PolicyKey, value: Option<String>) {
        let Some(row) = self.policy.iter_mut().find(|row| row.key == key) else {
            return;
        };
        match value {
            None => {
                row.value = match key {
                    PolicyKey::AutoMerge => AUTO_MERGE_DEFAULT,
                    PolicyKey::ReviewGate => REVIEW_GATE_DEFAULT,
                }
                .to_string();
                row.provenance = Provenance::Default;
            }
            // Pinning the default's own word writes the key, so it is still an edit.
            Some(value) if row.value != value || row.provenance == Provenance::Default => {
                row.value = value;
                row.provenance = Provenance::EditedDuringSetup;
            }
            Some(_) => {}
        }
    }

    fn moved(&mut self, name: String) -> Result<(), NotAmended> {
        if let Some(at) = self.checks.iter().position(|one| one.name == name) {
            if !self.checks[at].requires.is_empty() {
                return Err(NotAmended::NotMovable {
                    name,
                    holds: "requires",
                });
            }
            let check = self.checks.remove(at);
            self.commands.push(ProposedCommand {
                provenance: touched(&check.provenance),
                name: check.name,
                run: check.run,
                destructive: false,
            });
            return Ok(());
        }
        if let Some(at) = self.commands.iter().position(|one| one.name == name) {
            if self.commands[at].destructive {
                return Err(NotAmended::NotMovable {
                    name,
                    holds: "destructive",
                });
            }
            let command = self.commands.remove(at);
            self.checks.push(ProposedCheck {
                provenance: touched(&command.provenance),
                name: command.name,
                run: command.run,
                requires: Vec::new(),
            });
            return Ok(());
        }
        Err(NotAmended::NoSuchLine { band: None, name })
    }
}

/// Replace the line with `put`'s name, or add it. Provenance is decided here, never taken
/// from `put`, so an unchanged line keeps its own.
fn put_line<T: PartialEq>(
    lines: &mut Vec<T>,
    mut put: T,
    name: impl Fn(&T) -> &String,
    provenance: impl Fn(&mut T) -> &mut Provenance,
) {
    let Some(at) = lines.iter().position(|one| name(one) == name(&put)) else {
        *provenance(&mut put) = Provenance::AddedDuringSetup;
        return lines.push(put);
    };
    let was = provenance(&mut lines[at]).clone();
    *provenance(&mut put) = was.clone();
    if lines[at] != put {
        *provenance(&mut put) = touched(&was);
        lines[at] = put;
    }
}

fn take<T>(lines: &mut Vec<T>, found: impl Fn(&T) -> bool) -> Option<T> {
    let at = lines.iter().position(found)?;
    Some(lines.remove(at))
}
