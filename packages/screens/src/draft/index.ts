// The draft schema: shapes the new boards draw that Fleet does not serve yet.
//
// **This is not the wire, and that is the whole point.** `@armada/protocol` is
// a hand-written mirror of `crates/ipc` (`protocol.ts`, its own header says so)
// and no gate compares the two. A type invented there is indistinguishable from
// one Fleet serves — and a Fleet whose protocol version is behind Bridge's is
// refused outright (`docs/practices/protocol.md`), so bumping the version for a
// mock would lock out the real daemon.
//
// So the new shapes live here, one layer above the wire, and every one of them
// is filled from what Fleet serves **today**. A board built on these renders
// against the real Fleet as well as the mock — thinner, never broken.

// Each file names the `crates/ipc` module it is meant for and its source of
// truth, and carries a `…Of(…)` derivation from today's wire beside the type.
// A draft moves to `@armada/protocol` only in the same pull request as its Rust
// DTO (#1530, "Decided without asking"), and #1545 is where that is reviewed.
//
// **Nothing under `apps/desktop/src/main/**` may import this.** The main
// process is the one that talks to Fleet, and a draft type reaching it is a
// shape Fleet was never asked for. `xtask`'s
// `nothing_in_the_main_process_reads_the_draft_schema` refuses it, and refuses
// re-exporting this directory from `packages/screens/src/index.ts` as well —
// a re-export would put the drafts behind the package name, where that rule
// could no longer see them.

export * from "./branches";
export * from "./cases";
export * from "./coord";
export * from "./criterion";
export * from "./dispatch";
export * from "./group";
export * from "./held";
export * from "./landing";
export * from "./ledger";
export * from "./members";
export * from "./peers";
export * from "./proposal";
export * from "./pulse";
export * from "./revision";
export * from "./sketch";
export * from "./task";
export * from "./wave";
export * from "./words";
