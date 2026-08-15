//! One verb's answer, turned into one MCP tool result.
//!
//! **The result carries the `--json` envelope of PLAN.md §3.1 and nothing
//! else**, so an MCP caller and a shell caller parse identically — the text
//! block is byte-for-byte what `armada … --json` writes to stdout, produced by
//! the same [`Envelope::to_json`] and therefore in the same reading order.
//!
//! **No `structuredContent`, deliberately.** The specification says a tool that
//! returns one SHOULD declare an `outputSchema`, and the envelope's `data` is a
//! different shape per verb — thirteen schemas that would have to be kept in
//! step with thirteen structs, to restate bytes the caller already has. Worse,
//! `structuredContent` is a `serde_json::Value`, whose map is a `BTreeMap`:
//! routing the envelope through one alphabetises it, and this corpus writes
//! every payload in reading order on purpose (`docs/traps.md`). One text block
//! is the whole answer.
//!
//! **A failed verb is a tool-level error, never a protocol error.** rmcp's own
//! guidance is that clients render a JSON-RPC error opaquely — *"Tool result
//! missing due to internal error"* — so a `bad_config` that names the line to
//! edit would reach the caller as nothing at all. `is_error` is set, the
//! envelope with its typed `error` and its `next_action` travels in the content,
//! and the agent reads exactly what a person would.

use armada_core::envelope::{Envelope, NoData};
use armada_core::error::ArmadaError;
use rmcp::model::{CallToolResult, ContentBlock};

use crate::verbs::Output;

/// Turn a verb's outcome into a tool result.
///
/// `verb` names the failing verb for the envelope §3.2.2 requires on the error
/// path — the caller has to be able to tell *which* tool refused.
pub fn from(verb: &str, outcome: Result<Output, ArmadaError>) -> CallToolResult {
    match outcome {
        Ok(output) => text(output.to_json(), output.exit_code() != 0),
        Err(error) => {
            let envelope: Envelope<NoData> = Envelope::failed(verb, None, error, NoData {});
            text(envelope.to_json(), true)
        }
    }
}

/// The envelope, as the one content block, flagged or not.
fn text(json: String, is_error: bool) -> CallToolResult {
    let content = vec![ContentBlock::text(json)];
    if is_error {
        CallToolResult::error(content)
    } else {
        CallToolResult::success(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use armada_core::error::ErrClass;

    fn body(result: &CallToolResult) -> String {
        match result.content.first() {
            Some(ContentBlock::Text(text)) => text.text.clone(),
            other => panic!("expected one text block, got {other:?}"),
        }
    }

    /// **The failure a caller can act on survives the transport.** A protocol
    /// error would have reached them as "tool result missing"; this asserts the
    /// class, the location and the next action are all still there.
    #[test]
    fn a_refused_verb_answers_the_envelope_and_is_flagged_an_error() {
        let result = from(
            "check",
            Err(ArmadaError {
                class: ErrClass::BadConfig,
                r#where: "armada.yml:components.api.checks.lint.cmd".to_string(),
                message: "no such key".to_string(),
                next_action: Some("add a `cmd:`".to_string()),
            }),
        );

        assert_eq!(result.is_error, Some(true));
        let json = body(&result);
        assert!(json.contains("\"verb\": \"check\""), "{json}");
        assert!(json.contains("\"class\": \"bad_config\""), "{json}");
        assert!(json.contains("\"next_action\": \"add a `cmd:`\""), "{json}");
    }

    /// **Reading order, not alphabetical.** The one property that would be lost
    /// silently by assembling the answer as a `serde_json::Value`.
    #[test]
    fn the_envelope_arrives_in_the_order_the_corpus_writes_it() {
        // **The verb is deliberately not `status`.** The envelope carries a
        // key called `status` *and* a verb called `status`, and a test that
        // searched for the first `"status"` in the document would find the
        // value rather than the key and assert nothing.
        let result = from(
            "up",
            Err(ArmadaError {
                class: ErrClass::Environment,
                r#where: "docker".to_string(),
                message: "the daemon is not running".to_string(),
                next_action: None,
            }),
        );

        let json = body(&result);
        let order = |key: &str| json.find(key).unwrap_or_else(|| panic!("no {key}"));
        assert!(order("schema_version") < order("\"verb\""));
        assert!(order("\"verb\"") < order("\"workspace\""));
        assert!(order("\"workspace\"") < order("\"status\""));
        assert!(order("\"status\"") < order("\"error\""));
        assert!(order("\"error\"") < order("\"data\""));
    }
}
