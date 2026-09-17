// The dock's cards, from what main gathered: each names its repository and Job, and offers the
// answers its kind does, in Fleet's order.

import { describe, expect, it, vi } from "vitest";

import type { JobSummary, RepositorySummary } from "@armada/protocol";
import { dockQuestionsOf, jobNumber } from "./dock-questions";
import type { Outstanding } from "./outstanding";

function job(over: Partial<JobSummary> = {}): JobSummary {
  return {
    id: "a",
    handle: "12-the-drone-count",
    title: "The drone count is wrong",
    status: "running",
    workflow_id: "bug",
    owner_manifest_id: "armada",
    origin: "dispatched",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-09-13T09:00:00Z",
    ...over,
  };
}

const manifest = (id: string, root: string) => ({ id, repository: id, path: `${root}/armada.yml`, records_root: `/r/${id}`, version: 1, checks: [] });
const REPOSITORIES: RepositorySummary[] = [
  { root: "/u/armada", records_root: "/r/armada", manifest: manifest("armada", "/u/armada") },
  { root: "/u/shop", records_root: "/r/shop", manifest: manifest("shop-01", "/u/shop") },
];

const DRONE: Outstanding = {
  kind: "drone",
  job_id: "a",
  asking: {
    question_id: "q1",
    step_id: "implement",
    asked_at: "2026-09-13T10:05:00Z",
    question: "Count exited drones?",
    options: [
      { label: "Yes", consequence: "Counts them." },
      { label: "No", consequence: "Leaves them out." },
    ],
  },
};
const COMMAND: Outstanding = {
  kind: "command",
  job_id: "a",
  waiting: {
    call: "c1",
    step_id: "implement",
    asked_at: "2026-09-13T10:06:00Z",
    tool: "Bash",
    detail: "cargo test",
    truncated: false,
    offers: ["allow_for_job", "reject", "later_fleet_answer" as never],
    rules: [],
  },
};
const JUDGE: Outstanding = {
  kind: "judge",
  job_id: "b",
  question: {
    step_id: "review",
    criterion_id: "cause",
    question: "Does the fix address the cause?",
    expected: "The cause is fixed.",
    produced: "The symptom is hidden.",
    consequence: "The discount is still ignored.",
    asked_at: "2026-09-13T10:00:00Z",
  },
};

const NOW = Date.parse("2026-09-13T10:10:00Z");
const JOBS = [job(), job({ id: "b", handle: "3-checkout-total", title: "Checkout total", owner_manifest_id: "shop-01", status: "awaiting_review" })];

describe("the dock's cards", () => {
  it("name the repository by the picker's label and the Job by number, oldest first", () => {
    const cards = dockQuestionsOf([DRONE, COMMAND, JUDGE], JOBS, REPOSITORIES, NOW);
    expect(cards.map((card) => [card.id, card.repository, card.job])).toEqual([
      ["b:judge", "shop-01", "3"],
      ["a:drone", "armada", "12"],
      ["a:command", "armada", "12"],
    ]);
  });

  it("offer each kind's own answers, in Fleet's order, and drop one this build has no words for", () => {
    const [judge, drone, command] = dockQuestionsOf([DRONE, COMMAND, JUDGE], JOBS, REPOSITORIES, NOW);
    expect(drone?.answers.map((answer) => answer.id)).toEqual(["Yes", "No"]);
    expect(command?.answers.map((answer) => answer.id)).toEqual(["allow_for_job", "reject"]);
    expect(judge?.answers.map((answer) => answer.id)).toEqual(["agree", "disagree_once", "disagree_always"]);
    expect(judge?.detail).toBe("The discount is still ignored.");
  });

  it("leave out a question on a Job the Board does not hold", () => {
    expect(dockQuestionsOf([JUDGE], [job()], REPOSITORIES, NOW)).toEqual([]);
  });

  it("say where to answer until answering is handed in, and hand each act its own question", () => {
    const [unwired] = dockQuestionsOf([DRONE], JOBS, REPOSITORIES, NOW);
    expect(unwired?.onAnswer).toBeUndefined();
    expect(unwired?.note).toBe("Open job 12 to answer.");

    const onAnswer = vi.fn();
    const onDiscuss = vi.fn();
    const [wired] = dockQuestionsOf([DRONE], JOBS, REPOSITORIES, NOW, { onAnswer, onDiscuss });
    wired?.onAnswer?.("No");
    wired?.onDiscuss?.();
    expect(onAnswer).toHaveBeenCalledWith(DRONE, "No");
    expect(onDiscuss).toHaveBeenCalledWith(DRONE, JOBS[0]);
    expect(wired?.note).toBeUndefined();
  });

  it("read a number off the handle, and the whole handle where it carries none", () => {
    expect(jobNumber(job({ handle: "140-a-job" }))).toBe("140");
    expect(jobNumber(job({ handle: "unnumbered" }))).toBe("unnumbered");
  });
});

// #1389: a helm call has no job, so a card drawn from one names its repository and nothing else.
const HELM: Outstanding = {
  kind: "helm",
  call: {
    call: "helm-1",
    manifest_id: "shop-01",
    asked_at: "2026-09-13T10:02:00Z",
    tool: "Bash",
    detail: "gh issue list --milestone Helm",
    truncated: false,
    length: 30,
    rule: "Bash(gh issue list:*)",
    offers: ["allow_once", "allow_and_remember", "refuse"],
    holding_for_seconds: 300,
  },
};

describe("a helm call in the dock", () => {
  it("names its repository where a job's card names a job", () => {
    const [card] = dockQuestionsOf([HELM], [], REPOSITORIES, Date.parse("2026-09-13T10:03:00Z"));

    expect(card?.repository).toBe("shop-01");
    expect(card?.job).toBeUndefined();
    expect(card?.label).toBe("Helm needs your permission");
    expect(card?.detail).toBe("Your settings would have to allow Bash(gh issue list:*).");
    expect(card?.answers.map((answer) => answer.id)).toEqual([
      "allow_once",
      "allow_and_remember",
      "refuse",
    ]);
  });

  // The board is where a job's question is dropped from when its job is not on it. A helm call is
  // on no board, so nothing there can take it down — only its own answer does.
  it("is drawn even where the board holds nothing", () => {
    expect(dockQuestionsOf([HELM, DRONE], [], REPOSITORIES, Date.now())).toHaveLength(1);
  });

  it("sorts in among a job's questions by when each was asked", () => {
    const cards = dockQuestionsOf([DRONE, HELM], [job()], REPOSITORIES, Date.now());

    expect(cards.map((card) => card.id)).toEqual(["helm:helm-1", "a:drone"]);
  });

  it("sends its answer by the wire's own word", () => {
    const onAnswer = vi.fn();
    const [card] = dockQuestionsOf([HELM], [], REPOSITORIES, Date.now(), { onAnswer });

    card?.onAnswer?.("allow_and_remember");

    expect(onAnswer).toHaveBeenCalledWith(HELM, "allow_and_remember");
  });
});
