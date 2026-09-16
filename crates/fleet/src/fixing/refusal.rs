//! Why no fix was drafted, in the words the Drone reads.

use std::fmt;

use ipc::mcp::NotRecorded;

/// Why no fix was drafted. **Every one reaches the Drone as words it can act
/// on**, and none of them stops or advances its step.
#[derive(Debug)]
pub enum NotFixed {
    NothingIsWorking,
    NoSuchCheck {
        check: String,
    },
    NoWayToRunOneTest {
        check: String,
    },
    NotOneArgument {
        test: String,
    },
    AlreadyRunning,
    AlreadySubmitted,
    Unheard,
    MainIsBusy,
    Spent {
        allowed: u32,
    },
    NoMain {
        why: String,
    },
    NeverRan,
    PassesOnMain {
        test: String,
    },
    /// The name matched nothing on main — nextest and vitest both say so in
    /// their own summary rather than the exit code, which reports the same
    /// code a pass would. #1204.
    NoMatch {
        test: String,
    },
    NotDrafted {
        why: String,
    },
}

impl fmt::Display for NotFixed {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotFixed::NothingIsWorking => out.write_str(
                "no Job is being worked, so there is no Check to say a test is broken under. \
                 Stop — the Job this Drone was started for has already ended",
            ),
            NotFixed::NoSuchCheck { check } => write!(
                out,
                "`{check}` is not one of the Checks on the part you are on. Name the Check \
                 the test failed under, as your part's Checks name it"
            ),
            NotFixed::NoWayToRunOneTest { check } => write!(
                out,
                "`{check}` does not say how to run one test by name, so Fleet cannot run \
                 just that test on main and nothing is drafted. Say in your evidence what \
                 you found"
            ),
            NotFixed::NotOneArgument { test } => write!(
                out,
                "`{test}` cannot be passed as one argument, so it cannot be run by name. \
                 Copy the test's name without quotes"
            ),
            NotFixed::AlreadyRunning => out.write_str(
                "Fleet is already running something for you — your checks, or a test on \
                 main. Wait for its later turn, then ask again",
            ),
            NotFixed::AlreadySubmitted => out.write_str(
                "you have submitted, and the checks are about to be run against your work, \
                 so the test was not run on main. Say in your evidence what you found",
            ),
            NotFixed::Unheard => out.write_str(
                "Fleet restarted while this part was going and can no longer send you a \
                 later turn, so the test was not run on main and nothing is drafted. Say in \
                 your evidence what you found",
            ),
            NotFixed::MainIsBusy => out.write_str(
                "Fleet is already running a test against main for this repository. Carry on \
                 with your part and ask again in a few minutes",
            ),
            NotFixed::Spent { allowed } => write!(
                out,
                "this part has already asked for {}. No more are drafted from this part — \
                 say in your evidence what you found",
                match allowed {
                    1 => String::from("a fix"),
                    n => format!("{n} fixes"),
                }
            ),
            NotFixed::NoMain { why } => {
                write!(
                    out,
                    "the test could not run on main: {why}. Nothing is drafted"
                )
            }
            NotFixed::NeverRan => out.write_str(
                "the test did not run on main, so nothing about main is known and nothing is \
                 drafted",
            ),
            NotFixed::PassesOnMain { test } => write!(
                out,
                "`{test}` passes on main, so the failure is in your change. Nothing is \
                 drafted; fix it on your branch"
            ),
            NotFixed::NoMatch { test } => write!(
                out,
                "`{test}` matched nothing at all on main — neither a pass nor a failure, so \
                 nothing is drafted. Check the name against the Check's own output and ask \
                 again if it was copied wrong"
            ),
            NotFixed::NotDrafted { why } => write!(out, "the fix Job could not be drafted: {why}"),
        }
    }
}

impl From<NotFixed> for NotRecorded {
    fn from(why: NotFixed) -> NotRecorded {
        NotRecorded {
            because: why.to_string(),
        }
    }
}
