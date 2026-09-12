//! Everything a handler is given, and the constructors that build it.
//!
//! **Next door to the table rather than in it.** `routes.rs` holds the
//! inventory and the router, which is what the gate rule reads; this is state,
//! and keeping it there put that file past the length the gate refuses.

use std::sync::Arc;

use ipc::RunId;

use crate::journal::Journal;
use crate::stream::Broadcaster;

/// Everything a handler needs. Cloned per request, so nothing here may be
/// expensive to clone.
pub struct Served<D> {
    daemon: Arc<D>,
    /// **This process's** run id, minted at start and never Fleet's-by-
    /// assumption. Every error the transport raises carries it.
    run_id: RunId,
    events: Broadcaster,
    /// Where a Job's own log is read from, handed in by whoever built the
    /// listener.
    ///
    /// **Not a constructor argument**, which is the one thing worth explaining.
    /// Twenty-odd call sites build a `Served`, nearly all of them tests that
    /// exercise routes having nothing to do with a log, and widening the two
    /// constructors would have made every one of them state a reader they never
    /// call. `None` is answered by [`crate::sockets::NO_JOURNAL`] — a fault
    /// naming what is missing, never an empty stream.
    journal: Option<Arc<dyn Journal>>,
}

impl<D> Served<D> {
    /// The daemon this listener answers from, handed over.
    ///
    /// The transport is the only holder. For a caller that also has to *drive*
    /// the daemon — anything calling a turn on an interval — see
    /// [`Served::sharing`].
    pub fn by(daemon: D, run_id: RunId, events: Broadcaster) -> Served<D> {
        Served::sharing(Arc::new(daemon), run_id, events)
    }

    /// The same daemon the caller keeps a reference to.
    ///
    /// **This is what lets a Job advance.** Serving is one of two things a
    /// process does with a daemon and driving it is the other, so a
    /// constructor that consumed it left nothing in the process able to call
    /// `turn` — a Job dispatched on approval and then never settled. The state
    /// was already an `Arc` for cloning per request; this only stops that
    /// `Arc` being made where nobody else can reach it.
    ///
    /// No `Daemon` implementation for `Arc<D>` is needed for this and none is
    /// stated: the handlers reach the daemon through the state's own `Arc`, so
    /// `D` stays the concrete daemon and one indirection stays one.
    pub fn sharing(daemon: Arc<D>, run_id: RunId, events: Broadcaster) -> Served<D> {
        Served {
            daemon,
            run_id,
            events,
            journal: None,
        }
    }

    /// The reader for a Job's own log, from the side that knows where the logs
    /// are. See [`Served::journal`] for why this is not a constructor argument.
    pub fn reading(mut self, journal: Arc<dyn Journal>) -> Served<D> {
        self.journal = Some(journal);
        self
    }

    /// The stream this listener publishes from, for whoever holds the daemon.
    pub fn events(&self) -> Broadcaster {
        self.events.clone()
    }

    /// The daemon, for a handler in another module of this crate.
    pub(crate) fn daemon(&self) -> &D {
        &self.daemon
    }

    /// The daemon as the `Arc` this listener holds it by, for the commands that
    /// hand their work to a task of their own — `show_again`, `start_run`. A spawned task
    /// has to own what it runs on, and a borrow of the state does not outlive
    /// the request.
    pub(crate) fn shared(&self) -> Arc<D> {
        Arc::clone(&self.daemon)
    }

    /// The Job log reader, where one was handed in.
    pub(crate) fn journal(&self) -> Option<Arc<dyn Journal>> {
        self.journal.clone()
    }

    /// **This process's** run id, which every error the transport raises
    /// carries. Read by the handlers next door and by nothing outside.
    pub(crate) fn run_id(&self) -> &RunId {
        &self.run_id
    }
}

impl<D> Clone for Served<D> {
    fn clone(&self) -> Served<D> {
        Served {
            daemon: Arc::clone(&self.daemon),
            run_id: self.run_id.clone(),
            events: self.events.clone(),
            journal: self.journal.clone(),
        }
    }
}
