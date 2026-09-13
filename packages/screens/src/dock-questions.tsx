// Every question waiting on a person, as the dock's cards (#935). Main gathers them from every
// repository Fleet serves; this decides what each card says.

import type { DockAnswer, DockQuestion } from "@armada/components";
import type { JobSummary, JudgeAnswer, Outcome, RepositorySummary } from "@armada/protocol";
import { manifestLabel } from "@armada/shell/src/repository-label";
import { offeredOf, said } from "./copy";
import { span } from "./duration";
import { outstandingId, type Outstanding } from "./outstanding";

/** What a person may do from a card. Absent draws the control off. */
export type DockActs = {
  /** An answer by its wire name — a Drone's label, a `CommandAnswer`, a `JudgeAnswer`. #936. */
  onAnswer?: (question: Outstanding, answer: string) => void;
  /** True while this card's own answer is in flight. Draws its answers off without the "open the job" note. */
  answering?: (question: Outstanding) => boolean;
  /** Why this card's last answer did not take, in Fleet's own words. Cleared by a fresh attempt. */
  refusalFor?: (question: Outstanding) => string | undefined;
  /** Point Helm at the card's repository and Job. #944. */
  onDiscuss?: (question: Outstanding, job: JobSummary) => void;
};

/** A card's own refusal, in words — Fleet's for a real refusal, Bridge's own sentence otherwise. */
export function refusalWords(outcome: Outcome): string {
  return outcome.ok ? "" : outcome.why === "refused" ? outcome.error.message : said(outcome);
}

/**
 * The Judge's three answers. **The words `JudgeQuestion` draws on job detail**, retyped because
 * that composition holds them as literals; #938 writes the dock down once and is where they meet.
 */
export const JUDGE_ANSWER: Record<JudgeAnswer, { label: string; means: string }> = {
  agree: {
    label: "Agree with the refusal",
    means: "The step fails, as it would where the criterion is marked refuse.",
  },
  disagree_once: {
    label: "Disagree, just this step",
    means: "The step advances. The next job's gate asks about this criterion again.",
  },
  disagree_always: {
    label: "Always disagree",
    means: "The step advances, and no later job in this repository is asked about this criterion.",
  },
};

/** When it was asked, whichever kind it is. */
export function askedAt(question: Outstanding): string {
  switch (question.kind) {
    case "drone":
      return question.asking.asked_at;
    case "command":
      return question.waiting.asked_at;
    case "judge":
      return question.question.asked_at;
  }
}

/** A Job's number, off the handle Fleet derives from it — `12` of `12-the-drone-count`. */
export function jobNumber(job: JobSummary): string {
  return /^(\d+)-/.exec(job.handle)?.[1] ?? job.handle;
}

/**
 * The cards, oldest first. **A question on a Job the Board does not hold is left out**: it has no
 * number or repository to name, and main re-reads the Board when a move names a Job it lacks.
 */
export function dockQuestionsOf(
  questions: readonly Outstanding[],
  jobs: readonly JobSummary[],
  repositories: readonly RepositorySummary[],
  now: number,
  acts: DockActs = {},
): DockQuestion[] {
  const byId = new Map(jobs.map((job) => [job.id, job]));
  return [...questions]
    .sort((a, b) => Date.parse(askedAt(a)) - Date.parse(askedAt(b)))
    .flatMap((question) => {
      const job = byId.get(question.job_id);
      if (job === undefined) return [];
      const number = jobNumber(job);
      const { onAnswer, answering, refusalFor, onDiscuss } = acts;
      return [
        {
          id: outstandingId(question),
          repository: manifestLabel(job.owner_manifest_id, repositories),
          job: number,
          title: job.title,
          waiting: span(askedAt(question), now) ?? undefined,
          ...askedOf(question),
          ...(onAnswer === undefined
            ? { note: `Open job ${number} to answer.` }
            : { onAnswer: (answer: string) => onAnswer(question, answer), answering: answering?.(question) }),
          refusal: refusalFor?.(question),
          ...(onDiscuss === undefined ? {} : { onDiscuss: () => onDiscuss(question, job) }),
        },
      ];
    });
}

/** What the card says was asked, and the answers its kind offers, in Fleet's order. */
function askedOf(question: Outstanding): Pick<DockQuestion, "label" | "asked" | "detail" | "answers"> {
  switch (question.kind) {
    case "drone":
      return {
        label: "The drone asked a question",
        asked: question.asking.question,
        answers: question.asking.options.map(({ label, consequence }) => ({ id: label, label, consequence })),
      };
    case "command": {
      const { tool, detail, offers } = question.waiting;
      return {
        label: "The drone wants to run a command it was not given",
        asked: detail === "" ? `The drone wants to use ${tool}.` : <span className="mono">{detail}</span>,
        // An answer from a Fleet ahead of this build is left out, `offeredOf`'s rule.
        answers: offeredOf(offers).map(({ offer, label, means }) => ({ id: offer, label, consequence: means })),
      };
    }
    case "judge":
      return {
        label: "Judge refused a criterion and is asking you",
        asked: question.question.question,
        detail: question.question.consequence,
        answers: (Object.keys(JUDGE_ANSWER) as JudgeAnswer[]).map(
          (answer): DockAnswer => ({ id: answer, label: JUDGE_ANSWER[answer].label, consequence: JUDGE_ANSWER[answer].means }),
        ),
      };
  }
}
