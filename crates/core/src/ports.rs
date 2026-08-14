//! Port blocks: choosing one, and assigning declared names inside it.
//!
//! `port_block` is the **workspace's**; assignments are the component's
//! (PLAN.md §3.1). A block is claimed once at `char init` and kept until
//! `char clean` — `char down` deliberately keeps it, because it is still your
//! workspace.
//!
//! Choosing a block is pure, the claim can lose a race, and losing means
//! re-deciding — the same shape as lease acquisition, which is why the retry
//! lives in [`crate::lease`]'s reducer rather than in a loop of its own.

use crate::config::ResolvedConfig;
use crate::error::{CharError, ConfigWhere, ErrClass};
use serde::Serialize;
use std::collections::BTreeMap;

/// The first port char hands out.
///
/// **PLAN.md does not state a base and this is therefore a phase-2 decision.**
/// It is taken from §3.1's own payload, which shows `5460-5469` and
/// `5470-5479` — so the documented example and the implementation agree rather
/// than each picking a number. It sits above the registered range a repo's own
/// services conventionally use and well below the ephemeral range.
pub const PORT_BASE: u16 = 5460;

/// The last port char will hand out.
///
/// Below Linux's default ephemeral range (`32768–60999`): a block issued
/// inside it collides with whatever the kernel hands a client socket, which
/// fails intermittently and looks like anything but a port allocation bug.
pub const PORT_CEILING: u16 = 32767;

/// An inclusive span of ports reserved for one workspace.
///
/// Two integer columns rather than a packed string, because claiming is an
/// overlap query — `WHERE port_from <= ? AND port_to >= ?` — and against a
/// packed string that means parsing every row to answer it (PLAN.md §4.1.1,
/// decision 3). The field names map 1:1 onto the payload's `from` and `to`, so
/// no conversion exists to get wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct PortBlock {
    /// First port, inclusive.
    pub from: u16,
    /// Last port, inclusive. `from + size - 1`.
    pub to: u16,
}

impl PortBlock {
    /// Whether the two spans share any port.
    pub fn overlaps(&self, other: &PortBlock) -> bool {
        self.from <= other.to && other.from <= self.to
    }

    /// How many ports the span holds.
    pub fn size(&self) -> u16 {
        self.to - self.from + 1
    }
}

/// The lowest free block of `size` ports, or `None` when the machine has run
/// out.
///
/// Lowest-first rather than random or next-fit: a machine that repeatedly
/// creates and destroys worktrees then reuses the same low numbers instead of
/// walking upward forever, and a human reading `docker ps` sees stable,
/// small numbers.
pub fn choose_block(taken: &[PortBlock], size: u16) -> Option<PortBlock> {
    if size == 0 {
        return None;
    }
    let mut from = PORT_BASE;
    loop {
        let to = from.checked_add(size - 1)?;
        if to > PORT_CEILING {
            return None;
        }
        let candidate = PortBlock { from, to };
        match taken
            .iter()
            .filter(|t| t.overlaps(&candidate))
            .max_by_key(|t| t.to)
        {
            // Jump past the highest block this candidate collides with rather
            // than stepping one port at a time: with a hundred workspaces the
            // step version is quadratic for no reason.
            Some(blocker) => from = blocker.to.checked_add(1)?,
            None => return Some(candidate),
        }
    }
}

/// Assign every declared port name a port inside the block.
///
/// **Port names are a workspace-global namespace, not a per-component one.**
/// That is what makes `multi-lang`'s cross-component `${port.NAME}` reference
/// work: a check in one component may name a port declared by another, and
/// there is exactly one answer. Two components declaring the same name are
/// therefore declaring the same port, which is the intent — whether that is a
/// mistake is `config verify`'s question, not resolution's.
///
/// Assignment is sorted by name so it is stable across runs: an agent that
/// read `${port.api}` yesterday gets the same number today, and a golden
/// snapshot does not depend on map iteration order.
pub fn assign_ports(
    config: &ResolvedConfig,
    block: PortBlock,
    file: &str,
) -> Result<BTreeMap<String, u16>, CharError> {
    let mut names: Vec<&String> = config
        .components
        .values()
        .filter_map(|component| component.run.as_ref())
        .flat_map(|run| run.common().ports.keys())
        .collect();
    names.sort();
    names.dedup();

    if names.len() > block.size() as usize {
        return Err(CharError {
            class: ErrClass::BadConfig,
            r#where: ConfigWhere::Path {
                file: file.to_string(),
                path: "components.*.run.ports".to_string(),
            }
            .to_string(),
            message: format!(
                "this workspace declares {} ports and its block holds {}",
                names.len(),
                block.size()
            ),
            next_action: Some(
                "raise `port_block_size` in ~/.char/config.toml, then `char clean` and \
                 `char init` to take a new block"
                    .to_string(),
            ),
        });
    }

    Ok(names
        .into_iter()
        .enumerate()
        .map(|(i, name)| (name.clone(), block.from + i as u16))
        .collect())
}

/// What a probe found at a port, at the moment it looked.
///
/// **Probed at report time, never remembered.** A claim recorded at `init`
/// says nothing about what is bound days later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PortState {
    /// Assigned to a component, nothing bound — expected after `init` or
    /// `down`.
    Reserved,
    /// Bound by the service char started.
    Listening,
    /// Bound by something char did not start. This is the only way a port
    /// taken by a foreign process reaches a caller instead of surfacing later
    /// as a mysterious bind failure.
    Conflict,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_block_starts_at_the_base() {
        assert_eq!(
            choose_block(&[], 10),
            Some(PortBlock {
                from: PORT_BASE,
                to: PORT_BASE + 9
            })
        );
    }

    #[test]
    fn a_second_workspace_gets_the_next_block_up() {
        let taken = [PortBlock {
            from: 5460,
            to: 5469,
        }];
        assert_eq!(
            choose_block(&taken, 10),
            Some(PortBlock {
                from: 5470,
                to: 5479
            })
        );
    }

    #[test]
    fn a_hole_left_by_a_released_block_is_reused() {
        let taken = [
            PortBlock {
                from: 5460,
                to: 5469,
            },
            PortBlock {
                from: 5480,
                to: 5489,
            },
        ];
        assert_eq!(
            choose_block(&taken, 10),
            Some(PortBlock {
                from: 5470,
                to: 5479
            })
        );
    }

    #[test]
    fn a_hole_too_small_is_skipped() {
        let taken = [
            PortBlock {
                from: 5460,
                to: 5469,
            },
            PortBlock {
                from: 5475,
                to: 5484,
            },
        ];
        assert_eq!(
            choose_block(&taken, 10),
            Some(PortBlock {
                from: 5485,
                to: 5494
            })
        );
    }

    #[test]
    fn blocks_never_reach_the_ephemeral_range() {
        let taken = [PortBlock {
            from: PORT_BASE,
            to: PORT_CEILING - 3,
        }];
        assert_eq!(choose_block(&taken, 10), None);
    }

    #[test]
    fn overlap_is_inclusive_on_both_ends() {
        let a = PortBlock {
            from: 5460,
            to: 5469,
        };
        assert!(a.overlaps(&PortBlock {
            from: 5469,
            to: 5478
        }));
        assert!(!a.overlaps(&PortBlock {
            from: 5470,
            to: 5479
        }));
    }

    fn config_with_ports(yaml: &str) -> ResolvedConfig {
        let parsed = crate::config::parse(yaml, "armada.yml").expect("fixture parses");
        crate::config::resolve(parsed, &crate::config::Defaults::built_in(), "armada.yml")
            .expect("fixture resolves")
    }

    #[test]
    fn port_names_are_workspace_global_and_assigned_in_sorted_order() {
        let config = config_with_ports(
            r#"
version: 1
components:
  api:
    run: { driver: command, cmd: "./api", ports: { api: 3000 } }
  db:
    run: { driver: command, cmd: "./db", ports: { pg: 5432 } }
"#,
        );
        let assigned = assign_ports(
            &config,
            PortBlock {
                from: 5460,
                to: 5469,
            },
            "armada.yml",
        )
        .unwrap();
        assert_eq!(assigned["api"], 5460);
        assert_eq!(assigned["pg"], 5461);
    }

    #[test]
    fn one_name_declared_twice_is_one_port() {
        let config = config_with_ports(
            r#"
version: 1
components:
  a:
    run: { driver: command, cmd: "./a", ports: { web: 3000 } }
  b:
    run: { driver: command, cmd: "./b", ports: { web: 3000 } }
"#,
        );
        let assigned = assign_ports(
            &config,
            PortBlock {
                from: 5460,
                to: 5469,
            },
            "armada.yml",
        )
        .unwrap();
        assert_eq!(assigned.len(), 1);
    }

    #[test]
    fn more_ports_than_the_block_holds_is_bad_config_with_a_fix() {
        let config = config_with_ports(
            r#"
version: 1
components:
  a:
    run:
      driver: command
      cmd: "./a"
      ports: { one: 1, two: 2, three: 3 }
"#,
        );
        let err = assign_ports(
            &config,
            PortBlock {
                from: 5460,
                to: 5461,
            },
            "armada.yml",
        )
        .unwrap_err();
        assert_eq!(err.class, ErrClass::BadConfig);
        assert!(err.next_action.unwrap().contains("port_block_size"));
    }
}
