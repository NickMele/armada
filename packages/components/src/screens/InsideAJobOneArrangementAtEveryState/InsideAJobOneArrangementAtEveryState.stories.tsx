import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";
import { Button } from "../../primitives/Button/Button";
import { ActivityLog } from "../../compositions/ActivityLog/ActivityLog";
import { JobResources } from "../../compositions/JobResources/JobResources";
import { InsideAJob } from "./InsideAJobOneArrangementAtEveryState";
import {
  BRIEF,
  CHAPTERS,
  ESCALATED_HEADING,
  EXAMINED_WEDGED,
  EXAMINED_WORKING,
  FAILED_HEADING,
  FLEET_PREPARING,
  HEADING,
  HOLDS_AFTER_THE_END,
  HOLDS_AT_THE_GATE,
  HOLDS_IDLE,
  HOLDS_RUNNING,
  HOLDS_WEDGED,
  REPAIR_CHAPTERS,
  RUN_FAILED,
  RUN_NOT_STARTED,
  RUN_REPAIRING,
  RUN_RUNNING,
  RUN_STOPPED,
  RUN_WAITING,
  WAITING_HEADING,
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

/**
 * What the Job holds on this machine, above the Fleet log and above the run.
 *
 * **Six of the seven states carry one, and the seventh is the point.** The
 * region draws nothing when it is absent, so a story without one proves that —
 * and `A check failed` is the state where nothing about the machine is in
 * question, because a Drone is working and the log below says which line it is
 * on. Everywhere else the reading answers something the rest of the screen
 * cannot: whether the process behind `Drone alive, idle` exists, whether an
 * empty process list is a Job at its gate or a Job that has died, and how much
 * disk a finished Job is still holding.
 */
const nothingPressedYet = () => {};

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
        machine={
          <JobResources
            reading={HOLDS_RUNNING}
            age="3s"
            examined={EXAMINED_WORKING}
            onExamine={nothingPressedYet}
          />
        }
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
        heading={{ ...WAITING_HEADING, actions: JOB_ACTS }}
        run={RUN_WAITING}
        runElapsed="13m 47s"
        machine={
          <JobResources
            reading={HOLDS_AT_THE_GATE}
            age="8s"
            examined={null}
            onExamine={nothingPressedYet}
          />
        }
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
        machine={
          <JobResources
            reading={HOLDS_IDLE}
            age="4s"
            examined={null}
            onExamine={nothingPressedYet}
          />
        }
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
        machine={
          <JobResources
            reading={HOLDS_AFTER_THE_END}
            age="1m"
            examined={null}
            onExamine={nothingPressedYet}
          />
        }
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
 * **Nothing has started, and something is happening.** Armada is cutting a
 * worktree and running the repository's preparation commands; every step is
 * `not_started` and no Drone exists, so the step's own activity log is empty
 * and correct to be. What Armada has done sits above the run — #437. **Above
 * the steps and not under the first one**: attaching these to the step about
 * to start reads as though it were running when it has not begun, which is the
 * confusion that made a wedged Job look healthy.
 *
 * **And it is the state where nothing else on the screen says the Job is
 * dead.** The badge reads `running`, the tree reads `not started`, and both are
 * true — which is exactly what a person read for six minutes on 4 Sep 2026
 * while the Job held nothing. What it holds sits above the log, because the log
 * is the record of a span and this is what is true now: Fleet's last line is a
 * preparation command that failed, and no Drone was ever dispatched after it.
 */
export const NoDroneYet: Story = {
  render: () => (
    <div className="armada-screen">
      <InsideAJob
        heading={{ ...HEADING, actions: JOB_ACTS }}
        run={RUN_NOT_STARTED}
        runElapsed="2m 45s"
        machine={
          <JobResources
            reading={HOLDS_WEDGED}
            age="5s"
            examined={EXAMINED_WEDGED}
            onExamine={nothingPressedYet}
          />
        }
        fleet={<ActivityLog entries={FLEET_PREPARING} openId="f3" />}
        where={WHERE}
        brief={BRIEF}
        step={{
          label: "Reproduction",
          fields: [{ label: "State", value: "not started" }],
          chapters: [],
          phasesAbsent: "This step has not started, so no gate has been asked anything.",
        }}
      />
    </div>
  ),
  // Two claims a rendering makes and cannot hold on its own. The first is that
  // the reading survives the company it keeps: alone the panel is the only
  // thing on screen, and here it competes with a log, a tree, seven paths and a
  // whole panel, so the sentence a person came for is asserted to still be one
  // of them. The second is the arrangement — what the Job holds reads before
  // what Armada has done, and before the run — which is #437's decision and the
  // one thing about this screen that a later region added above it would break
  // silently.
  play: async ({ canvas }) => {
    await expect(canvas.getByText(/is not doing what it should be/)).toBeVisible();
    await expect(canvas.getByText(/Fleet holds no process for this job/)).toBeVisible();

    const holds = canvas.getByText("What this Job holds");
    const armada = canvas.getByText("What Armada has done");
    const run = canvas.getByText("The run");
    await expect(holds.compareDocumentPosition(armada)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
    await expect(armada.compareDocumentPosition(run)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
  },
};

/**
 * **What the wire does not carry, said where it would have gone.** A hole that
 * names its cause is a finding; one that reads "coming soon" is not — and a
 * region that closes up reads as a screen that is finished.
 *
 * **The panel stays and says why it is empty, rather than being taken off the
 * screen.** Every other region here names its own cause — the run, the paths,
 * the brief, the step — and the one that vanished would be the only region a
 * reader could not account for. It is also the region a person came to this
 * screen for, since it is the one that answers *is this working*.
 *
 * **What it must not do is offer to ask.** Every reading on it is Fleet's, so
 * a Fleet that did not answer leaves nothing to ask, and `Look now` over
 * "Looking costs no model call" was a live control pointed at the silence.
 * #462.
 */
export const NothingServesTheStep: Story = {
  render: () => (
    <div className="armada-screen">
      <InsideAJob
        heading={{ ...HEADING, actions: JOB_ACTS }}
        run={[]}
        runAbsent="Fleet did not answer for this Job, so its steps are unknown."
        machine={
          <JobResources
            reading={null}
            examined={null}
            nothingToAsk="no_answer"
            onExamine={nothingPressedYet}
          />
        }
        where={undefined}
        whereAbsent="Nothing serves this Job's paths, and no branch exists yet."
        brief={undefined}
        briefAbsent="Nothing serves this Job's brief or its acceptance criteria."
        step={undefined}
        stepAbsent="No step is open, because the run could not be read."
      />
    </div>
  ),
  play: async ({ canvas }) => {
    await expect(
      canvas.getByText(/Fleet is not answering, so there is nothing to ask/),
    ).toBeVisible();
    await expect(canvas.queryByRole("button", { name: /Look now/ })).toBeNull();
  },
};
