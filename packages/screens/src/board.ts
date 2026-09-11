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
// # The six tabs partition the twelve statuses, and the registry does most of
// the partitioning
//
// No list of statuses is written here. `job-statuses.toml` says whether a Job
// is over and what it is doing, the codegen carries both onto `JOB_LIFECYCLE`,
// and four of the five state tabs are read off those two fields plus the one
// status the `Queued` tab is named after. **`tabOf` itself is in `needs-you.ts`
// now**, so that main can read the same rule; what it does is unchanged and the
// table below is still what it does:
//
// | Tab | Rule |
// |---|---|
// | Cleared | `reclaimed_at` is set — a row fact, not a registry one, and read first |
// | Finished | `terminal`, and `reclaimed_at` is not set |
// | Running | `mode` is `Working` — `running` and `piloted`, because a job somebody has taken over is still moving |
// | Needs you | `who_is_acting` is `Person`, **or the row says `asking`** |
// | Queued | `who_is_acting` is `Drone` — `queued` |
//
// **`Cleared` is not a registry state.** `reclaimed_at` is a fact `#570`
// added to the row when a job's worktree and branch are given back while its
// record stays — a `Cleared` job's own `status` is still whatever it stopped
// at, terminal either way. Splitting it out of `Finished` is what keeps a
// bulk clear from reading as though it deleted the row it was asked to keep.
//
// **Every tab is a positive rule and none is a leftover.** `Needs you` was
// reached by subtraction until `who_is_acting` was emitted: not over, not
// working, not queued. That made its membership depend on the absence of a rule
// rather than on the rule, so the first status added anywhere would have joined
// it silently. The registry has said `who_is_acting = "Person"` on exactly those
// four all along; the codegen just was not carrying it.
//
// **`asking` is the one rule that is not a lifecycle row, and still a positive
// one.** A drone waiting on an answer is on a `running` job and its status does
// not move while it waits, so the registry cannot say this. Without the row's
// own fact a question would sit under Running, invisible until somebody opened
// that job — the failure `Needs you` exists to prevent.
//
// **The order of the two mode-and-actor tests is the one thing to read
// carefully.** `piloted` is `Working` and its actor is a `Person`, so `Working`
// has to be asked first or a job somebody is piloting lands under `Needs you`
// while they are the one working it.
//
// **A status matching none of the four is unplaceable**, which is now something
// that can happen and be seen rather than a case that silently fell into
// `Queued` — a non-terminal, non-working status with `who_is_acting = "None"`
// would be exactly that.
//
// # Search reads every job whatever tab is set, and the tab is suspended
//
// **The sentence is about what search reaches, not an instruction to move the
// tab.** While the field holds text the state tab is bypassed — the list is
// every match — and the tab itself does not change: it stays where the person
// put it, drawn set back, and clearing the search restores it.
//
// Resetting it to `All` was built first and was wrong for one reason: it spends
// a filter the person chose in order to make a sentence true, and then cannot
// give it back. Suspending makes the same sentence true and costs nothing.
//
// **Choosing a tab clears the search**, whether by `1`–`6` or by clicking one.
// A suspended tab that did nothing when pressed would be a dead control, and
// pressing one is asking for a state rather than for a match — so the search is
// what gives way, in the direction that has an undo. Retyping a query is a
// keystroke; recovering a tab you did not know you had lost is not.
//
// Every tab's count stays a count of what the search matched, so a suspended
// strip is still a breakdown of what is on screen — and a tab that empties
// under a search renders no count rather than a `0`.

import type { JobSummary } from "@armada/protocol";
import type { WorkflowSummary } from "@armada/protocol";
import { needsYou, oldest, tabOf } from "./needs-you";
import type { BoardTab } from "./needs-you";

/** The tab names, defined beside the rule that sorts a job into one. */
export type { BoardTab, StateTab } from "./needs-you";

/**
 * The tabs, their labels, the key that selects each — `1` through `6`, in the
 * order they are drawn — and what each says when it holds nothing.
 *
 * The key is the position, which is why it is here and not typed into a
 * component: a tab moving in this list moves its key with it, and a strip whose
 * second tab answered `3` would be unlearnable.
 *
 * **Each empty line is written, not templated.** "Nothing is under running" is
 * what a template produces and nobody says; the six sentences below are what a
 * person would say, and there are only six of them.
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
  // Split out of `Finished` by `#570`: a reclaimed job is still terminal, and
  // a tab that could not tell it apart is the defect the issue was filed over.
  { id: "cleared", label: "Cleared", shortcut: "6", empty: "Nothing has been cleared." },
];

/** Where the Board opens. Everything, because nothing has been asked yet. */
export const FIRST_TAB: BoardTab = "all";

/**
 * The membership rule, and the sentence that counts it — re-exported so nothing
 * that already reads them from here has to learn a second path.
 *
 * **They live in `needs-you.ts` because main needs them too.** A notification
 * fires when a job *enters* this set, and that decision is made in the process
 * that holds the connection rather than in a window that may be closed — so the
 * rule had to sit somewhere a Node bundle can reach without dragging React
 * behind it. The rule itself did not change.
 */
export { needsYou, needsYouClause, oldest, tabOf } from "./needs-you";

/** Whether a tab admits a job. `all` admits every one, including the unplaceable. */
export function inTab(job: JobSummary, tab: BoardTab): boolean {
  return tab === "all" || tabOf(job) === tab;
}

/**
 * Whether the state tab is suspended — bypassed, and not changed.
 *
 * A text match is not a state, so while one is running the tab does not narrow.
 * It is not reset either: the person's choice is still there and comes back the
 * moment the field is empty.
 */
export function tabSuspended(query: string): boolean {
  return query.trim() !== "";
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
 * What the table view's columns are called, in the order the row supplies its
 * facts. The badge and the actions take no name: a status reads as one without
 * a header, and a column of buttons is not a fact about the Job.
 *
 * **Three, not four.** `Dispatched by` is the fourth fact the Board wants and
 * the one it cannot draw. The words exist: `enum-verbs.toml` carries all five
 * `origin` rows and `sub_dispatched` carries a form rather than a word, both
 * settled when #234 closed. Two things are still missing, and only the first is
 * a line: the generator's wanted list does not name `origin`, so no `ORIGIN`
 * map reaches Bridge; and `JobSummary` carries the full five-value `Origin`
 * without `dispatched_by`, so on a sub-dispatched row the form has nothing to
 * fill its slot with. Naming a column before both are answered would reserve a
 * track for a value that never arrives, which reads as one that failed to load.
 */
export const BOARD_COLUMNS = ["Workflow", "Progress", "Run time"];

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

/**
 * The All tab's sections, in the order a person works down them: what needs
 * them, what is running, what is waiting its turn, and what is over. **Done is
 * finished and cleared together**, the owner's ruling on 11 Sep 2026: a Job
 * whose worktree was given back is still a Job that ended.
 */
export const BOARD_SECTIONS = [
  { id: "needs-you", label: "Needs you" },
  { id: "running", label: "Running" },
  { id: "queued", label: "Queued" },
  { id: "done", label: "Done" },
  // A Job no tab claims, which `tabOf` answers `null` for. Drawn under its
  // own label rather than dropped: a grouped list that loses a row is worse
  // than one with a fifth heading.
  { id: "other", label: "Other" },
] as const;

export type BoardSection = (typeof BOARD_SECTIONS)[number]["id"];

/** Which section of the All tab a Job is drawn under. */
export function sectionOf(job: JobSummary): BoardSection {
  const tab = tabOf(job);
  if (tab === "finished" || tab === "cleared") return "done";
  return tab ?? "other";
}

/** The sections that have rows in them, each keeping the order it was given. */
export function sectionsOf(
  jobs: readonly JobSummary[],
): { id: BoardSection; label: string; jobs: JobSummary[] }[] {
  return BOARD_SECTIONS.map((section) => ({
    id: section.id,
    label: section.label,
    jobs: jobs.filter((job) => sectionOf(job) === section.id),
  })).filter((section) => section.jobs.length > 0);
}

/**
 * What the empty state says when a filter emptied the list.
 *
 * **A board with nothing on it under a filter is not a Manifest with no jobs**,
 * and the two must not read alike. This names the filter that did it, so the
 * next press is obvious rather than guessed at.
 */
export function emptiedBy(tab: BoardTab, query: string): string | null {
  // The tab is suspended while a search runs, so it did not do this and naming
  // it would send a person to clear the wrong control.
  if (tabSuspended(query)) return "No jobs match your search.";
  if (tab === "all") return null;
  return BOARD_TABS.find((row) => row.id === tab)?.empty ?? null;
}
