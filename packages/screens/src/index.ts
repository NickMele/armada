// A whole screen, assembled, and the readings behind it.
//
// **Props in, callbacks out.** Nothing here opens a socket, reaches for a
// preload or knows what Electron is: a screen decides what the wire means and
// what a person may do about it, and the app it is mounted in does the asking.
// That is what lets a screen be rendered, storied and tested with no daemon.
//
// The host calls a screen needs arrive as arguments — `onReadDiff`,
// `onOpenArtifact`, `onReadCall`, `onNeedMaterial`, `onStage`, `onWant`. Each
// used to be a `window.armada` call written inline, which is precisely what
// held these files inside the app.

export * from "./Acts";
export * from "./Composer";
export * from "./Decide";
export * from "./DispatchJob";
export * from "./JobDetail";
export * from "./Jobs";
export * from "./Log";
export * from "./Overrule";
export * from "./Redirect";
export * from "./Report";
export * from "./Reports";
export * from "./Row";
export * from "./Sheets";
export * from "./board";
export * from "./calls";
export * from "./chapters";
export * from "./copy";
export * from "./declared";
export * from "./detail-keys";
export * from "./duration";
export * from "./facts";
export * from "./files";
export * from "./frozen";
export * from "./keys";
export * from "./lineage";
export * from "./opening";
export * from "./phases";
export * from "./preview";
export * from "./produced";
export * from "./proposal";
export * from "./reading";
export * from "./recovery";
export * from "./render";
export * from "./review";
export * from "./run";
export * from "./steering";
export * from "./stopped";
export * from "./story";
export * from "./work";
