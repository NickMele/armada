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
export * from "./Manifest";
export * from "./Jobs";
export * from "./Log";
export * from "./Overrule";
export * from "./RaiseCap";
export * from "./RaiseTurnCap";
export * from "./Redirect";
export * from "./Report";
export * from "./Reports";
export * from "./Row";
export * from "./Taken";
export * from "./freeze";
export * from "./Sheets";
export * from "./Worktrees";
export * from "./BridgeSettings";
export * from "./pending";
export * from "./board";
// On All repositories, the question a surface that needs one repository asks first.
export * from "./AskRepository";
export * from "./calls";
export * from "./outputs";
export * from "./chapters";
export * from "./checkout-runs";
export * from "./checkout-run-diff";
export * from "./checks";
export * from "./copy";
export * from "./editing";
export * from "./declared";
export * from "./detail-keys";
// Every question waiting on a person, from every repository, as Helm's dock draws them.
export * from "./dock-questions";
export * from "./outstanding";
export * from "./duration";
export * from "./evidence";
export * from "./facts";
export * from "./files";
export * from "./frozen";
export * from "./gates";
export * from "./held";
export * from "./keys";
export * from "./lineage";
export * from "./manifest-file";
export * from "./manifest-form";
export * from "./verify";
export * from "./notes";
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
export * from "./StepActs";
export * from "./steering";
export * from "./stopped";
export * from "./story";
export * from "./verdicts";
export * from "./waiting";
export * from "./work";
// Setup — Journey 3: the picker and a proposal over it.
export * from "./Setup";
export * from "./setup-held";
export type * from "./setup-reads";
// Locate — Journey 3's *Getting in*: a repository added by folder or cloned, then Setup.
export * from "./Locate";
export * from "./locate-reads";
// Overview — #919: the band of five readings, and what each reads.
export * from "./OverviewTiles";
export * from "./overview";
export type * from "./overview-reads";
// Overview — #920: Needs you, Running, Queued and Other, as the Board's own rows.
export * from "./OverviewLists";
export * from "./overview-lists";
