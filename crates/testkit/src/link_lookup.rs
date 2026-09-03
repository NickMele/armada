//! A [`LinkLookup`] scripted by a test, standing in for whatever real one
//! `fleet` is assembled with.
//!
//! No link shape lives here, deliberately: recognising a real one is
//! `adapters`' job, tested there against real link text. A fixture that
//! matched real links would be asserting `fleet`'s own runner against a
//! shape only `adapters` is supposed to know.

use std::sync::Mutex;

use adapter_traits::{LinkLookup, LookupCall};

/// Matches every request against one fragment and answers with one script.
///
/// `prints`, when given, rides on the argument list rather than inside the
/// script text — the same reason [`crate::FakeJudge`] never interpolates an
/// answer into its shell string: a fixture's text should not have to be shell
/// quoting-safe to be usable here.
#[derive(Debug)]
pub struct FakeLinkLookup {
    fragment: Option<&'static str>,
    script: &'static str,
    prints: Option<String>,
    calls: Mutex<usize>,
}

impl FakeLinkLookup {
    /// Resolves nothing. What most fixtures want: the request the test wrote
    /// carries no link at all, and this is the fake saying so.
    pub fn resolving_nothing() -> FakeLinkLookup {
        FakeLinkLookup {
            fragment: None,
            script: "",
            prints: None,
            calls: Mutex::new(0),
        }
    }

    /// Any request containing `fragment` resolves to a call that prints
    /// `text` on success.
    pub fn resolving(fragment: &'static str, text: &str) -> FakeLinkLookup {
        FakeLinkLookup {
            fragment: Some(fragment),
            script: "printf %s \"$0\"",
            prints: Some(text.to_string()),
            calls: Mutex::new(0),
        }
    }

    /// Any request containing `fragment` resolves to a call that fails —
    /// the network blip, the private repository, the deleted issue.
    pub fn failing_to_resolve(fragment: &'static str) -> FakeLinkLookup {
        FakeLinkLookup {
            fragment: Some(fragment),
            script: "exit 7",
            prints: None,
            calls: Mutex::new(0),
        }
    }

    /// How many times `resolve` matched and rendered a call. What a test
    /// asserts to say the fetch ran exactly once per request, not once per
    /// Job a multi-Job plan mints from it.
    pub fn resolved_count(&self) -> usize {
        *self.calls.lock().expect("not poisoned")
    }
}

impl LinkLookup for FakeLinkLookup {
    fn resolve(&self, request: &str) -> Option<LookupCall> {
        let fragment = self.fragment?;
        if !request.contains(fragment) {
            return None;
        }
        *self.calls.lock().expect("not poisoned") += 1;
        let mut args = vec!["-c".to_string(), self.script.to_string()];
        if let Some(text) = &self.prints {
            args.push(text.clone());
        }
        Some(LookupCall::rendered("/bin/sh", args))
    }
}
