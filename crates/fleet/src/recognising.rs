//! Links already on a Studio, read as what their addresses name. `#1394`.
//!
//! **A Link nothing converted would offer no Dispatch while the same address
//! pasted today is an Issue** — one address, two behaviours, which is the
//! defect `#1394` is about. So every Link an adapter recognises moves.
//!
//! **Fleet does it because `store` may not**: a `LIKE` over a forge's host
//! written into V79 is a rule `verify-foundations` refuses in that crate. The
//! migration widens the `CHECK`; this reads the rows.
//!
//! **Run on every boot, not once.** It converges — a paste writes its kind
//! directly — so there is no flag to keep and a restored backup is caught the
//! same way. Nothing is touched: a conversion is nobody's write, and touching
//! every Studio would reorder the list on first start.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

use crate::daemon::Fleet;
use crate::studios::recognised;

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
    /// Every Link whose address an adapter recognises, written back as the
    /// kind it names. Answers how many moved.
    ///
    /// **A Link nothing recognises is left exactly as it is**, which is what
    /// happens to a board, a page or a session. **Nothing to log under**, as
    /// `scouts_left_gathering` has it: a Studio has no Job's log, and a store
    /// that will not read now answers the next read the same way.
    pub(crate) async fn links_recognised(&self) -> usize {
        let mut store = self.store().lock().await;
        let Ok(links) = store.every_link() else {
            return 0;
        };
        let mut moved = 0;
        for (studio, node) in links {
            let content = recognised(node.content().clone());
            if content.kind() == node.kind() {
                continue;
            }
            // `recognised` answered with one of the three at the same address,
            // which is exactly what `StudioNode::recognised` admits.
            let Some(recognised) = node.recognised(content) else {
                continue;
            };
            if store.recognise_studio_node(&studio, &recognised).is_ok() {
                moved += 1;
            }
        }
        moved
    }
}
