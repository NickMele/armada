//! The three Drone tools, called as the Drone there is one of.
//!
//! The shipped path takes the Job from `crate::peer`, which reads the socket a
//! call arrived on; a fake harness holds no socket, so these say which Drone is
//! speaking by there being one. `crate::tests::concurrency` is where two are
//! told apart, and it names the Job itself.
//!
//! Free functions rather than `cfg(test)` methods on `Fleet`: a shorthand only
//! a fixture uses is a fixture's, and the production types carry no affordance
//! that exists for a test.

use ipc::mcp::{CheckReport, DeclareScope};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::evidence::Call;
use crate::{Declared, NotDeclared, NotRun, NotSubmitted, Recorded};

// **A fixture speaks as the Drone there is one of.** The shipped path takes the
// Job from `crate::peer`, which reads the socket a call arrived on; a fake
// harness holds no socket, so these say which Drone is speaking by there being
// one. `crate::tests::concurrency` is where two of them are told apart, and it
// names the Job itself.
//
// Free functions here rather than `cfg(test)` methods on `Fleet`: a shorthand
// only a fixture uses is a fixture's, and the production types carry no
// affordance that exists for a test.

pub type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// Submit as the Drone of the one Job being worked.
pub async fn submitted_by_the_one(
    fleet: &Fixture,
    call: Call<'_>,
) -> Result<Recorded, NotSubmitted> {
    let Some(job) = fleet.working_on().await.first().cloned() else {
        return Err(NotSubmitted::NothingIsWorking);
    };
    fleet.submit_evidence(&job, call).await
}

/// Declare as the Drone of the one Job being worked.
pub async fn declared_by_the_one(
    fleet: &Fixture,
    declaration: &DeclareScope,
) -> Result<Declared, NotDeclared> {
    let Some(job) = fleet.working_on().await.first().cloned() else {
        return Err(NotDeclared::NothingIsWorking);
    };
    fleet.declare_scope(&job, declaration).await
}

/// Run the Checks as the Drone of the one Job being worked.
pub async fn checked_by_the_one(fleet: &Fixture) -> Result<CheckReport, NotRun> {
    let Some(job) = fleet.working_on().await.first().cloned() else {
        return Err(NotRun::NothingIsWorking);
    };
    fleet.run_checks(&job).await
}
