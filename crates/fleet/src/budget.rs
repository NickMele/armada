//! Racing a plain command's work against its own budget, spawned rather than
//! merely timed.
//!
//! **Split out of `commanding` at the 900-line refusal, `#897`.** [`budgeted`]
//! and [`budgeted_for`] are generic over the answer and reach for nothing
//! `commanding` owns beyond [`CommandBudget`](crate::commanding::CommandBudget)
//! itself, so where the race lives and where the type it carries lives are two
//! different questions — the type stays put, because `Fleet::command_budget`
//! and the spawn configuration both already carry it by that path.

use std::future::Future;

use ipc::JobId;

use crate::adrift::Adrift;
use crate::commanding::CommandBudget;

/// Race a plain command's work against [`CommandBudget`], spawned rather than
/// merely timed: a losing race stops waiting without cancelling a write
/// already in progress.
pub(crate) async fn budgeted<T>(
    budget: CommandBudget,
    work: impl Future<Output = Result<T, Adrift>> + Send + 'static,
) -> Result<T, Adrift>
where
    T: Send + 'static,
{
    let waited = budget.duration();
    match tokio::time::timeout(waited, tokio::spawn(work)).await {
        Ok(Ok(answered)) => answered,
        // Resumed rather than folded into a refusal a caller might retry: a
        // panic has already unwound past every lock it held.
        Ok(Err(panicked)) => std::panic::resume_unwind(panicked.into_panic()),
        Err(_elapsed) => Err(Adrift::CommandTimedOut { job: None, waited }),
    }
}

/// [`budgeted`], naming the Job a losing race's refusal is about.
pub(crate) async fn budgeted_for<T>(
    budget: CommandBudget,
    job: JobId,
    work: impl Future<Output = Result<T, Adrift>> + Send + 'static,
) -> Result<T, Adrift>
where
    T: Send + 'static,
{
    match budgeted(budget, work).await {
        Err(Adrift::CommandTimedOut { waited, .. }) => Err(Adrift::CommandTimedOut {
            job: Some(job.to_domain()),
            waited,
        }),
        answered => answered,
    }
}
