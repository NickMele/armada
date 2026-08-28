// The fixture data the stories beside this file arrange into a screen.
//
// **Not a component and not a story.** Nothing the app imports comes from here,
// and Storybook's glob does not pick it up. It exists because the story file
// crossed the gate's 500-line warning once the record landed on this screen —
// the same split `Panels.tsx` and `Acts.tsx` made on Bridge's side, for the
// same reason. What arranges these into a screen stays in the story file.
//
// **The recourse sentences live with the stories rather than here**, because
// each one belongs to the state its story draws and they are the one thing on
// the screen that has to read the same as `recovery.ts` on Bridge's side.

import type { LucideIcon } from "lucide-react";
import { FileCheck, ShieldCheck, ShieldMinus, ShieldX, X } from "lucide-react";
import { DroneTurns } from "../../compositions/DroneTurns/DroneTurns";
import { EvidenceTrail } from "../../compositions/EvidenceTrail/EvidenceTrail";
import type { JobRecordSection } from "../../compositions/JobRecord/JobRecord";
import { TransitionHistory } from "../../compositions/TransitionHistory/TransitionHistory";
import { UnifiedDiff } from "../../compositions/UnifiedDiff/UnifiedDiff";
import type { WorkflowRailStep } from "../../compositions/WorkflowRail/WorkflowRail";

/* `file` has no entry in `packages/icons/icons.toml`, so the log row renders a
   channel short rather than reaching for an unregistered glyph. Reported. */
const NO_GLYPH_IN_REGISTRY = undefined as unknown as LucideIcon;

export const steps: WorkflowRailStep[] = [
  {
    id: "plan",
    label: "Plan the change",
    activity: "advanced",
    status: "advanced",
    // The drawing draws no row under Plan the change here. The rail always
    // draws one. Reported.
    evidence: { icon: NO_GLYPH_IN_REGISTRY, iconLabel: "Evidence", label: "" },
  },
  {
    id: "implement",
    label: "Implement",
    activity: "advanced",
    status: "advanced",
    gates: [
      {
        command: "build · cargo build --workspace",
        result: "exit 0",
        icon: ShieldCheck,
        iconLabel: "Passed",
      },
      // The drawing draws `shield-minus` on this row, whose registry entry
      // means "not reached", beside the result "passed". A glyph is never
      // written by hand against the registry, so the row takes `shield-check`.
      // Reported as a slip in the drawing.
      {
        command: "diff_nonempty",
        result: "passed",
        icon: ShieldCheck,
        iconLabel: "Passed",
      },
    ],
  },
  {
    id: "verify",
    label: "Run tests",
    activity: "failed",
    status: "failed a check",
    gates: [
      {
        command: "test · cargo test --workspace",
        result: "exit 1",
        icon: ShieldX,
        iconLabel: "Failed",
      },
    ],
  },
  {
    id: "handoff",
    label: "Summarise",
    activity: "not_started",
    status: "not started",
    evidence: { icon: NO_GLYPH_IN_REGISTRY, iconLabel: "Evidence", label: "" },
  },
];

export const tail = [
  "running 84 tests",
  "test manifest::cache::reads_once ... FAILED",
  "test manifest::cache::invalidates_on_write ... FAILED",
  "",
  "failures:",
  "",
  "---- manifest::cache::reads_once stdout ----",
  "assertion `left == right` failed",
  "  left: 2",
  " right: 1",
  "   at core/manifest/src/cache.rs:214",
  "",
  "test result: FAILED. 82 passed; 2 failed",
].join("\n");

/* What the record folds away. Four sections and not the finished record's
   eight: the rail is the region above rather than a tab, the brief and the
   paths are the region beside it, and a footprint section would be empty on
   every stopped job — nothing serves one once the drone has gone. */
export const record: JobRecordSection[] = [
  {
    id: "moves",
    label: "Every move it made",
    panel: (
      <TransitionHistory
        emptyNote="This job has no recorded moves."
        note="Every move Fleet recorded for this job, oldest first."
        moves={[
          { seq: 1, at: "14:02:11", kind: "created", moved: "queued", actor: "you" },
          { seq: 2, at: "14:02:12", kind: "approved", moved: "queued → running", actor: "you" },
          {
            seq: 3,
            at: "14:19:40",
            kind: "step_advanced",
            subject: "implement",
            moved: "implement → verify",
            actor: "fleet",
          },
          {
            seq: 4,
            at: "14:24:52",
            kind: "escalated",
            subject: "verify",
            moved: "running → escalated",
            why: "gate_failure",
            actor: "fleet",
          },
        ]}
      />
    ),
  },
  {
    id: "turns",
    label: "The drone's turns",
    panel: (
      <DroneTurns
        emptyNote="This job has no turns."
        turns={[
          {
            id: "t1",
            at: "14:19:44",
            kind: "tool_use",
            subject: "Edit",
            detail: "core/manifest/src/cache.rs",
          },
          {
            id: "t2",
            at: "14:22:03",
            kind: "tool_use",
            subject: "Bash",
            detail: "cargo test --workspace",
          },
          {
            id: "t3",
            at: "14:24:48",
            kind: "assistant",
            said: "Two cache tests fail against the new key. Submitting anyway to get a verdict.",
          },
        ]}
      />
    ),
  },
  {
    id: "changed",
    label: "What it changed",
    panel: (
      <UnifiedDiff
        emptyNote="This job's worktree holds no change against the branch it was cut from."
        note="Read from this job's worktree against the branch it was cut from."
        files={[
          {
            path: "core/manifest/src/cache.rs",
            lines: [
              { kind: "hunk", text: "@@ -18,7 +18,9 @@ impl Cache {" },
              { kind: "context", text: "     pub fn read(&self, path: &Path) -> Manifest {" },
              { kind: "removed", text: "-        self.load(path)" },
              { kind: "added", text: "+        let key = path.canonicalize().unwrap_or_else(|_| path.into());" },
              { kind: "added", text: "+        self.entries.entry(key).or_insert_with(|| self.load(path)).clone()" },
              { kind: "context", text: "     }" },
            ],
          },
        ]}
      />
    ),
  },
  {
    id: "claims",
    label: "What the drone claimed",
    panel: (
      <EvidenceTrail
        entries={[
          {
            step: "Implement",
            provenance: "self_reported",
            icon: FileCheck,
            iconLabel: "Evidence",
            claimed: "The manifest is read once per dispatch and cached on the absolute path.",
            shownBy: "core/manifest/src/cache.rs, and the two tests beside it",
            notClaimed: "Nothing about a manifest that changes while a job is running.",
          },
        ]}
      />
    ),
  },
];

export const heading = {
  status: "completed-failed",
  statusIcon: X,
  statusLabel: "Failed",
  headline: "Cache the manifest read",
  jobId: "job_91ab",
  fields: [
    // A step name is a label, so it stays sans beside its mono siblings, and
    // the two halves are one fact joined by a comma.
    { label: "Stopped at", value: "Run tests" },
    { label: "step", value: "3 of 4", mono: true, continues: true },
    { label: "Ran", value: "22m 41s", mono: true },
    { label: "Spend, estimated", value: "~$2.10", mono: true },
    { label: "Dispatched by you" },
  ],
};

