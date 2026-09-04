// The Fleet wire, in TypeScript, and the one thing every other package may
// import.
//
// **This package imports nothing.** It is what Fleet sends and what a read of
// it looks like, so a screen can render a Job without knowing that an Electron
// preload carried it — which is the rule that lets Screens sit above Bridge
// rather than inside it.
//
// Half of it is generated. `apps/desktop/codegen/vocabulary.mjs` writes
// `generated/` from the Rust domain registries under
// `crates/core-model/domain/`, which stay the authority: a status, a verb or a
// glyph is decided there and rendered here.

export * from "./acts";
export * from "./artifacts";
export * from "./attempt";
export * from "./connection";
export * from "./events";
export * from "./footprint";
export * from "./generated/protocol-version";
export * from "./history";
export * from "./holding";
export * from "./journal";
export * from "./proposal";
export * from "./protocol";
export * from "./proposing";
export * from "./reads";
export * from "./reclaimed";
export * from "./report";
export * from "./setup";
export * from "./turn";
export * from "./version";
export * from "./waiting";
export * from "./work";
