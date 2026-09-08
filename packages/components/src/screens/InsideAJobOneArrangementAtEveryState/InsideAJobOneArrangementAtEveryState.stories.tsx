import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, within } from "storybook/test";
import { Button } from "../../primitives/Button/Button";
import { ActivityLog } from "../../compositions/ActivityLog/ActivityLog";
import { JobHoldsSheet } from "../../compositions/JobHoldsSheet/JobHoldsSheet";
import { JobHoldsSummary } from "../../compositions/JobHoldsSummary/JobHoldsSummary";
import { Refusals } from "../../compositions/Refusals/Refusals";
import { InsideAJob } from "./InsideAJobOneArrangementAtEveryState";
import {
  BRIEF,
  CHAPTERS,
  ESCALATED_HEADING,
  escalatedHeading,
  EXAMINED_WEDGED,
  EXAMINED_WORKING,
  FAILED_HEADING,
  HEADING,
  HOLDS_RUNNING,
  HOLDS_WEDGED,
  OnAScreen,
  REPAIR_CHAPTERS,
  RUN_FAILED,
  RUN_NOT_STARTED,
  RUN_REPAIRING,
  RUN_RUNNING,
  RUN_STOPPED,
  RUN_WAITING,
  SUMMARY_AFTER_THE_END,
  SUMMARY_AT_THE_GATE,
  SUMMARY_IDLE,
  SUMMARY_RUNNING,
  SUMMARY_WEDGED,
  TAIL_LIVE,
  TAIL_PREPARING,
  WAITING_HEADING,
  WHERE,
} from "./fixtures";
import {
  CONSOLE_SHEET,
  EVIDENCE_CHAPTERS,
  INPUTS_SHEET,
  QUEUED_EVIDENCE_CHAPTERS,
  aRefusedJob,
  REFUSED_DECISION,
  RUNNING_PHASES,
  TWO_REFUSAL_CHAPTERS,
} from "./evidence";
import { PlayableJob } from "./playable";

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
  decorators: [(Story) => <OnAScreen><Story /></OnAScreen>],
};
export default meta;

type Story = StoryObj<typeof InsideAJob>;

/**
 * What the Job holds on this machine, below the run and above the pointers.
 *
 * **Five lines, and the reading a press away.** It used to be a card at the top
 * of this column, above the run a person opens a Job to read — the largest
 * thing on the column, answering a question nobody had asked yet. What survives
 * up here is what changes the answer: the tail of Armada's own log, the process
 * count, the worktree and the disk.
 *
 * **Six of the seven states carry one, and the seventh is the point.** The
 * region draws nothing when it is absent, so a story without one proves that —
 * and `A check failed` is the state where nothing about the machine is in
 * question, because a Drone is working and the story below says which line it
 * is on.
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
          <JobHoldsSummary
            tail={TAIL_LIVE}
            figures={SUMMARY_RUNNING}
            age="3s"
            onOpen={nothingPressedYet}
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
          <JobHoldsSummary
            tail={TAIL_LIVE}
            figures={SUMMARY_AT_THE_GATE}
            age="8s"
            onOpen={nothingPressedYet}
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
          <JobHoldsSummary
            tail={TAIL_LIVE}
            figures={SUMMARY_IDLE}
            age="4s"
            onOpen={nothingPressedYet}
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
 * **Blocked by policy — and now the screen says what by.** The report this
 * story exists for: *"It says it's blocked by policy but there are no details
 * anywhere on what the hell blocked it. I can't review anything. It's telling
 * me I need to unblock but — what am I unblocking it from?"*
 *
 * The badge, the trigger and the acts were all here already. What was not was
 * the command each refusal was on: the tool is on one transcript row and the
 * command is on the next, joined by a call id that nothing joined, so the only
 * way to answer the question was to open the transcript and do it by hand.
 *
 * **Four things to read against the drawing.** The commands share one left
 * edge however wide the tool names are; the same command refused three times is
 * three rows, because a Drone that did not learn is the most diagnostic thing
 * here; the heredoc says how much of it is on the row, so nobody pastes a cut
 * command into an allowlist as a whole one; and the note under the list says
 * the list is short, so nobody widens an allowlist for the eight they can see
 * and thinks they are done.
 *
 * **The second report, and the sentence under the rows is the answer to it.**
 * The list arrived and the screen still offered a restart beside it with
 * nothing saying whether restarting met the same wall: *"I feel like if I
 * restart, the drone is going to do the same thing again. Nothing on the screen
 * is giving me confidence that I can unblock the drone and not run into the
 * same issue."* It would have. A Drone's toolset is rendered when it spawns and
 * a restart renders the same one, so the rows read as a diagnosis and the
 * button read as its cure.
 *
 * **Read the band top to bottom and it is three things in order**: what was
 * blocked, that a restart meets it again and what would change that, then the
 * act. Neither sentence tells a person to do anything — they say how the
 * toolset is built, which is a fact this screen was withholding.
 */
export const BlockedByPolicy: Story = {
  render: () => (
    <div className="armada-screen">
      <InsideAJob
        heading={{ ...escalatedHeading("blocked_by_policy"), actions: JOB_ACTS }}
        run={RUN_STOPPED}
        runElapsed="9m 12s"
        machine={
          <JobHoldsSummary
            tail={TAIL_LIVE}
            figures={SUMMARY_IDLE}
            age="6s"
            onOpen={nothingPressedYet}
          />
        }
        where={WHERE}
        brief={BRIEF}
        step={{
          label: "Regression check",
          fields: [
            { label: "Held for", value: "3m 02s", mono: true },
            { label: "Attempt", value: "1", mono: true },
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
            title: "blocked by policy · stopped at Regression check",
            children: (
              <>
                <Refusals
                  said="The job was blocked from the following:"
                  note="showing 8 of 31 refused calls"
                  again={
                    "A drone's toolset is fixed when it starts, and a restart builds the same " +
                    "one from the same declaration. A drone that reaches for these again is " +
                    "refused again. A command is in that toolset only where the repository's " +
                    "armada.yml declares it under commands and does not mark it destructive."
                  }
                  refused={[
                    { tool: "Bash", detail: "cargo nextest run --package ipc 2>&1 | tail -80" },
                    { tool: "Bash", detail: "cargo nextest run --package ipc" },
                    { tool: "Bash", detail: "cargo nextest run --package ipc 2>&1 | tail -80" },
                    {
                      tool: "Bash",
                      detail: "mkdir -p xtask/src/rules_tests",
                    },
                    { tool: "Write", detail: "xtask/src/rules_tests/refused.rs" },
                    {
                      tool: "WebFetch",
                      detail: "https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html",
                    },
                    {
                      tool: "Bash",
                      detail:
                        "cat <<'EOF' > xtask/src/rules_tests/refused.rs " +
                        "use std::path::Path; use crate::Report; pub fn every_refusal_names_its_command" +
                        "(root: &Path) -> Report { let mut report = Repo",
                      size: "showing 200 of 14,320 characters",
                    },
                    { tool: "Bash", detail: "" },
                  ]}
                />
                <div>
                  The drone is holding at this step. Nothing advances until you decide what happens
                  next.
                </div>
                <div>
                  Restart is not offered while the drone is alive: a restart throws that session
                  away.
                </div>
              </>
            ),
          },
          phases: {
            note: "Nothing was submitted, so no gate has been asked anything. The drone stopped reaching for what it needed.",
            stages: [
              { id: "instructed", label: "Instructed", state: "cleared" },
              { id: "working", label: "Working", state: "current" },
              { id: "submitted", label: "Submitted", state: "ahead" },
              { id: "checks", label: "Checks", kind: "checks", state: "ahead", stands: "not reached" },
              { id: "judge", label: "Judge · 2 criteria", kind: "judge", state: "ahead", stands: "not reached" },
              { id: "you", label: "You", kind: "human", state: "waiting", stands: "waiting · 3m 02s" },
            ],
          },
          chapters: CHAPTERS,
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
          <JobHoldsSummary
            tail={TAIL_LIVE}
            figures={SUMMARY_AFTER_THE_END}
            age="1m"
            onOpen={nothingPressedYet}
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
 * and correct to be. What Armada has done is the tail of this block — #437 put
 * it on this column and it is still here, folded into the reading rather than
 * given a region of its own. **Not attached to the step about to start**:
 * hanging these off step one reads as though it were running when it has not
 * begun, which is the confusion that made a wedged Job look healthy.
 *
 * **And it is the state where nothing else on the screen says the Job is
 * dead.** The badge reads `running`, the tree reads `not started`, and both are
 * true — which is exactly what a person read for six minutes on 4 Sep 2026
 * while the Job held nothing. The tail and the figures answer it together:
 * Armada's last line is a preparation command that failed, no process is held,
 * and no Drone was ever dispatched after it.
 */
export const NoDroneYet: Story = {
  render: () => (
    <div className="armada-screen">
      <InsideAJob
        heading={{ ...HEADING, actions: JOB_ACTS }}
        run={RUN_NOT_STARTED}
        runElapsed="2m 45s"
        machine={
          <JobHoldsSummary
            tail={TAIL_PREPARING}
            figures={SUMMARY_WEDGED}
            age="5s"
            onOpen={nothingPressedYet}
          />
        }
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
  // Armada's last line survives the company it keeps: it had a region of its
  // own and is now two rows inside a reading, competing with a tree, seven
  // paths and a whole panel — so the line a person came for is asserted to
  // still be one of them.
  //
  // The second is the arrangement, and it is the one this change decided: the
  // run reads first, the holdings after it, the pointers after those. Nothing
  // about a rendering says which region a later one was inserted above, which
  // is how this block came to sit at the top of the column in the first place.
  play: async ({ canvas }) => {
    await expect(canvas.getByText("A preparation command failed")).toBeVisible();
    await expect(canvas.getByText("Processes")).toBeVisible();

    const run = canvas.getByText("The run");
    const holds = canvas.getByText("What this Job holds");
    const where = canvas.getByText("Where things are");
    await expect(run.compareDocumentPosition(holds)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
    await expect(holds.compareDocumentPosition(where)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
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
          <JobHoldsSummary
            tail={[]}
            tailNote="Armada is not reading this Job's log."
            figures={null}
            note="Fleet did not answer, so what this Job holds is unknown."
            onOpen={nothingPressedYet}
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
  /**
   * **Nothing on this column offers to ask.** `Look now` went to the sheet with
   * the reading it acts on, and the summary says why there is no reading rather
   * than drawing an empty figure — a blank `Processes` row here would report a
   * silent seam as a Job holding nothing, which is #462 in a smaller region.
   */
  play: async ({ canvas }) => {
    await expect(
      canvas.getByText(/Fleet did not answer, so what this Job holds is unknown/),
    ).toBeVisible();
    await expect(canvas.queryByText("Processes")).toBeNull();
    await expect(canvas.queryByRole("button", { name: /Look now/ })).toBeNull();
  },
};

/**
 * **The evidence surface — chapters four and five.** A Check result is an
 * evidence record: it has a kind and a file, exactly like a Drone's evidence,
 * and the only difference is who produced it. So the Checks are a chapter in
 * the step's story rather than a status footer, and the panel that read them is
 * the chapter after.
 *
 * **The whole state is the argument.** Every Check exited 0, and the Job is
 * refused — because the suite went green by deleting the assertion that was
 * failing. `exit 0 · 315 passed` cannot express that, and nothing else on this
 * screen could either until the output became reachable.
 *
 * **The Verdicts chapter opens in place and the output does not.** A refusal is
 * prose and fits the panel; 2,180 lines is not a longer preview, so the Checks
 * chapter carries an act that leaves. That is the split `StepChapter` already
 * names, applied to two new chapters rather than invented for them.
 *
 * Nothing on the wire serves any of this. `evidence.tsx` says what would have
 * to — a `kind` and a `log_path` on a check result, and citations recorded by
 * the judges that met a criterion as well as the ones that refused.
 */
export const TheEvidenceSurface: Story = {
  render: () =>
    aRefusedJob({
      fields: [
        { label: "Took", value: "8m 41s", mono: true },
        { label: "Attempt", value: "3 of 3", mono: true },
        { label: "Refused at", value: "15:02:44", mono: true },
      ],
      acts: <Button variant="secondary">Re-run the gate</Button>,
      notice: {
        tone: "stopped",
        title: "All mechanical checks passed, but the panel of judges refused the work.",
        children:
          "The suite is green because one of its cases stopped existing. 2 of the 3 judges refused criterion 02, and the assertion they both cite is in the check output below.",
      },
      // One set of acts for the whole step. They sat inside the refusal block
      // until 2026-09-08, which on the Job below would have offered to kill
      // the same Job twice.
      after: REFUSED_DECISION,
    }),
};

/**
 * **Two criteria refused on one panel.** This is the state that moved the
 * refusal into the row: two blocks below the grid meant two `Refused — 0N`
 * headings a reader had to map back to rows they had scrolled past, and the
 * note about criteria being frozen — a second description of criterion 02 —
 * sat four inches away from criterion 02.
 *
 * **One is open and the other says how large it is.** `2 of 3 refused` and
 * `1 of 3 refused` are the same verdict and different situations, so the shape
 * of the whole panel stays readable without opening anything.
 */
export const TwoCriteriaRefused: Story = {
  render: () =>
    aRefusedJob({
      fields: [
        { label: "Took", value: "8m 41s", mono: true },
        { label: "Attempt", value: "3 of 3", mono: true },
        { label: "Refused at", value: "15:02:44", mono: true },
      ],
      acts: <Button variant="secondary">Re-run the gate</Button>,
      notice: {
        tone: "stopped",
        title: "All mechanical checks passed, and the panel refused two criteria.",
        children:
          "One is a deleted assertion that made the suite green; the other is an entry point only one judge went looking for. The two lead to different acts, which is why the panel names both rather than reporting a count.",
      },
      chapters: [...CHAPTERS, ...TWO_REFUSAL_CHAPTERS],
      after: REFUSED_DECISION,
    }),
};

/**
 * **A Check's output, open in the viewer.** One layer, one artifact, and the
 * panel underneath exactly as it was — the log and the diff already work this
 * way, and a check's output is the third reading with no end.
 *
 * **The strip is a band on the sheet.** Every other artifact this step produced
 * stays one press away, so changing what you are looking at never means closing
 * the layer; and *Back to the default view* is what returns the page to the
 * artifact it was composed with.
 *
 * **Line 2,322 is the design.** A transcript ending `315 passed` is a fact with
 * no reference point. The same transcript with the parent commit's count under
 * it is an argument — and the check emitted both lines, so Bridge renders what
 * is already in the file rather than diffing two stored logs.
 */
export const AChecksOutputOpen: Story = {
  render: () => aRefusedJob({ sheet: CONSOLE_SHEET }),
};

/**
 * **The judgment's inputs — the view a log file could never give you.** A panel
 * is only a panel if the judges ran independently on identical inputs, and the
 * digest is the evidence for that guarantee. It is the first thing to doubt
 * when a unanimous verdict looks too easy.
 *
 * **The criteria stay on the screen behind the layer.** Nothing the viewer ever
 * shows covers the yardstick the work is being read against — which is the rule
 * that decided the sheet's width rather than a full-screen route.
 */
export const TheJudgmentsInputs: Story = {
  render: () => aRefusedJob({ sheet: INPUTS_SHEET }),
};

/**
 * **The same refused Job, with every selector wired.** A check row, a chip, a
 * citation, a run-tree fact and a phase-card row resolve their artifact id
 * against one registry, in `playable.tsx`; the log keeps its own sheet and
 * shares the slot, being a stream and no artifact. **Only this story holds
 * state** — the rest are the record of a moment.
 */
export const Playable: Story = {
  render: () => <PlayableJob />,
  /**
   * A selector opens the artifact it names rather than the composed one, the
   * way back returns to that one, and the log replaces the viewer rather than
   * stacking. A resolver answering every id alike fails on assertion two.
   */
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.queryByRole("dialog")).toBeNull();
    await userEvent.click(canvas.getByRole("button", { name: "output · 96 lines" }));
    await expect(await canvas.findByText("check:bench — output")).toBeVisible();

    await userEvent.click(canvas.getByRole("tab", { name: "Measurement" }));
    await expect(canvas.getByText("check:bench — measured")).toBeVisible();

    // The way back is the composed artifact's own chip, in the strip.
    const strip = within(canvas.getByRole("dialog"));
    await userEvent.click(strip.getByRole("button", { name: /check:test_suite/ }));
    await expect(canvas.getByText("check:test_suite — output")).toBeVisible();
    // One slot: the log takes the layer the viewer was on rather than stacking.
    await userEvent.click(canvas.getByRole("button", { name: /Open the log/ }));
    await expect(canvas.getAllByRole("dialog")).toHaveLength(1);
    await expect(canvas.getByRole("dialog", { name: "Activity log" })).toBeVisible();

    await userEvent.keyboard("{Escape}");
    await expect(canvas.queryByRole("dialog")).toBeNull();
  },
};

/**
 * **A running Job, and the same two chapters.** The Checks chapter draws the
 * one Check in flight and the two behind it as queued rows — the shape of what
 * is coming is part of reading a running Job, and a list that grew as Checks
 * started would make a Job look like it had fewer gates than it has.
 *
 * **The Verdicts chapter says `not reached` in words.** An empty region under a
 * running step reads as a region that failed to load.
 */
export const EvidenceOnARunningJob: Story = {
  render: () => (
    <div className="armada-screen">
      <InsideAJob
        heading={{ ...HEADING, actions: JOB_ACTS }}
        run={RUN_RUNNING}
        runElapsed="4m 12s"
        onOpenArtifact={nothingPressedYet}
        where={WHERE}
        brief={BRIEF}
        step={{
          label: "Verify",
          fields: [
            { label: "Running for", value: "4m 12s", mono: true },
            { label: "Attempt", value: "1", mono: true },
          ],
          phases: RUNNING_PHASES,
          chapters: [...CHAPTERS, ...QUEUED_EVIDENCE_CHAPTERS],
        }}
      />
    </div>
  ),
};

/**
 * **The full reading, open.** The summary under the run says whether anything
 * is wrong; this is what it opens when the answer is yes — `JobResources`
 * whole, with the act that goes and looks, on the layer the log and the patch
 * already use.
 *
 * **The panel underneath is exactly as it was.** That is the point of the
 * layer, and the reason the reading left the column rather than shrinking
 * inside it: the run stays on screen, and closing puts the reader back where
 * they were reading.
 */
export const TheFullReadingOpen: Story = {
  render: () => <WithTheReading />,
  /**
   * The two things a rendering cannot show: the control opens the sheet, and
   * `Esc` returns to the panel rather than to another layer.
   *
   * **A drawn-open sheet proves neither.** The wiring is what breaks — a
   * control that stopped opening, or a close that came back to a second sheet —
   * and both render identically to a working screen at the instant they fail.
   */
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.queryByRole("dialog")).toBeNull();

    await userEvent.click(canvas.getByRole("button", { name: "Open the full reading" }));
    const sheet = await canvas.findByRole("dialog", { name: "What this Job holds" });
    await expect(sheet).toBeVisible();
    await expect(canvas.getByRole("button", { name: /Look now/ })).toBeVisible();

    await userEvent.keyboard("{Escape}");
    await expect(canvas.queryByRole("dialog")).toBeNull();
    await expect(canvas.getByText("Where things are")).toBeVisible();
  },
};

/**
 * The pair, wired the way `JobDetail` wires it: the summary holds the values,
 * the sheet holds the reading, and one piece of state says whether it is open.
 */
function WithTheReading() {
  const [open, setOpen] = useState(false);
  return (
    <div className="armada-screen">
      <InsideAJob
        heading={{ ...HEADING, actions: JOB_ACTS }}
        run={RUN_RUNNING}
        runElapsed="11m 03s"
        // The pulse goes with the reading: with a sheet open the tree's current
        // step is behind the layer.
        pulsing={!open}
        machine={
          <JobHoldsSummary
            tail={TAIL_LIVE}
            figures={SUMMARY_RUNNING}
            age="3s"
            onOpen={() => setOpen(true)}
          />
        }
        where={WHERE}
        brief={BRIEF}
        step={{
          label: "Fix",
          fields: [{ label: "Running for", value: "6m 11s", mono: true }],
          chapters: CHAPTERS,
        }}
        sheet={
          <JobHoldsSheet
            open={open}
            jobId="job_2d90bb"
            reading={HOLDS_RUNNING}
            age="3s"
            examined={EXAMINED_WORKING}
            onExamine={nothingPressedYet}
            onClose={() => setOpen(false)}
          />
        }
      />
    </div>
  );
}
