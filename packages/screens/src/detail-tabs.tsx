// Job detail's five destinations, and the strip that chooses between them.
//
// **One label per tab, written once.** The design boards call the same
// destination "Plan", "Plan the split" and "Plan 4 groups" in three places, and
// a screen that repeats a name is a screen where two of them drift. Every
// surface that has to say a tab's name reads `TAB_LABEL`.
//
// **A tab counts what is behind it, or says nothing.** `TabsWithCounts` renders
// zero as no count at all, so a count is only ever a figure this screen has
// actually read: the workflow's steps, and the plan's tasks. `Record` and
// `Pulse` count nothing yet — the ledger and the process table are `#1537` and
// `#1538` — and `Overview` is the arrangement itself rather than a queue.

import { TabsWithCounts } from "@armada/components";
import type { JobDetail } from "@armada/protocol";

/** The five destinations, in the order the strip draws them. */
export const DETAIL_TABS = ["overview", "workflow", "plan", "record", "pulse"] as const;

export type DetailTab = (typeof DETAIL_TABS)[number];

/** Where a reader lands, and what `every-state` opens on: today's arrangement. */
export const FIRST_TAB: DetailTab = "overview";

/**
 * The one spelling of each tab's name. Sentence case, one word each — the
 * noun the journey doc names the destination by.
 */
export const TAB_LABEL: Record<DetailTab, string> = {
  overview: "Overview",
  workflow: "Workflow",
  plan: "Plan",
  record: "Record",
  pulse: "Pulse",
};

/** What each destination answers, in the tab's own words. For the journey doc and the strip alike. */
export const TAB_NOUN: Record<DetailTab, string> = {
  overview: "needs-you",
  workflow: "machinery",
  plan: "work",
  record: "produced",
  pulse: "cost",
};

/**
 * What a tab has behind it, where it has a number at all.
 *
 * Read off the Job Fleet answered with, never off the Board's row: the row
 * carries neither the frozen workflow's steps nor the plan's tasks, so a Job
 * whose detail has not arrived draws no counts rather than wrong ones.
 */
export function countsOf(whole: JobDetail | null): Partial<Record<DetailTab, number>> {
  if (whole === null) return {};
  return {
    workflow: whole.steps.length,
    // Tasks a Drone may still do. A dropped task is not work outstanding, and
    // counting it would make the figure climb as a plan is pruned.
    ...(whole.work_plan === undefined
      ? {}
      : { plan: whole.work_plan.tasks.filter((task) => task.state !== "dropped").length }),
  };
}

export type JobTabsProps = {
  value: DetailTab;
  onChange: (tab: DetailTab) => void;
  counts: Partial<Record<DetailTab, number>>;
};

/**
 * The strip under the Job header. **The whole of navigation inside a Job**,
 * which is why it carries its own name: there is no heading beside it to be
 * one, and a reader who cannot see it would otherwise hear five tabs and never
 * what they divide.
 */
export function JobTabs({ value, onChange, counts }: JobTabsProps) {
  return (
    <div className="armada-screen__detail-tabs">
      <TabsWithCounts
        label="Job detail"
        value={value}
        onChange={(id) => onChange(id as DetailTab)}
        items={DETAIL_TABS.map((tab) => ({
          id: tab,
          label: TAB_LABEL[tab],
          ...(counts[tab] === undefined ? {} : { count: counts[tab] }),
        }))}
      />
    </div>
  );
}
