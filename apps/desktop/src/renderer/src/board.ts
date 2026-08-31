// The Board's filter set, its order and its count sentence — the arithmetic
// behind the controls, with no React in it.
//
// # Two axes, and the Manifest is neither
//
// The Board is already scoped to one Manifest, so scope is not a control. Origin
// was drawn as a filter and rejected on the drawing: *what needs me*, *what is
// running* and *why has that not started* are all state, and an origin axis
// would narrow by a fact nobody narrows on. What is left is state, and one text
// match. `docs/concepts/job-board.md` carries both decisions.
//
// # The five tabs partition the twelve statuses, and the registry does the
// partitioning
//
// No list of statuses is written here. `job-statuses.toml` says whether a Job
// is over and what it is doing, the codegen carries both onto `JOB_LIFECYCLE`,
// and the four state tabs are read off those two fields plus the one status the
// `Queued` tab is named after:
//
// | Tab | What it is |
// |---|---|
// | Needs you | Not over, not working, and not queued — the statuses that stop until a person reads them |
// | Running | `mode` is `Working`, which is `running` and `piloted` — a job somebody has taken over is still moving |
// | Queued | The `queued` status itself. The tab is named after it, so naming it is the definition rather than a second copy of one |
// | Finished | `terminal` |
//
// **`Needs you` is defined by subtraction and would rather not be.**
// `job-statuses.toml` carries `who_is_acting = "Person"` on exactly the four
// statuses that belong there, and it is the field this tab is actually about —
// but the codegen emits `terminal` and `mode` only, and the generator sits
// outside this change's write scope. Carrying `who_is_acting` onto
// `JOB_LIFECYCLE` would make the tab a lookup rather than a subtraction, and
// would let `piloted` be told apart from `running` by the registry instead of by
// this comment. Reported, not done here.
//
// # Search reads every job whatever tab is set
//
// A text match is not a state, so it does not narrow by one — and the way that
// is honoured is that **starting a search puts the tab back to `All`**. Typing
// is asking a question across every state, so the state control returns to the
// state that means "every", and the sentence holds literally: whatever tab was
// set, the search reads every job.
//
// It happens on the transition into a search and never again, which is the
// difference between a control that gets out of the way and one that fights
// you: refining a query does not snap the tab back, so searching, pressing `3`
// and typing another letter narrows the way a person meant it to.
//
// Every tab's count is a count of what the search matched, so the strip says
// where the matches are as well as how many there are — and a tab that empties
// under a search renders no count rather than a `0`.

import { JOB_LIFECYCLE } from "../../shared/generated/vocabulary";
import type { JobSummary } from "../../shared/protocol";
import type { WorkflowSummary } from "../../shared/setup";
import { instant } from "./duration";

/** The five tabs, in the order they are drawn and keyed. */
export type BoardTab = "all" | "needs-you" | "running" | "queued" | "finished";

/** The four a job can be in. `all` is not a state and holds every job. */
export type StateTab = Exclude<BoardTab, "all">;

/**
 * The tabs, their labels, the key that selects each — `1` through `5`, in the
 * order they are drawn — and what each says when it holds nothing.
 *
 * The key is the position, which is why it is here and not typed into a
 * component: a tab moving in this list moves its key with it, and a strip whose
 * second tab answered `3` would be unlearnable.
 *
 * **Each empty line is written, not templated.** "Nothing is under running" is
 * what a template produces and nobody says; the four sentences below are what a
 * person would say, and there are only four of them.
 */
export const BOARD_TABS: readonly {
  id: BoardTab;
  label: string;
  shortcut: string;
  empty: string;
}[] = [
  { id: "all", label: "All", shortcut: "1", empty: "No jobs." },
  { id: "needs-you", label: "Needs you", shortcut: "2", empty: "Nothing needs you." },
  { id: "running", label: "Running", shortcut: "3", empty: "Nothing is running." },
  { id: "queued", label: "Queued", shortcut: "4", empty: "Nothing is queued." },
  { id: "finished", label: "Finished", shortcut: "5", empty: "Nothing has finished." },
];

/** Where the Board opens. Everything, because nothing has been asked yet. */
export const FIRST_TAB: BoardTab = "all";

/**
 * Which state tab a job is in, or `null` where this build's registry has no
 * lifecycle row for its status.
 *
 * **`null` is a real answer and it is not a fifth tab.** The codegen refuses a
 * status that is in one registry and not the other, so a job reaching here with
 * no lifecycle row also has no verb and no glyph, and the list already draws it
 * as a named line beneath the rows rather than as a row. It counts under `All`
 * and under no state tab, which makes the counts stop summing — visibly, which
 * is the point.
 */
export function tabOf(job: JobSummary): StateTab | null {
  const life = JOB_LIFECYCLE[job.status];
  if (life === undefined) return null;
  if (life.terminal) return "finished";
  if (life.mode === "Working") return "running";
  // The tab is named after the status, so the status is what it holds. Every
  // other non-terminal status that is not working is one that stops until a
  // person reads it.
  return job.status === "queued" ? "queued" : "needs-you";
}

/** Whether a job is one of the ones waiting on a person. */
export function needsYou(job: JobSummary): boolean {
  return tabOf(job) === "needs-you";
}

/** Whether a tab admits a job. `all` admits every one, including the unplaceable. */
export function inTab(job: JobSummary, tab: BoardTab): boolean {
  return tab === "all" || tabOf(job) === tab;
}

/**
 * Whether a job answers a text match.
 *
 * **What a person types is what they can see, plus the ids they quote.** The
 * title, the job id, the branch and the step it is on are all on the row; the
 * workflow's name is on the row as the workflow field, and its id is what a
 * person pastes out of a log. Nothing here searches a field the row does not
 * carry — a match a person cannot see the reason for reads as a bug.
 */
export function matches(
  job: JobSummary,
  query: string,
  workflows: readonly WorkflowSummary[],
): boolean {
  const needle = query.trim().toLowerCase();
  if (needle === "") return true;
  const workflow = workflows.find((held) => held.id === job.workflow_id);
  return [job.title, job.id, job.branch, job.current_step_id, job.workflow_id, workflow?.name]
    .filter((field): field is string => field !== undefined)
    .some((field) => field.toLowerCase().includes(needle));
}

/** The orders the control offers. `Critical first` is where the Board opens. */
export type BoardSort = "critical_first" | "oldest_first";

export const BOARD_SORTS: readonly { id: BoardSort; label: string }[] = [
  { id: "critical_first", label: "Critical first" },
  { id: "oldest_first", label: "Oldest first" },
];

/**
 * The Board's default order, recorded as `job_board.default_sort`.
 *
 * **Critical first: the needs-you cluster, then oldest inside every group.**
 * Settled 2026-08-31 against `[job-board-sort-order]`, which had carried no body
 * since it was filed — see `docs/concepts/job-board.md`.
 *
 * The setting exists in `crates/config/settings.toml` as
 * `job-board-default-sort-filter`, scoped Kit → Manifest. Nothing delivers
 * configuration to Bridge, so this constant is the resolved value and the row
 * over there is the specification it satisfies. When config reaches the
 * renderer this is what it replaces.
 */
export const DEFAULT_SORT: BoardSort = "critical_first";

/**
 * The jobs, in the order asked for.
 *
 * **Oldest first inside every group, in both orders.** The thing waiting
 * longest is the thing most likely to have gone wrong, and that is true of a
 * queue whether or not it is the queue that needs a person. `Critical first`
 * differs from `Oldest first` only in lifting one group above the rest, which
 * is what lets the tab and the sort agree rather than cut across each other:
 * switching to `Needs you` reorders nothing a person had already learned.
 *
 * A job whose `created_at` will not parse sorts last within its group, never
 * first — a corrupt date must not shove real work off the bounded window.
 */
export function sorted(jobs: readonly JobSummary[], sort: BoardSort): JobSummary[] {
  return [...jobs].sort((a, b) => {
    if (sort === "critical_first") {
      const lifted = Number(needsYou(b)) - Number(needsYou(a));
      if (lifted !== 0) return lifted;
    }
    return oldest(a, b);
  });
}

/** Oldest first, with an unreadable date last and the id as the tiebreak. */
function oldest(a: JobSummary, b: JobSummary): number {
  const left = instant(a.created_at);
  const right = instant(b.created_at);
  if (left === null) return right === null ? a.id.localeCompare(b.id) : 1;
  if (right === null) return -1;
  return left === right ? a.id.localeCompare(b.id) : left - right;
}

/**
 * The count, stating both numbers.
 *
 * `4 jobs need you. 15 on the Board.` **Neither number alone says anything** —
 * the first is what a person is deciding whether to act on, the second is what
 * it is a fraction of, and a board reading `4 jobs` gives no sense of whether
 * that is the whole of it or a corner.
 *
 * Under a search the sentence changes shape rather than the numbers changing
 * meaning: the match count leads, because that is the number the person just
 * created, and how many of them need a person follows.
 */
export function countSentence(args: {
  /** Every job Bridge holds, before any filter. */
  total: number;
  /** How many the text match left, where there is one. */
  matched: number;
  /** How many of those need a person. */
  needsYou: number;
  /** The text match, as typed. Empty is no match. */
  query: string;
}): string {
  const board = `${args.total} on the Board.`;
  if (args.query.trim() === "") {
    return `${needsYouClause(args.needsYou)} ${board}`;
  }
  // The query itself is not echoed. It is in the field the person is looking
  // at, and a sentence carrying somebody's typing back at them breaks the
  // moment they type a quotation mark.
  if (args.matched === 0) return `No jobs match. ${board}`;
  const of = `${args.matched} of ${plural(args.total)} match.`;
  return args.needsYou === 0 ? of : `${of} ${args.needsYou} needs you.`;
}

/** "Nothing needs you", or how many do. */
function needsYouClause(count: number): string {
  if (count === 0) return "Nothing needs you.";
  return count === 1 ? "1 job needs you." : `${count} jobs need you.`;
}

/** "6 jobs", on its own. Lowercase anything countable. */
export function plural(total: number): string {
  return `${total} ${total === 1 ? "job" : "jobs"}`;
}

/**
 * What the empty state says when a filter emptied the list.
 *
 * **A board with nothing on it under a filter is not a Manifest with no jobs**,
 * and the two must not read alike. This names the filter that did it, so the
 * next press is obvious rather than guessed at.
 */
export function emptiedBy(tab: BoardTab, query: string): string | null {
  const named = BOARD_TABS.find((row) => row.id === tab);
  if (query.trim() === "") return tab === "all" ? null : (named?.empty ?? null);
  if (tab === "all") return "No jobs match your search.";
  return `No jobs match your search under ${named?.label ?? tab}.`;
}
