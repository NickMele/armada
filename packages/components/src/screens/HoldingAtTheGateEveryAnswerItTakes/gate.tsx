import { Button } from "../../primitives/Button/Button";
import { ActivityLog } from "../../compositions/ActivityLog/ActivityLog";
import { ChangedFiles, changedFilesSummary } from "../../compositions/ChangedFiles/ChangedFiles";
import type { JobBriefProps } from "../../compositions/JobBrief/JobBrief";
import type { JobLogReferenceRow } from "../../compositions/JobLogReference/JobLogReference";
import type { PhaseStripProps } from "../../compositions/PhaseStrip/PhaseStrip";
import type { ReviewComment } from "../../compositions/ReviewComments/ReviewComments";
import type { RunTreeStep } from "../../compositions/RunTree/RunTree";
import type { StepChapter } from "../../compositions/StepStory/StepStory";
import { JOB_STATUS } from "../../generated/vocabulary";
import { badgeOf } from "../badge";
import { BEHIND, HEADING } from "../InsideAJobOneArrangementAtEveryState/fixtures";
import {
  chapterAct,
  PRODUCED_FILES,
} from "../InsideAJobOneArrangementAtEveryState/chapters";

/**
 * The two Jobs this sheet draws, and the one difference between them.
 *
 * **The first is the sheet next door's own Job**, carried on to the step that
 * delivers: same title, same branch, same worktree, same brief. That is
 * deliberate — a reader can put the two sheets side by side and the only thing
 * that has changed is that `Land` ran and pushed.
 *
 * **The second is a Design Plan Job**, because it is the shortest way to a gate
 * with no pull request behind it. `.armada/workflows/design-plan.json` declares
 * two steps, `draft` and `present`, neither of which delivers, and `present`'s
 * gate is `human_always`. So there is no branch to push, no pull request to
 * merge and nothing anybody could have commented on — and every one of those is
 * an absence rather than a pending thing.
 */

/** The forge address the fixtures across this repository already use. */
export const PULL_REQUEST = "https://forge.invalid/org/repo/pull/4711";

/**
 * The Job's header at its delivering gate.
 *
 * **The fifth fact, and it is news from the moment it exists.** `facts.ts`
 * draws the number and never the address — a forge address is sixty characters
 * of which a person reads four — with the whole of it on the link's `title`.
 * `#4711` is `pullRequestNumber`'s answer for the address above.
 */
export const GATE_HEADING = {
  ...HEADING,
  ...badgeOf("awaiting_review", JOB_STATUS),
  fields: [
    HEADING.fields[0]!,
    HEADING.fields[1]!,
    { label: "Pull request", value: "#4711", mono: true, href: PULL_REQUEST },
    { label: "Elapsed", value: "34m 18s", mono: true },
    { label: "Spend", value: "~$3.10", mono: true },
    { label: "Turns", value: "41 of 80", mono: true },
  ],
};

/** The run once every step but the gate has cleared and the branch has gone out. */
export const RUN_AT_THE_GATE: RunTreeStep[] = [
  ...BEHIND,
  {
    id: "fix",
    label: "Fix",
    activity: "advanced",
    status: "advanced",
    elapsed: "6m 11s",
    startedAt: "14:22:07",
    facts: [],
  },
  {
    id: "regression_verify",
    label: "Regression check",
    activity: "advanced",
    status: "advanced",
    elapsed: "4m 18s",
    startedAt: "14:44:20",
    facts: [
      { label: "Checks", value: "2 of 2 passed", named: "passed" },
      { label: "Judge", value: "2 of 2 met", named: "passed" },
    ],
  },
  {
    id: "consumers",
    label: "Check the consumers still compile",
    activity: "advanced",
    status: "advanced",
    elapsed: "2m 07s",
    startedAt: "14:48:38",
    facts: [{ label: "Checks", value: "1 of 1 passed", named: "passed" }],
  },
  {
    id: "land",
    label: "Land",
    activity: "awaiting_human",
    status: "waiting on you",
    elapsed: "3m 41s",
    startedAt: "14:50:45",
    current: true,
    factsOpen: true,
    facts: [
      { label: "Pushed", value: "fix/settings-split-selectors" },
      { label: "Opened", value: "pull request #4711" },
      { label: "Waiting", value: "on you · 3m 41s" },
    ],
  },
];

/**
 * Where a delivering step stands. **No Check tier and no Judge tier, and the
 * note is what says so** — `.armada/workflows/bug.json`'s delivering step
 * declares neither, and `phases.tsx` draws no stage for a tier a step does not
 * have. A greyed-out gate would read as a gate that failed to render.
 *
 * The sentence is `noteOf`'s own, for a step that gates on nothing.
 */
export const PHASES_DELIVERED: PhaseStripProps = {
  note: "This step declares no Check and asks no Judge. Its evidence advances it, and nothing else.",
  stages: [
    { id: "instructed", label: "Instructed", state: "cleared" },
    { id: "working", label: "Working", state: "cleared" },
    { id: "submitted", label: "Submitted", state: "cleared" },
    { id: "you", label: "You", kind: "human", state: "waiting", stands: "waiting on you" },
  ],
};

/** The step's story at the gate. Three chapters, as every step has. */
export const GATE_CHAPTERS: StepChapter[] = [
  {
    id: "instructions",
    ordinal: 1,
    title: "Drone instructions",
    summary: "14:50:45",
    preview:
      "Push the branch and open a pull request against the base it was cut from. Do not merge " +
      "it and do not request a reviewer.",
  },
  {
    id: "log",
    ordinal: 2,
    title: "Activity log",
    summary: "31 entries · every line opens",
    act: chapterAct("Open the log", "L"),
    preview: (
      <ActivityLog
        entries={[
          {
            id: "g1",
            at: "14:51:12",
            actor: "drone",
            summary: "Bash",
            subject: "git push -u origin fix/settings-split-selectors",
            ran: "exit 0 · 2.41s",
          },
          {
            id: "g2",
            at: "14:52:03",
            actor: "fleet",
            summary: "Pull request #4711 opened against main.",
          },
          {
            id: "g3",
            at: "14:54:26",
            actor: "fleet",
            summary: "Step complete. The workflow asks for a person here.",
          },
        ]}
      />
    ),
  },
  {
    id: "produced",
    ordinal: 3,
    title: "Produced",
    says: "Click to view the files",
    summary: changedFilesSummary(PRODUCED_FILES),
    preview: (
      <ChangedFiles emptyNote="This drone has not changed anything yet." files={PRODUCED_FILES} />
    ),
    act: chapterAct("Open the diff", "f"),
  },
];

/**
 * What people wrote on the pull request, oldest first as the forge ordered
 * them.
 *
 * **Three comments and only two of them are change requests**, which is the
 * whole reason this is a surface a person picks on rather than a payload a
 * drone is handed. The third is somebody asking about something else entirely,
 * and a drone given it would go and do it.
 *
 * **The fourth is already sent and cannot be picked.** The forge has no memory
 * of what Armada did, so it reads exactly the same forever — it is drawn with
 * what it says and marked, rather than hidden, because a review with holes in
 * it is worse than a row a person cannot tick.
 */
export const REMARKS: ReviewComment[] = [
  {
    id: "IC_kwDOgate1",
    by: "a-reviewer",
    at: "2026-09-09 09:12",
    said: "selectColumnOrder is memoised on the whole settings slice again in the new module. That is the bug this job was dispatched for, moved rather than fixed.",
    takenUp: false,
  },
  {
    id: "IC_kwDOgate2",
    by: "a-reviewer",
    at: "2026-09-09 09:14",
    said: "index.ts re-exports the internal selectors as well as the public ones. Nothing outside the package should be able to reach selectVisibleColumnsRaw.",
    takenUp: false,
  },
  {
    id: "IC_kwDOgate3",
    by: "somebody-else",
    at: "2026-09-09 09:40",
    said: "Unrelated to this change, but is the density setting still read anywhere? I could not find a consumer.",
    takenUp: false,
  },
  {
    id: "IC_kwDOgate0",
    by: "a-reviewer",
    at: "2026-09-09 08:58",
    said: "The test file has no case for an empty column list, which is the one the board draws differently.",
    takenUp: true,
  },
];

const PLAN_JOB = "job_71c4ad";
const PLAN_WORKTREE = `.armada/worktrees/${PLAN_JOB}`;
const PLAN_DOCUMENT = `.armada/artifacts/${PLAN_JOB}/draft.md`;

/**
 * A Design Plan Job at its second pass, holding at `present`.
 *
 * **No pull request fact, and no `No branch yet` either.** The Job has a branch
 * — Fleet cuts a worktree for every Job — and nothing was ever pushed off it,
 * so `facts.ts` draws the branch and draws nothing where the pull request would
 * be. The absence is the fact.
 */
export const PLAN_HEADING = {
  ...badgeOf("awaiting_review", JOB_STATUS),
  headline: "Decide how a Job's evidence reaches the Board without a second read",
  jobId: PLAN_JOB,
  fields: [
    { value: "Design Plan", suffix: " workflow", copyValue: "design_plan" },
    {
      value: "plan/board-evidence-read",
      mono: true,
      copyValue: "plan/board-evidence-read",
    },
    { label: "Elapsed", value: "22m 06s", mono: true },
    { label: "Spend", value: "~$0.74", mono: true },
    { label: "Turns", value: "12 of 40", mono: true },
  ],
};

/**
 * The loop, on its second pass. **`draft` ran twice** — the first `present` came
 * back with changes asked for, and `verdict_routing` sent it to `draft` rather
 * than ending anything. That is the one instantiated loop Armada ships.
 */
export const PLAN_RUN: RunTreeStep[] = [
  {
    id: "draft",
    label: "Draft",
    activity: "advanced",
    status: "advanced",
    elapsed: "9m 22s",
    startedAt: "11:02:10",
    facts: [
      { label: "Attempt 1", value: "advanced", named: "advanced" },
      { label: "Attempt 2", value: "advanced", named: "advanced" },
      {
        label: "Produced",
        paths: [{ directory: `.armada/artifacts/${PLAN_JOB}/`, basename: "draft.md" }],
      },
    ],
  },
  {
    id: "present",
    label: "Present",
    activity: "awaiting_human",
    status: "waiting on you",
    elapsed: "4m 55s",
    startedAt: "11:19:41",
    current: true,
    factsOpen: true,
    facts: [
      { label: "Iteration", value: "2 of 5" },
      { label: "Waiting", value: "on you · 4m 55s" },
    ],
  },
];

/** The same tiers as a delivering step: none of them mechanical. */
export const PLAN_PHASES: PhaseStripProps = {
  note: "This step declares no Check and asks no Judge. Its evidence advances it, and nothing else.",
  stages: [
    { id: "instructed", label: "Instructed", state: "cleared" },
    { id: "working", label: "Working", state: "cleared" },
    { id: "submitted", label: "Submitted", state: "cleared", opens: true },
    { id: "you", label: "You", kind: "human", state: "waiting", stands: "waiting on you" },
  ],
};

export const PLAN_WHERE: JobLogReferenceRow[] = [
  { iconLabel: "Worktree", value: PLAN_WORKTREE, copyValue: PLAN_WORKTREE },
  {
    iconLabel: "Branch",
    value: "plan/board-evidence-read",
    copyValue: "plan/board-evidence-read",
  },
  { iconLabel: "Manifest", value: "armada.yml", copyValue: "armada.yml", separated: true },
  {
    iconLabel: "Workflow",
    value: "design_plan",
    copyValue: "design_plan",
    meta: "as it was at 11:00",
  },
];

export const PLAN_BRIEF: JobBriefProps = {
  facts:
    "The Board reads a Job's evidence a second time to draw the row, which is the read that " +
    "makes a hundred rows cost a hundred queries.",
  criteria: [],
  only: "facts",
  factsLabel: null,
};

/**
 * The Design Plan step's story. **The product is a document and not a patch**,
 * so Produced says the repository did not change and names what the step wrote
 * instead — `chapters.tsx`'s own sentence for a step whose product is a
 * document, and the reason `0 files` is not a step that produced nothing.
 */
export const PLAN_CHAPTERS: StepChapter[] = [
  {
    id: "instructions",
    ordinal: 1,
    title: "Drone instructions",
    summary: "11:19:41",
    preview:
      "Present the draft as it stands, with what changed since the last pass called out at the " +
      "top. Do not start a third pass.",
  },
  {
    id: "log",
    ordinal: 2,
    title: "Activity log",
    summary: "18 entries · every line opens",
    act: chapterAct("Open the log", "L"),
    preview: (
      <ActivityLog
        entries={[
          {
            id: "p1",
            at: "11:20:58",
            actor: "drone",
            summary: "Write",
            subject: PLAN_DOCUMENT,
            ran: "7,605 bytes",
          },
          {
            id: "p2",
            at: "11:24:12",
            actor: "fleet",
            summary: "Step complete. The workflow asks for a person here.",
          },
        ]}
      />
    ),
  },
  {
    id: "produced",
    ordinal: 3,
    title: "Produced",
    // **`0 files` is not a step that produced nothing**, which is why the
    // documents are their own segment rather than folded into the file count:
    // a path deliberately outside the patch inside the number that measures the
    // patch reads as a step that did nothing.
    summary: "1 document",
    preview: (
      <>
        <ChangedFiles
          files={[]}
          emptyNote="Nothing in the repository changed. This step's product is the document below."
        />
        {/* `Documents`, as `chapters.tsx` draws it: a sub-label, one control
            per file naming its basename, and the sentence that reconciles the
            count above with the document under it. Drawn here rather than
            imported, because it is `@armada/screens`'s markup and a component
            cannot reach across that seam. */}
        <div>
          <span className="text-2xs text-fg-muted">Documents this step wrote</span>
          <span>
            <Button variant="ghost" size="sm" title={PLAN_DOCUMENT}>
              draft.md
            </Button>
            <span className="text-2xs text-fg-subtle">attempt 2</span>
          </span>
          <p className="text-2xs text-fg-subtle">
            Kept outside the diff, so the count above does not include them. One per attempt.
          </p>
        </div>
      </>
    ),
  },
];
