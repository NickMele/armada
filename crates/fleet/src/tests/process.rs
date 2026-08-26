//! The identity primitive, on its own.
//!
//! Two of these three are v1's own tests by another name — a zero pid is never
//! alive, and a pid nothing holds reads as absent rather than as an error. The
//! third is the one v1 had no reason to write: two different processes with the
//! same pid do not have the same start time, which is what makes the runtime
//! file's claim about a pid a claim about a process.

use crate::process::{holder_of, Holder};

#[test]
fn a_zero_pid_is_never_held() {
    // What a half-written file reads as, and the reason v1 carried a test with
    // this name. It is also what `kill(2)` reads as "my own process group",
    // which is the worst possible thing to probe by accident.
    assert_eq!(holder_of(0).expect("no probe is taken"), Holder::Vacant);
}

#[test]
fn a_pid_the_platform_cannot_express_is_vacant_rather_than_a_probe_failure() {
    // `ps` calls this a malformed argument, not an absent process, and the two
    // must not be confused: one means "nothing is there" and the other means
    // "the check did not run".
    let above_ceiling = u32::from(u16::MAX) * u32::from(u16::MAX);
    assert_eq!(
        holder_of(above_ceiling).expect("no probe is taken"),
        Holder::Vacant
    );
}

#[test]
fn this_process_holds_its_own_pid_and_reports_a_start_time() {
    let Holder::Held(started_at) = holder_of(std::process::id()).expect("the probe runs") else {
        panic!("a running process holds its own pid");
    };
    assert!(!started_at.as_str().is_empty());
    // Stable across readings, which is what makes it usable as an identity.
    assert_eq!(
        holder_of(std::process::id()).expect("the probe runs"),
        Holder::Held(started_at)
    );
}

#[tokio::test]
async fn a_pid_that_has_been_released_is_vacant() {
    let mut child = crate::Detached::program("/bin/sh")
        .args(["-c", "exit 0"])
        .spawn()
        .expect("a shell spawns");
    let pid = child.id().expect("a spawned child has a pid");
    assert!(matches!(
        holder_of(pid).expect("the probe runs"),
        Holder::Held(_)
    ));

    child.wait().await.expect("it exits and is reaped");

    // Reaped first, then probed. A zombie is still listed, and v1 fixed the
    // same confusion three times before putting the reap in the probe's own
    // path rather than in its callers.
    assert_eq!(holder_of(pid).expect("the probe runs"), Holder::Vacant);
}
