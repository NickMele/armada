import type { Meta, StoryObj } from "@storybook/react-vite";
import { Button } from "../../primitives/Button/Button";
import { InsideAJob } from "./InsideAJobOneArrangementAtEveryState";
import {
  BRIEF,
  CHAPTERS,
  ESCALATED_HEADING,
  FAILED_HEADING,
  HEADING,
  REPAIR_CHAPTERS,
  RUN_FAILED,
  RUN_REPAIRING,
  RUN_RUNNING,
  RUN_STOPPED,
  RUN_WAITING,
  WHERE,
} from "./fixtures";

/**
 * **One Job, six moments.** Every story below is the same Bug workflow at a
 * different point, on purpose: the claim the screen makes is that the
 * arrangement does not move between states, and six unrelated fixtures could
 * not test it. Read them in order and nothing but the content changes.
 *
 * The one region that is not the same shape twice is the panel's `before`
 * block, and that is the design: what changes between states is which chapter
 * is the reason you are here, and what the panel offers you to do about it.
 */
const meta: Meta<typeof InsideAJob> = {
  title: "Screens/Inside a job — one arrangement at every state",
  component: InsideAJob,
};
export default meta;

type Story = StoryObj<typeof InsideAJob>;

/** The acts that end or replace the Job. Pilot lands left of Kill — #250. */
const JOB_ACTS = (
  <>
    <Button variant="ghost">Kill</Button>
  </>
);

/**
 * A Drone working. **Nothing has been submitted, so no gate has been asked
 * anything yet** — and the strip says exactly that rather than drawing three
 * empty gates.
 */
export const Running: Story = {
  render: () => (
    <div className="armada-screen">
      <InsideAJob
        heading={{ ...HEADING, actions: JOB_ACTS }}
        run={RUN_RUNNING}
        runElapsed="11m 03s"
        where={WHERE}
        whereNote="A path opens where it lives; an identifier copies. This milestone is about never needing these — they are here for when you want them anyway."
        brief={BRIEF}
        step={{
          label: "Fix",
          fields: [
            { label: "Running for", value: "6m 11s", mono: true },
            { label: "Attempt", value: "1", mono: true },
          ],
          acts: (
            <>
              <Button variant="secondary">Restart step</Button>
              <Button variant="primary">Redirect</Button>
            </>
          ),
          phases: {
            note: "The Drone is working. Nothing has been submitted, so no gate has been asked anything yet.",
            stages: [
              { id: "instructed", label: "Instructed", state: "cleared" },
              { id: "working", label: "Working", state: "current" },
              { id: "submitted", label: "Submitted", state: "ahead" },
              {
                id: "checks",
                label: "build, test",
                kind: "checks",
                state: "ahead",
                stands: "not run",
                rows: [
                  { label: "cargo build --workspace --locked", mono: true, result: "not run" },
                  { label: "cargo nextest run --workspace", mono: true, result: "not run" },
                ],
              },
              {
                id: "judge",
                label: "Judge · 2 criteria",
                kind: "judge",
                state: "ahead",
                stands: "not reached",
                rows: [
                  { label: "Selectors import without the store", result: "not reached" },
                  { label: "No behaviour change in the reducer", result: "not reached" },
                ],
              },
              { id: "you", label: "You", kind: "human", state: "ahead" },
            ],
          },
          chapters: CHAPTERS,
        }}
      />
    </div>
  ),
};

/**
 * **Waiting on you — everything mechanical cleared.** Amber, never red: the Job
 * is stopped and that is the workflow working. The decision sits at the end of
 * the story rather than in the header, because you make it after reading — and
 * the header is for acts that change what a Drone is doing. `Restart step`
 * stays up there; it interrupts rather than concludes.
 */
export const WaitingOnYou: Story = {
  render: () => (
    <div className="armada-screen">
      <InsideAJob
        heading={{
          ...HEADING,
          status: "awaiting_review",
          statusLabel: "Waiting on you",
          actions: JOB_ACTS,
        }}
        run={RUN_WAITING}
        runElapsed="13m 47s"
        where={WHERE}
        brief={BRIEF}
        step={{
          label: "Regression check",
          fields: [
            { label: "Waiting", value: "2m 04s", mono: true },
            { label: "Took", value: "4m 18s", mono: true },
            { label: "Attempt", value: "1", mono: true },
          ],
          acts: <Button variant="secondary">Restart step</Button>,
          notice: {
            tone: "waiting",
            title: "Nothing is wrong. The workflow asks for a person here.",
            children:
              "The suite passed and the Judge met both criteria. Nothing advances until you answer.",
          },
          phases: {
            note: "The suite passed and the Judge met both criteria. Nothing is wrong; the workflow asks for a person here.",
            stages: [
              { id: "instructed", label: "Instructed", state: "cleared" },
              { id: "working", label: "Working", state: "cleared" },
              { id: "submitted", label: "Submitted", state: "cleared" },
              {
                id: "checks",
                label: "build, test",
                kind: "checks",
                state: "cleared",
                stands: "2 of 2 passed",
                rows: [
                  { label: "cargo build --workspace --locked", mono: true, result: "exit 0 · 47s", named: "passed" },
                  { label: "cargo nextest run --workspace", mono: true, result: "exit 0 · 1m 22s", named: "passed" },
                ],
              },
              {
                id: "judge",
                label: "Judge · 2 of 2 met",
                kind: "judge",
                state: "cleared",
                stands: "2 of 2 met",
                rows: [
                  { label: "Selectors import without the store", result: "met", named: "met" },
                  { label: "No behaviour change in the reducer", result: "met", named: "met" },
                ],
              },
              { id: "you", label: "You", kind: "human", state: "waiting", stands: "waiting · 2m 04s" },
            ],
          },
          chapters: [
            ...CHAPTERS,
            {
              id: "decision",
              ordinal: 4,
              title: "Your decision",
              summary: "nothing advances until you answer",
              preview:
                "Approve, or send it back with a note. Send back returns it to this step; reject ends the Job. A note is optional on approve.",
            },
          ],
          after: (
            <div className="armada-screen__actions">
              <Button variant="primary">Approve</Button>
              <Button variant="secondary">Send back</Button>
              <Button variant="destructive">Reject</Button>
            </div>
          ),
        }}
      />
    </div>
  ),
};

/**
 * **A Check failed and the Drone is fixing it — the Job is not over.** Nothing
 * here asks anything of you, which is the point: a failing test is work, and
 * the Drone that wrote the code is the thing that should fix it. The band is
 * red because a Check failed, not because the Job is in trouble.
 *
 * **This is ahead of Fleet on purpose.** `docs/concepts/job.md` says a failed
 * mechanical Check ends the Job at `completed_failed`, and
 * `crates/fleet/src/tests/retrying.rs` asserts it. The contradiction is named
 * in the journey and the change is Recovery's, not this screen's.
 */
export const ACheckFailed: Story = {
  render: () => (
    <div className="armada-screen">
      <InsideAJob
        heading={{ ...HEADING, actions: JOB_ACTS }}
        run={RUN_REPAIRING}
        runElapsed="15m 20s"
        where={WHERE}
        brief={BRIEF}
        step={{
          label: "Regression check",
          fields: [
            { label: "Running for", value: "1m 09s", mono: true },
            { label: "Attempt", value: "2 of 3", mono: true },
            { label: "First failed", value: "14:47:11", mono: true },
          ],
          acts: (
            <>
              <Button variant="secondary">Restart step</Button>
              <Button variant="primary">Redirect</Button>
            </>
          ),
          notice: {
            tone: "failed",
            title: "The suite failed, and the Drone has been given the output to fix.",
            children:
              "cargo nextest run --workspace exited 101 with 3 failures. Attempt 2 of 3 is running. Nothing needs you unless it runs out of attempts.",
          },
          phases: {
            note: "The Check went back to the Drone with its output. The tiers behind it are still ahead, not cancelled.",
            stages: [
              { id: "instructed", label: "Instructed", state: "cleared" },
              { id: "working", label: "Working", state: "current" },
              { id: "submitted", label: "Submitted", state: "cleared" },
              {
                id: "checks",
                label: "test failed · fixing",
                kind: "checks",
                state: "failed",
                stands: "exit 101 · attempt 2 of 3",
                rows: [
                  { label: "cargo build --workspace --locked", mono: true, result: "exit 0 · 47s", named: "passed" },
                  { label: "cargo nextest run --workspace", mono: true, result: "exit 101 · 3 failures", named: "failed" },
                ],
              },
              { id: "judge", label: "Judge · 2 criteria", kind: "judge", state: "ahead", stands: "not reached" },
              { id: "you", label: "You", kind: "human", state: "ahead" },
            ],
          },
          chapters: REPAIR_CHAPTERS,
        }}
      />
    </div>
  ),
};

/**
 * **Out of attempts — now it needs you, and the levers already exist.** Three
 * different fixes, one unchanged failure: it is caching in the wrong place, not
 * caching wrongly, which is the thing a person sees in ten seconds and the
 * Drone could not see in three attempts.
 *
 * **The redirect box comes last, after the failure and the attempts**, because
 * you cannot write a useful sentence until you have read them.
 *
 * **No Judge was involved, and the strip says so precisely.** A Check that
 * fails ends the step before the Judge reads anything, so there is no verdict
 * to show — the tier was never reached.
 */
export const OutOfAttempts: Story = {
  render: () => (
    <div className="armada-screen">
      <InsideAJob
        heading={{ ...ESCALATED_HEADING, actions: JOB_ACTS }}
        run={RUN_STOPPED}
        runElapsed="21m 55s"
        where={WHERE}
        brief={BRIEF}
        step={{
          label: "Regression check",
          fields: [
            { label: "Held for", value: "6m 40s", mono: true },
            { label: "Attempts", value: "3 of 3", mono: true },
            { label: "Drone", value: "alive, idle" },
          ],
          acts: (
            <>
              <Button variant="secondary">Restart step</Button>
              <Button variant="primary">Redirect</Button>
            </>
          ),
          notice: {
            tone: "stopped",
            title: "Three attempts at the same failure. The Drone is holding, waiting on you.",
            children:
              "The same test has failed each time — visible_manifests_memoises. The Drone still has its session and its worktree, so a word from you costs no respawn.",
          },
          phases: {
            note: "A Check that fails ends the step before the Judge reads anything, so there is no verdict here. The Judge tier was never reached.",
            stages: [
              { id: "instructed", label: "Instructed", state: "cleared" },
              { id: "working", label: "Working", state: "cleared" },
              { id: "submitted", label: "Submitted", state: "cleared" },
              {
                id: "checks",
                label: "test failed · retries spent",
                kind: "checks",
                state: "failed",
                stands: "exit 101 · 3 of 3 attempts",
                rows: [
                  { label: "cargo build --workspace --locked", mono: true, result: "exit 0 · 47s", named: "passed" },
                  { label: "cargo nextest run --workspace", mono: true, result: "exit 101 · same failure ×3", named: "failed" },
                ],
              },
              { id: "judge", label: "Judge · 2 criteria", kind: "judge", state: "ahead", stands: "not reached" },
              { id: "you", label: "You", kind: "human", state: "ahead" },
            ],
          },
          before: (
            <>
              <div className="armada-screen__sunken">
                <span className="armada-screen__eyebrow">The failure, every time</span>
                <pre className="armada-screen__output">{`FAIL settings::selectors::visible_manifests_memoises
  assert_eq!(a, b) — expected the same reference on repeat calls
  left:  Manifests([..]) @0x7f9c2a
  right: Manifests([..]) @0x7f9c31
  packages/settings/test/selectors.test.ts:112`}</pre>
                <p className="armada-screen__caption" data-note>
                  The same assertion, at the same line, on all three attempts.
                </p>
              </div>
              <div className="armada-screen__sunken">
                <span className="armada-screen__eyebrow">What it tried, and what it said it was doing</span>
                <p className="armada-screen__why">
                  Attempt 1 · +18 −4 selectors.ts · same failure — memoised on the selector itself
                  with a module-level cache.
                </p>
                <p className="armada-screen__why">
                  Attempt 2 · +22 −18 selectors.ts · same failure — replaced the cache with a WeakMap
                  keyed on the state object.
                </p>
                <p className="armada-screen__why">
                  Attempt 3 · +6 −22 selectors.ts · same failure — went back to the module cache and
                  widened the key.
                </p>
                <p className="armada-screen__recourse">
                  Three different fixes, one unchanged failure. It is caching in the wrong place, not
                  caching wrongly.
                </p>
              </div>
            </>
          ),
          chapters: [
            { ...REPAIR_CHAPTERS[0]!, summary: "14:44:20" },
            { ...REPAIR_CHAPTERS[1]!, summary: "126 entries · three attempts" },
            { ...REPAIR_CHAPTERS[2]!, summary: "4 files · on the branch" },
          ],
          after: (
            <div className="armada-screen__sunken">
              <span className="armada-screen__eyebrow">
                Tell it what it is missing — the Drone carries on, no attempt spent
              </span>
              <p className="armada-screen__why">
                Before it stopped, the Drone was asked what it would try next. Picking one drafts the
                instruction; it stays yours to edit, and writing your own from nothing is always
                available.
              </p>
              <div className="armada-screen__actions">
                <Button variant="primary">Redirect</Button>
                <Button variant="secondary">Restart step</Button>
                <Button variant="ghost">Redispatch</Button>
                <Button variant="destructive">Kill</Button>
              </div>
            </div>
          ),
        }}
      />
    </div>
  ),
};

/**
 * **Failed — the Job is over.** Hued in both channels on the tree, because
 * failed is an outcome rather than a position. Nothing below the step ever ran,
 * and the tree shows that by having nothing below it.
 *
 * The panel keeps every region and every one of them is still answerable — that
 * is what "one arrangement" costs nothing to hold on a dead Job.
 */
export const Failed: Story = {
  render: () => (
    <div className="armada-screen">
      <InsideAJob
        heading={{ ...FAILED_HEADING, actions: <Button variant="ghost">Redispatch</Button> }}
        run={RUN_FAILED}
        runElapsed="13m 54s"
        pulsing={false}
        where={WHERE}
        whereNote="The worktree and the branch are left in place. Nothing was rolled back."
        brief={BRIEF}
        step={{
          label: "Regression check",
          fields: [
            { label: "Took", value: "2m 51s", mono: true },
            { label: "Attempt", value: "1", mono: true },
            { label: "Drone", value: "gone" },
          ],
          notice: {
            tone: "failed",
            title: "A Check failed and the Job ended at completed_failed.",
            children:
              "cargo nextest run --workspace exited 101. The Judge was never reached, and nothing below this step ran.",
          },
          phases: {
            note: "Nothing advances this Job. Redispatch mints a replacement; it does not reopen this one.",
            stages: [
              { id: "instructed", label: "Instructed", state: "cleared" },
              { id: "working", label: "Working", state: "cleared" },
              { id: "submitted", label: "Submitted", state: "cleared" },
              {
                id: "checks",
                label: "test failed",
                kind: "checks",
                state: "failed",
                stands: "exit 101",
                rows: [
                  { label: "cargo build --workspace --locked", mono: true, result: "exit 0 · 47s", named: "passed" },
                  { label: "cargo nextest run --workspace", mono: true, result: "exit 101", named: "failed" },
                ],
              },
              { id: "judge", label: "Judge · 2 criteria", kind: "judge", state: "ahead", stands: "not reached" },
            ],
          },
          chapters: REPAIR_CHAPTERS,
        }}
      />
    </div>
  ),
};

/**
 * **What the wire does not carry, said where it would have gone.** A hole that
 * names its cause is a finding; one that reads "coming soon" is not — and a
 * region that closes up reads as a screen that is finished.
 */
export const NothingServesTheStep: Story = {
  render: () => (
    <div className="armada-screen">
      <InsideAJob
        heading={{ ...HEADING, actions: JOB_ACTS }}
        run={[]}
        runAbsent="Fleet did not answer for this Job, so its steps are unknown."
        where={undefined}
        whereAbsent="Nothing serves this Job's paths, and no branch exists yet."
        brief={undefined}
        briefAbsent="Nothing serves this Job's brief or its acceptance criteria."
        step={undefined}
        stepAbsent="No step is open, because the run could not be read."
      />
    </div>
  ),
};
