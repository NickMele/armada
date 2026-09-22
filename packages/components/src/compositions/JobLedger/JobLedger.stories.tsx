import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";
import { useState } from "react";
import { JobLedger, type JobLedgerRow } from "./JobLedger";

/**
 * One table of everything that happened to a Job, newest first, with a strip
 * that narrows it and the open row beside it.
 *
 * The rows, their words and which filter each answers to are the caller's.
 * This decides the chrome, where the panel goes at a width, and how much of an
 * endless list is drawn.
 */
const meta: Meta<typeof JobLedger> = {
  title: "Compositions/Job ledger",
  component: JobLedger,
};
export default meta;

type Story = StoryObj<typeof JobLedger>;

const ROWS: JobLedgerRow[] = [
  {
    id: "r1",
    when: "10:44",
    whenExact: "2026-09-22T10:44:00Z",
    where: "Implement · group 3",
    who: "check",
    whoSays: "Check",
    kind: "checked",
    what: "screens_test",
    outcome: "failed — 1 of 1384 failed: the Drones row opened the Board",
    tone: "failed",
  },
  {
    id: "r2",
    when: "10:14",
    whenExact: "2026-09-22T10:14:00Z",
    where: "Implement · group 2",
    who: "check",
    whoSays: "Check",
    kind: "checked",
    what: "typecheck",
    outcome: "passed",
    tone: "passed",
  },
  {
    id: "r3",
    when: "09:41",
    whenExact: "2026-09-22T09:41:00Z",
    where: "Implement · group 1 · T1",
    who: "drone",
    whoSays: "Drone",
    kind: "task_done",
    what: "T1 — Serve one read of everything running",
    outcome: "The read answers Drones, Checks and Judge calls in one call",
    tone: "passed",
  },
  {
    id: "r4",
    when: "09:21",
    whenExact: "2026-09-22T09:21:00Z",
    where: "Plan the change",
    who: "judge",
    whoSays: "Judge",
    kind: "judged",
    what: "The rail's Drones stat reads one running beside the machine's most",
    outcome: "met",
  },
  {
    id: "r5",
    when: "09:21",
    whenExact: "2026-09-22T09:21:00Z",
    where: "Plan the change",
    who: "fleet",
    whoSays: "Fleet",
    kind: "plan_recorded",
    what: "the plan was recorded",
    outcome: "8 tasks",
  },
  {
    id: "r6",
    when: "09:14",
    whenExact: "2026-09-22T09:14:00Z",
    where: "The Job itself",
    who: "you",
    whoSays: "You",
    kind: "status_queued",
    what: "approved the dispatch",
    outcome: "the workflow and the gates are frozen",
  },
];

/**
 * All is the total; the seven are families of it. They come to five, because
 * `status_queued` — the Job's own machine moving — answers to none of them.
 * `NOTE` is what tells a reader that rather than leaving the subtraction.
 */
const FILTERS = [
  { id: "all", label: "All", count: ROWS.length },
  { id: "evidence", label: "Evidence", count: 0 },
  { id: "files", label: "Files", count: 0 },
  { id: "checks", label: "Checks", count: 2 },
  { id: "judges", label: "Judges", count: 1 },
  { id: "drones", label: "Drones", count: 0 },
  { id: "tasks", label: "Tasks", count: 2 },
  { id: "tests", label: "Tests", count: 0 },
];

const NOTE = "One more row is under All alone: the Job's own machine moving, which no filter names.";

/** The ledger with nothing open — eight filters, and the table under them. */
export const OneLedger: Story = {
  args: {
    rows: ROWS,
    filters: FILTERS,
    filter: "all",
    onFilter: () => undefined,
    note: NOTE,
  },
  /**
   * **No row is counted twice, and the difference is said out loud.** The
   * defect this was built against is a board whose All read 34 while its
   * filters summed to 35 — so the families may never exceed All. They may come
   * to less, and where they do a reader is owed the line rather than the
   * subtraction.
   */
  play: async ({ args, canvas }) => {
    const all = args.filters.find((one) => one.id === "all")!.count;
    const families = args.filters
      .filter((one) => one.id !== "all")
      .reduce((total, one) => total + one.count, 0);

    await expect(families).toBeLessThanOrEqual(all);
    await expect(canvas.getByRole("note")).toHaveTextContent(/under All alone/);
  },
};

/** A row open, with the inspector beside the table. */
export const ARowOpen: Story = {
  args: {
    rows: ROWS,
    filters: FILTERS,
    filter: "all",
    onFilter: () => undefined,
    note: NOTE,
    openRow: "r1",
    inspector: <p className="armada-ledger__note">What this Check printed goes here.</p>,
  },
};

/** Under `--layout-breakpoint`: the Kind column gives way and the panel folds. */
export const Narrow: Story = {
  args: {
    rows: ROWS,
    filters: FILTERS,
    filter: "all",
    onFilter: () => undefined,
    narrow: true,
  },
  /**
   * **Kind is the column that gives way, and the identifier never does.** The
   * `what` beside it carries the same fact in words, so a reader loses a
   * spelling rather than a fact — and Where, which is what places a row in the
   * Job, is still drawn.
   */
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("columnheader", { name: "Kind" })).toBeNull();
    await expect(canvas.getByRole("columnheader", { name: "Where" })).toBeVisible();
  },
};

/** A filter that holds nothing. Never a bare strip over an empty frame. */
export const NothingUnderThisFilter: Story = {
  args: {
    rows: [],
    filters: FILTERS,
    filter: "evidence",
    onFilter: () => undefined,
    emptyNote: "No Drone has submitted evidence on this Job.",
  },
};

/** More rows than the bound draws. What was left out is counted, never dropped. */
export const BoundedAndSaidSo: Story = {
  args: {
    rows: [...ROWS, ...ROWS.map((row) => ({ ...row, id: `${row.id}-b` }))],
    filters: FILTERS,
    filter: "all",
    onFilter: () => undefined,
    bound: 4,
  },
  /**
   * **A bounded list says by how much.** A list that simply stopped would be
   * indistinguishable from a Job that did nothing else, which is the quiet
   * truncation the v1 failure log is full of.
   */
  play: async ({ canvas }) => {
    await expect(canvas.getByText(/8 older rows are not drawn/)).toBeVisible();
  },
};

/** The press reaches the surface from the row and from the keyboard alike. */
export const OpeningARow: Story = {
  render: function Opening({ onOpenRow, ...args }) {
    const [open, setOpen] = useState<string | null>(null);
    return (
      <JobLedger
        {...args}
        openRow={open}
        onOpenRow={(id) => {
          setOpen(id);
          onOpenRow?.(id);
        }}
        inspector={open === null ? undefined : <p className="armada-ledger__note">Row {open}</p>}
      />
    );
  },
  args: {
    rows: ROWS,
    filters: FILTERS,
    filter: "all",
    onFilter: () => undefined,
    onOpenRow: fn(),
  },
  /**
   * **One press is one report.** The button inside the cell sits inside the
   * row that handles the same act, so its click bubbles into it: on screen the
   * two are indistinguishable, because both name the same row — and a surface
   * counting presses hears the act twice. The spy is the only thing that can
   * see it, which is what earns one here.
   */
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: /screens_test/ }));

    await expect(canvas.getByText("Row r1")).toBeVisible();
    await expect(args.onOpenRow).toHaveBeenCalledTimes(1);
  },
};
