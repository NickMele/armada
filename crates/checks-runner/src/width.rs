//! How wide one Check may run: the worker count Armada hands a runner, and the
//! `${width}` a Manifest substitutes it into. #1444.
//!
//! **A number handed in, never a cap imposed.** Nothing stops a process
//! spawning what it likes, so the repository spells the flag —
//! `--test-threads ${width}`, `--maxWorkers=${width}` — and nothing here knows
//! nextest from vitest. `docs/concepts/manifest.md`, *How wide a Check runs*.

/// The variable every Check's command gets, for a `run` that reaches its runner
/// through a script and so has nowhere to write `${width}`.
///
/// `ARMADA_PORT_<NAME>`'s reasoning, one registry along: a command string
/// covers one channel and a `package.json` script reads the other.
pub const WIDTH_ENV: &str = "ARMADA_CHECK_WIDTH";

/// How many concurrent workers one Check may start.
///
/// **No `Default`**, for `fleet::places::ChecksAtOnce`'s reason: the shipped
/// number is a decision and `armada::serve` says what decided it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CheckWidth(u32);

impl CheckWidth {
    /// At least one: a Check allowed no worker would run nothing.
    pub const fn of(workers: u32) -> CheckWidth {
        CheckWidth(if workers == 0 { 1 } else { workers })
    }

    /// What this machine hands a Check that declares nothing.
    ///
    /// **Half the machine, divided by the Jobs sharing it.** `ChecksAtOnce`
    /// already leaves the other half to the Drones, Bridge and the person, and
    /// two Jobs at a test step is the case #1444 was filed for: two must not
    /// each claim what one was allowed. Floored at two so no machine hands out
    /// a serial test run, capped at eight.
    pub const fn for_machine(cores: usize, drones_at_once: usize) -> CheckWidth {
        let share = cores
            / 2
            / if drones_at_once == 0 {
                1
            } else {
                drones_at_once
            };
        CheckWidth::of(if share < 2 {
            2
        } else if share > 8 {
            8
        } else {
            share as u32
        })
    }

    /// This machine's number, read from the machine.
    ///
    /// **Derived rather than saved**, which is what lets `armada check test` in
    /// a terminal and the gate ruling on the same Check resolve `${width}` to
    /// the same number without either asking the other.
    pub fn read(drones_at_once: usize) -> CheckWidth {
        let cores = std::thread::available_parallelism().map_or(1, |cores| cores.get());
        CheckWidth::for_machine(cores, drones_at_once)
    }

    /// What a Check declaring `width` gets: its own number, never above the
    /// machine's.
    ///
    /// **Lower only.** A width is a claim on a machine the repository does not
    /// own, which is the opposite of a timeout — patience rather than a
    /// resource — and why `[manifest-check-timeout-raise-or-lower]` stays open
    /// beside this.
    pub fn narrowed_to(self, declared: Option<core::num::NonZeroU32>) -> CheckWidth {
        match declared {
            None => self,
            Some(asked) => CheckWidth(self.0.min(asked.get())),
        }
    }

    pub const fn get(&self) -> u32 {
        self.0
    }
}

/// Replace every `${width}` in `text` with `width`.
///
/// **Plain substitution, not shell expansion**, as `fleet::ports::resolve_ports`
/// is: `run` gets no shell, so this happens before the string reaches a
/// splitter that does not interpret `$`.
///
/// A command with no `${width}` comes back unchanged, which is a repository
/// saying this Check sizes itself — `cargo fmt --all --check` has no width to
/// be told.
pub fn resolve_width(text: &str, width: CheckWidth) -> String {
    text.replace("${width}", &width.get().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_owners_machine_hands_a_check_a_quarter_of_it() {
        // 18 logical cores, two Drones: half is nine, and two Jobs at a test
        // step get four each rather than eighteen each.
        assert_eq!(CheckWidth::for_machine(18, 2).get(), 4);
    }

    #[test]
    fn a_small_machine_still_hands_out_two() {
        assert_eq!(CheckWidth::for_machine(2, 2).get(), 2);
        assert_eq!(CheckWidth::for_machine(1, 1).get(), 2);
    }

    #[test]
    fn a_large_machine_stops_at_eight() {
        assert_eq!(CheckWidth::for_machine(128, 1).get(), 8);
    }

    #[test]
    fn a_declared_width_lowers_and_never_raises() {
        let machine = CheckWidth::of(4);
        let three = core::num::NonZeroU32::new(3).expect("three");
        let sixteen = core::num::NonZeroU32::new(16).expect("sixteen");
        assert_eq!(machine.narrowed_to(Some(three)).get(), 3);
        assert_eq!(machine.narrowed_to(Some(sixteen)).get(), 4);
        assert_eq!(machine.narrowed_to(None).get(), 4);
    }

    #[test]
    fn every_mention_is_substituted_and_a_command_without_one_is_untouched() {
        let width = CheckWidth::of(3);
        assert_eq!(
            resolve_width("vitest run --maxWorkers=${width} -j ${width}", width),
            "vitest run --maxWorkers=3 -j 3"
        );
        assert_eq!(
            resolve_width("cargo fmt --all --check", width),
            "cargo fmt --all --check"
        );
    }
}
