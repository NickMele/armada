// The wave a Job dispatched — the graph, the list, and what it is asking you.
// `#1544`.
//
// **Canvas by default, list available**, the toggle the Workflow tab already
// keeps (#1530, 22 Sep). The list is `ActiveJobsList` and `JobRowStacked`,
// which is the row every other surface draws a Job as.
//
// **No word here is minted.** "Landed" is `Settled`'s own spelling, "Asking
// you" is `escalated`'s registry verb, and blocked and waiting are the two
// halves of `job-statuses.toml`'s `drone_process` — `wave.ts` carries the map.

import {
  ActiveJobsList,
  DockQuestions,
  JobRowStacked,
  JudgeQuestion,
  Tabs,
  WaveCanvas,
} from "@armada/components";
import { JOB_STATUS } from "@armada/components/src/generated/vocabulary";
import type { ReactNode } from "react";

import type {
  CommandAnswer,
  JobDetail as JobWhole,
  JobSummary,
  JudgeAnswer,
  RepositorySummary,
} from "@armada/protocol";

import { answerNamed } from "./copy";
import { dockQuestionsOf } from "./dock-questions";
import type { JobDraft } from "./draft/held";
import type { WaveJobView, WaveView } from "./draft/wave";
import { waveOf } from "./draft/wave";
import type { Outstanding } from "./outstanding";
import { Eyebrow } from "./regions";
import { waveRunOf, waveSaid, waveStandingOf } from "./wave";
import { WORKFLOW_VIEWS, WORKFLOW_VIEW_LABEL, type WorkflowView } from "./workflow-view";

/** `escalated`'s own verb, which is the registry's words for a Job asking you. */
export const ASKING_YOU = JOB_STATUS.escalated?.verb ?? "escalated";

/** Why nothing can be answered from a window that is not live. */
const NOT_LIVE = "This window is not live, so nothing can be sent.";

type JudgeAsking = Extract<Outstanding, { kind: "judge" }>;

export type WaveRegionProps = {
  job: JobSummary;
  whole: JobWhole | null;
  /** What this moment's boards draw that Fleet cannot serve yet. */
  draft?: JobDraft;
  /** The Board's own rows, which is where a wave with no draft is derived from. */
  board: readonly JobSummary[];
  /** Every question waiting on a person, as main gathers them. */
  questions: readonly Outstanding[];
  repositories: readonly RepositorySummary[];
  now: number;
  /** Nothing is live, so every answer is refused. */
  stale: boolean;
  /** An act on one of these Jobs is already out. */
  acting: boolean;
  view: WorkflowView;
  onView: (view: WorkflowView) => void;
  onOpenJob: (jobId: string) => void;
  onAnswerJudge: (jobId: string, askedAt: string, answer: JudgeAnswer, note?: string) => void;
  onAnswerCommand: (
    jobId: string,
    call: string,
    answer: CommandAnswer,
    note?: string,
    rule?: string,
  ) => void;
};

/**
 * The wave to draw: the moment's own where it has one, else what the Board's
 * rows say. Absent is a Job that dispatched nothing.
 */
export function waveReadingOf(
  whole: JobWhole | null,
  draft: JobDraft | undefined,
  board: readonly JobSummary[],
): WaveView | undefined {
  if (draft?.wave !== undefined) return draft.wave;
  return whole === null ? undefined : waveOf(whole, board);
}

/**
 * Which pass of the loop the Job is on, and what the loop is.
 *
 * Read off the wire: `StepDetail.pass` is which pass of how many, and
 * `verdict_routing_target` is the step it returns to. A workflow that does not
 * loop says nothing rather than drawing a pass of one.
 */
export function loopSaid(whole: JobWhole | null): string | undefined {
  if (whole === null) return undefined;
  const closes = whole.steps.find(
    (step) => step.pass !== undefined && step.verdict_routing_target !== undefined,
  );
  if (closes?.pass === undefined) return undefined;
  const back = whole.steps.find((step) => step.step_id === closes.verdict_routing_target);
  const names = whole.steps.map((step) => step.label).join(", then ");
  const again = back === undefined ? "" : `, and back to ${back.label.toLowerCase()}`;
  return `Pass ${String(closes.pass.number)} of ${String(closes.pass.of)} — ${names}${again}.`;
}

/** One Job of the wave, as the list draws it. */
function WaveRow({
  job,
  waits,
  onOpen,
}: {
  job: WaveJobView;
  waits: readonly WaveJobView[];
  onOpen: () => void;
}) {
  const rendering = JOB_STATUS[job.status];
  if (rendering?.badgeStatus == null || rendering.icon == null) return null;
  return (
    <JobRowStacked
      status={rendering.badgeStatus}
      statusIcon={rendering.icon}
      statusLabel={rendering.verb ?? job.status}
      headline={job.title}
      jobId={job.job}
      {...(job.handle === undefined ? {} : { handle: job.handle })}
      fields={[
        {
          label: "Waits on",
          value: waits.length === 0 ? "nothing" : waits.map((one) => one.title).join(", "),
          quiet: waits.length === 0,
        },
        ...(job.landed === undefined
          ? []
          : [
              {
                label: "Pull request",
                value: job.landed === "merged" ? "merged" : "closed without merging",
              },
            ]),
      ]}
      pulsing={job.status === "running"}
      onOpen={onOpen}
    />
  );
}

/**
 * A Job's Judge refusal, answered where it is read.
 *
 * **The existing card, not a new one** (#1530): a refusal here is the same
 * record as a refusal anywhere else, and a second shape for one record is a
 * record that can be renamed in one place.
 */
function JudgeAsk({
  ask,
  title,
  stale,
  acting,
  onAnswerJudge,
}: {
  ask: JudgeAsking;
  title: string;
  stale: boolean;
  acting: boolean;
  onAnswerJudge: WaveRegionProps["onAnswerJudge"];
}) {
  return (
    <div className="armada-wave__ask" aria-label={`${title}, a Judge refusal`}>
      <p className="armada-wave__ask-of">{title}</p>
      <JudgeQuestion
        question={ask.question.question}
        expected={ask.question.expected}
        produced={ask.question.produced}
        consequence={ask.question.consequence}
        disabled={stale || acting}
        {...(stale ? { disabledNote: NOT_LIVE } : {})}
        onAnswer={(answer, note) => onAnswerJudge(ask.job_id, ask.question.asked_at, answer, note)}
      />
    </div>
  );
}

/** One half of Needs you, with its own count. Nothing is drawn where it is empty. */
function NeedsHalf({
  label,
  says,
  count,
  children,
}: {
  label: string;
  says: string;
  count: number;
  children: ReactNode;
}) {
  if (count === 0) return null;
  return (
    <section className="armada-wave__needs-half" aria-label={label}>
      <Eyebrow>
        {label} · {count}
      </Eyebrow>
      <p className="armada-wave__needs-said">{says}</p>
      {children}
    </section>
  );
}

export function WaveRegion({
  job,
  whole,
  draft,
  board,
  questions,
  repositories,
  now,
  stale,
  acting,
  view,
  onView,
  onOpenJob,
  onAnswerJudge,
  onAnswerCommand,
}: WaveRegionProps) {
  const wave = waveReadingOf(whole, draft, board);
  if (wave === undefined) return null;

  const byId = new Map(wave.jobs.map((one) => [one.job, one]));
  const mine = questions.filter((one) => one.kind !== "helm" && byId.has(one.job_id));
  const asking = new Set(mine.map((one) => (one.kind === "helm" ? "" : one.job_id)));
  const standing = waveStandingOf(wave, asking);
  const run = waveRunOf(wave, { onOpen: onOpenJob });
  const waitsOf = (one: WaveJobView) =>
    one.waits_on.flatMap((id) => {
      const held = byId.get(id);
      return held === undefined ? [] : [held];
    });

  /**
   * A command the Manifest has not cleared, as the dock's own card — allow for
   * this Job, add the rule to the repository's Manifest, or reject, which are
   * the three `COMMAND_ANSWER` offers Fleet sent. One place those words are
   * written (#1518, #1519).
   */
  const commandsFor = (jobs: readonly WaveJobView[]) => {
    const held = new Set(jobs.map((one) => one.job));
    return dockQuestionsOf(
      mine.filter((one) => one.kind === "command" && held.has(one.job_id)),
      board,
      repositories,
      now,
      stale
        ? {}
        : {
            onAnswer: (question, answer) => {
              const named = answerNamed(answer);
              if (named !== undefined && question.kind === "command") {
                onAnswerCommand(question.job_id, question.waiting.call, named);
              }
            },
          },
    ).map((card) => (stale ? { ...card, note: NOT_LIVE } : card));
  };

  const judgesFor = (jobs: readonly WaveJobView[]) => {
    const held = new Set(jobs.map((one) => one.job));
    return mine.filter(
      (one): one is JudgeAsking => one.kind === "judge" && held.has(one.job_id),
    );
  };

  const rows = (jobs: readonly WaveJobView[], label: string) => (
    <ActiveJobsList variant="panel" label={label} selectable>
      {jobs.map((one) => (
        <WaveRow key={one.job} job={one} waits={waitsOf(one)} onOpen={() => onOpenJob(one.job)} />
      ))}
    </ActiveJobsList>
  );

  const half = (label: string, says: string, jobs: readonly WaveJobView[]) => (
    <NeedsHalf label={label} says={says} count={jobs.length}>
      {rows(jobs, label)}
      {judgesFor(jobs).map((ask) => (
        <JudgeAsk
          key={ask.job_id}
          ask={ask}
          title={byId.get(ask.job_id)?.title ?? ask.job_id}
          stale={stale}
          acting={acting}
          onAnswerJudge={onAnswerJudge}
        />
      ))}
      <DockQuestions questions={commandsFor(jobs)} />
    </NeedsHalf>
  );

  const loop = loopSaid(whole);
  const needs = standing.blocked.length + standing.waiting.length;

  return (
    <section className="armada-wave" aria-label="The wave">
      <div className="armada-wave__head">
        <Eyebrow>The wave</Eyebrow>
        <Tabs
          items={WORKFLOW_VIEWS.map((one) => ({ id: one, label: WORKFLOW_VIEW_LABEL[one] }))}
          value={view}
          onChange={(id) => onView(id as WorkflowView)}
        />
      </div>
      <p className="armada-wave__said">{waveSaid(standing)}</p>
      {loop === undefined ? null : (
        <p className="armada-wave__loop" role="note">
          {loop}
        </p>
      )}

      {view === "canvas" ? (
        <div className="armada-wave__canvas">
          <WaveCanvas
            nodes={run.nodes}
            edges={run.edges}
            label={`${job.title}, as the Jobs it dispatched and which waits on which`}
            opensOn={run.opensOn}
          />
        </div>
      ) : (
        rows(wave.jobs, "The wave, as a list")
      )}

      {needs === 0 ? null : (
        <section className="armada-wave__needs" aria-label={`The wave ${ASKING_YOU}`}>
          {half(
            "Blocked",
            "Each holds a Drone and the worktree it is in, and nothing moves until you answer.",
            standing.blocked,
          )}
          {half(
            "Waiting",
            "Each is resting at a gate. No Drone is held, and its slot has gone back.",
            standing.waiting,
          )}
        </section>
      )}
    </section>
  );
}
