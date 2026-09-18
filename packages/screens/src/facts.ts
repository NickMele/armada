// The header's field run on job detail: what a person reads about the Job,
// and nothing they would go and fetch.
//
// It is here rather than in `JobDetail.tsx` because it is one job — turning a
// served Job into a list of labelled facts — and the screen beside it is
// another. Nothing in this file decides how a fact looks; `JobDetailField` is
// the design system's shape and this only fills it in.
//
// # Readings, and the run is only readings now
//
// How long the Job has been going, what became of its pull request, who sent
// it and what it replaced. **The repository and the branch left in #1481**,
// and the argument is that they were already on the screen: *Where things are*
// draws both, one column down, each behind its own glyph and its own label. A
// value drawn twice is two values that can disagree, and the second copy was
// the one nobody could name — the owner read `armada/2-show-what-s-running-in-
// the-drones-stat` off this line and asked whether it was a branch, a worktree
// or an id.
//
// **The branch was also the handle said again.** Fleet builds a branch out of
// the Job's own handle, so the run opened with `2-show-what-s-running-in-the-
// drones-stat` and then repeated it three values later with `armada/` in
// front. Two thirds of the line was one string.
//
// **Spend and Turns went to Pulse in the same change.** They are worth
// knowing and they are not what a person opens a Job to read; Pulse is the
// region that answers *what is this costing and is it still working*, and both
// figures belong beside its processes rather than above its run. Step,
// Manifest, Model, Origin, Urgency, Drone and the write-scope overlap had all
// left before them, on the same rule — the ones worth reaching are under
// *Where things are*, which is the region for a value you want rather than one
// you are reading.
//
// # The pull request is one fact, and what became of it continues it
//
// `Pull request #4711` while it is open, `Pull request #4711, merged` once
// somebody has taken it. Two facts with the run's gap between them would read
// as two things to know; they are one thing — what the branch came to — said
// to whatever depth the record can say it, so the second continues the first.
//
// **A Job with no pull request draws neither**, and that is most Jobs: one
// still running, one in a repository with no remote, one that stopped before
// it delivered. The rule `pullRequestFact` and `landedFact` already keep.
//
// **Provenance closes the line, since #1115** — the mock's own order, and the
// registry `Row.tsx` names the gap in. Since #1439 that tail is two: who
// dispatched this job, then which job it replaced.

import type { JobDetailField } from "@armada/components";

import type { JobDetail as JobWhole, JobSummary, StepDetail } from "@armada/protocol";
import { freezeLineOf } from "./freeze";
import { fromAStudio, originReading } from "./origin";
import { leading } from "./reading";
import { LANDED, elapsedOf } from "./Row";

/**
 * The run, in the order the drawing runs it: what is holding this Job, what
 * became of its pull request, how long it has been going, and where it came
 * from. **Every one of them a reading** — nothing here is a value a person
 * came to fetch, and #1481 is where the two that were left.
 *
 * Run time is read off the row's own fields rather than off the detail. Both
 * carry them and cannot disagree — the detail is built from the same record —
 * and the row is already in hand, so the figure is there on the first frame
 * instead of appearing when `GET /jobs/:job_id` lands.
 */
export function factsOf(job: JobSummary, whole: JobWhole | null, now: number): JobDetailField[] {
  const address = whole?.delivery?.pull_request;
  return [
    ...freezeFact(job),
    ...pullRequestFact(address),
    ...landedFact(whole, address !== undefined),
    ...runTimeFact(job, now),
    ...dispatchedByFact(job),
    ...studioGoneFact(job, whole),
    ...redispatchedFromFact(job, whole),
  ];
}

/**
 * Which job this one replaced, where a redispatch minted it — the quiet half
 * of the callout on the job it replaced. `#1439`, `#1474`.
 *
 * **The handle, and pressable.** It drew `job.redispatched_from`, a raw ULID:
 * the one fact about where this job came from was the one a person could
 * neither read nor use. `replaces` is that id looked up. `foldLineages`
 * counts a chain; this names the direct predecessor and leaves the climb.
 *
 * **A forgotten predecessor is named, not left silent** — `studioGoneFact`'s
 * rule exactly: saying so is what keeps a dead end from reading as a job
 * nobody redispatched. The word is the record's, from `store::forget_job`.
 *
 * Nothing until the read lands, and the ULID never: drawing it was the bug.
 */
function redispatchedFromFact(job: JobSummary, whole: JobWhole | null): JobDetailField[] {
  const from = whole?.replaces;
  if (from !== undefined) {
    return [{ label: "Redispatched from", value: from.handle, mono: true, opensJob: from.job_id }];
  }
  // Before the read lands there is nothing to claim either way: a sentence
  // that appears and then disappears is worse than one that arrives late.
  if (whole === null || job.redispatched_from === undefined) return [];
  return [{ label: "The job it replaced has been forgotten" }];
}

/**
 * Who or what dispatched this Job, in the registry's own words. The reading is
 * `origin.ts`'s, shared with the board row that draws the same words since
 * #1362 — two spellings of one sentence is the thing that file exists to stop.
 */
function dispatchedByFact(job: JobSummary): JobDetailField[] {
  const reading = originReading(job);
  return reading === undefined ? [] : [{ value: reading }];
}

/**
 * That the Studio a Job came off is gone. **Only where there is nothing to
 * open** — the Studio that is still there is named under *Where things are*,
 * beside the worktree, where a value you want to reach lives.
 *
 * `origin` says the Job came off a Studio and `from_studio` says which, and
 * only the first survives the Studio being deleted. Saying so is what keeps a
 * dead end from reading as a Job nobody dispatched from one. `#1362`.
 */
function studioGoneFact(job: JobSummary, whole: JobWhole | null): JobDetailField[] {
  if (!fromAStudio(job)) return [];
  // Before the read lands there is nothing to claim either way: a sentence
  // that appears and then disappears is worse than one that arrives late.
  if (whole === null || whole.from_studio !== undefined) return [];
  return [{ label: "That Studio has been deleted" }];
}

/** Where a workflow came from, in Fleet's own words for `WorkflowSource` — #425. */
export const WORKFLOW_SOURCE: Readonly<Record<string, string>> = {
  armada: "carried by Armada",
  kit: "from Kit",
  repository: "from the repository",
};

/** `, from Kit`, or nothing where Fleet did not say. An unknown word renders as itself. */
export function sourceOf(source: string | undefined): string {
  if (source === undefined || source === "") return "";
  return `, ${WORKFLOW_SOURCE[source] ?? source}`;
}

/**
 * The pull request Fleet opened, by its number, clickable.
 *
 * **News from the moment it exists.** Every Job that finishes is in this state
 * — open, waiting on a reviewer — so a fact that waited for a merge would be
 * absent exactly when a person is looking for it, which is what sent the owner
 * to the forge to find a branch by hand. `#422`.
 *
 * **The number, never the address.** A forge address is sixty characters of
 * which a person reads four, and the run is a line of short readings. The whole
 * of it is on the link's `title`, which is where somebody who wants to copy it
 * can still get at it.
 *
 * **The number is read off the address rather than served.** Nothing on the
 * wire carries it: `JobDelivery` has the address and no id beside it, so the
 * choice was to parse or to draw something longer. A forge that numbers its
 * pull requests some other way falls back to the words alone, still linked —
 * see `pullRequestNumber`.
 */
function pullRequestFact(address: string | undefined): JobDetailField[] {
  if (address === undefined) return [];
  const number = pullRequestNumber(address);
  if (number === null) return [{ value: "Pull request", href: address }];
  return [{ label: "Pull request", value: number, mono: true, href: address }];
}

/**
 * The number out of a pull request address, or `null` where there is not one.
 *
 * **The last all-digit segment of the path, and that is the whole rule.**
 * `…/pull/4711` and `…/-/merge_requests/12` both answer, without this file
 * holding a list of forges — a roster of URL shapes would be a second statement
 * of something Fleet already resolved, and it would be wrong for the first
 * forge nobody thought of. Last rather than first, so an organisation or a
 * repository named in digits does not win over the number at the end.
 *
 * **The query and the fragment are cut before anything is read, and that is
 * not tidying.** A forge address that arrives with a line anchor on it ends
 * `#3000`, which is all digits and sits after the number — so reading them as
 * segments draws a line number where the pull request goes.
 *
 * `null` is a real answer and not a failure: a forge that addresses a pull
 * request by a slug is a forge whose pull requests have no number, and the fact
 * draws the words alone rather than inventing one.
 */
export function pullRequestNumber(address: string): string | null {
  const last = (address.split(/[?#]/)[0] ?? "")
    .split("/")
    .filter((part) => /^\d+$/.test(part))
    .at(-1);
  return last === undefined ? null : `#${last}`;
}

/**
 * What a person calls the code host a pull request's address names — its
 * domain, read off the address itself.
 *
 * **Never a friendly name this file invents for one vendor over another.**
 * `xtask/src/rules.rs`'s `no_vendor_literal_outside_adapters` holds the line
 * that only `crates/adapters` gets to know whose API Armada is talking to —
 * a vendor's name anywhere else, string or comment, is that boundary having
 * leaked — and Bridge does not get to cross it just because most pull
 * requests today happen to be on the same host. The domain a real address
 * names is not a name this file chose; it is Fleet's own record, read back.
 * Where the address does not even parse, the fallback names what it is
 * without pretending to know where.
 */
export function hostLabel(address: string): string {
  try {
    return new URL(address).hostname;
  } catch {
    return "the pull request's host";
  }
}

/**
 * What became of that pull request — continuing the fact that names it, where
 * there is one to continue.
 *
 * **Absent on nearly every Job, which is why it is beside the branch and not a
 * fact of its own line.** It appears the moment there is something to say and
 * takes no room until then: a Job with no remote, one still running, and one
 * whose pull request nobody has merged yet all draw nothing here, because
 * "nobody has merged it yet" is the state a pull request is in from the moment
 * it exists and is not news about this Job. The address above it is.
 *
 * **`continues` where the address is drawn, standalone where it is not.** With
 * one, this is the second half of a sentence and reads mid-line: `Pull request
 * #4711, merged`. Without one — a Job old enough that Fleet recorded the
 * verdict and not the address — it opens a fact of its own and takes a capital.
 */
function landedFact(whole: JobWhole | null, linked: boolean): JobDetailField[] {
  const landed = LANDED[whole?.delivery?.landed ?? ""];
  if (landed === undefined) return [];
  return linked ? [{ label: landed, continues: true }] : [{ label: leading(landed) }];
}

/** The freeze holding this Job, first: it is why nothing else on the line is moving. */
function freezeFact(job: JobSummary): JobDetailField[] {
  const line = freezeLineOf(job);
  return line === null ? [] : [{ label: line.lead, value: line.names, mono: true, suffix: line.tail }];
}

/**
 * How long the Job ran.
 *
 * **It ticks while the Job runs and stops where the Job stopped.** It read
 * `Elapsed 10h 29m` on a Job that had been dead since the morning, because the
 * figure was measured against the clock rather than against the instant the
 * Job stopped. `elapsedOf` is where that is decided now, for the Board row and
 * for this line both — #1481 fixed the same defect in the same subtraction.
 *
 * **Run time, not Elapsed.** Elapsed names a stopwatch and says nothing about
 * what it is timing, and half of what this figure had to say was that it had
 * stopped. Two words, the spelling the Board column already carries.
 *
 * **Absent, not from `created_at`, while the Job has never run** — waiting
 * for approval and waiting in the queue for a slot must not count.
 */
function runTimeFact(job: JobSummary, now: number): JobDetailField[] {
  const ran = elapsedOf(job, now);
  return ran === undefined ? [] : [{ label: "Run time", value: ran, mono: true }];
}

// # The money spellings, which outlived the fact that needed them
//
// `spent` and `money` were written for the header's own `Spend`, and that fact
// went to Pulse in #1481 while four other surfaces — the settings panel, a
// Helm thread, a cap warning and a verdict — had already come here for the
// spelling. They stay because that is what they are now: one way to write a
// figure Fleet reports in millionths, kept in one place so two screens cannot
// round the same dollar differently.

/**
 * A spend, and whether it is the whole of one.
 *
 * **A figure that is a floor says so.** Cost reaches Armada on the terminating
 * line of a session, so a drone signalled mid-run leaves none behind — and one
 * job read as `~$5.28` against a $5 cap while two of its six drones, stopped
 * after 277 and 299 seconds, had contributed nothing to the number a person was
 * deciding on.
 *
 * **`at least` rather than a footnote or a warning colour.** It is the same
 * fact either way and this is the shortest way to say it; a reader who does not
 * care reads past two words, and one who does is not sent hunting.
 */
export function spent(micros: number, unpriced: number | undefined): string {
  const shown = money(micros);
  return unpriced === undefined || unpriced === 0 ? shown : `at least ${shown}`;
}

/**
 * A spend, from millionths of a dollar. **Four places under a cent**, so a Job
 * that has cost a tenth of a penny does not round to `~$0.00` and read as free.
 */
export function money(micros: number): string {
  const dollars = micros / 1_000_000;
  return `~$${dollars.toFixed(dollars > 0 && dollars < CENT ? 4 : 2)}`;
}

/** A cent, in dollars. Below this two decimal places round to nothing. */
const CENT = 0.01;

/** The steps in the order they were frozen. `ordinal` is the order, not the array. */
export function ordered(whole: JobWhole | null): StepDetail[] {
  return whole === null ? [] : [...whole.steps].sort((a, b) => a.ordinal - b.ordinal);
}

/**
 * The rows of a step's own list — `check_runs` or `judged` — from the latest
 * attempt that has rows of that kind.
 *
 * **Read off the list itself, not off `step.attempts`.** A rerun gate or an
 * overrule records a fresh attempt without re-running the Checks, and
 * possibly before the Judge has answered again — so the step's newest
 * attempt can hold no rows of one kind, or of either. Narrowing to
 * `step.attempts.at(-1)` made that attempt's absence read as "nothing has
 * run", when what had actually not run was only this attempt; narrowing to
 * the list's own last row instead finds the latest attempt that has
 * something to show, whichever list is asked. `check_runs` and `judged` are
 * therefore narrowed separately, and can answer from different attempts on
 * the same step.
 *
 * **Since 7.0 both lists hold every attempt's rows, oldest first**, so the
 * latest attempt with rows of this kind is the one the last row names.
 */
export function onlyCurrentAttempt<T extends { attempt: number }>(rows: T[]): T[] {
  const attempt = rows.at(-1)?.attempt;
  return attempt === undefined ? rows : rows.filter((row) => row.attempt === attempt);
}
