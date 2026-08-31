import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { BoardControls, type BoardControlsProps } from "./BoardControls";

/**
 * The Job Board's filter set: state, plus one text match.
 *
 * **Five tabs, each carrying its own count in mono and the key that selects
 * it.** `Needs you` clusters the statuses that stop until a person reads them;
 * `Running` holds everything in flight, `piloted` included, because a job a
 * person has taken over is still moving.
 *
 * **The Manifest is not an axis and neither is origin.** The Board is already
 * scoped to one Manifest, and origin was drawn as a filter and rejected — it
 * answers a question asked after a row is found, not one that narrows a list.
 */
const meta: Meta<typeof BoardControls> = {
  title: "Compositions/Board controls",
  component: BoardControls,
};
export default meta;

type Story = StoryObj<typeof BoardControls>;

const SORTS = [
  { id: "critical_first", label: "Critical first" },
  { id: "oldest_first", label: "Oldest first" },
];

const TABS = [
  { id: "all", label: "All", count: 15, shortcut: "1" },
  { id: "needs-you", label: "Needs you", count: 4, shortcut: "2" },
  { id: "running", label: "Running", count: 6, shortcut: "3" },
  { id: "queued", label: "Queued", count: 2, shortcut: "4" },
  { id: "finished", label: "Finished", count: 3, shortcut: "5" },
];

/**
 * Live, because a control that cannot be operated is a picture of a control.
 *
 * It wires the two rules the surface owns, so a story can be typed into and
 * behave: the tab suspends while the field holds text, and choosing a tab
 * clears the field. Neither is the composition's — `BoardControls` draws
 * `suspended` and reports `onTab`, and `apps/desktop`'s `board.ts` is where the
 * real ones live.
 */
function Live(props: Partial<BoardControlsProps>) {
  const [query, setQuery] = useState(props.query ?? "");
  const [sort, setSort] = useState(props.sort ?? "critical_first");
  const [tab, setTab] = useState(props.tab ?? "all");
  return (
    <div className="armada-screen">
      <BoardControls
        sorts={SORTS}
        tabs={TABS}
        searchKey="/"
        {...props}
        query={query}
        onQuery={setQuery}
        sort={sort}
        onSort={setSort}
        tab={tab}
        onTab={(next) => {
          setTab(next);
          setQuery("");
        }}
        suspended={query.trim() !== ""}
      />
    </div>
  );
}

/**
 * At rest. `Critical first` is the default order — the needs-you cluster, then
 * oldest within every group — and `Oldest first` stays in the control.
 */
export const Resting: Story = { render: () => <Live /> };

/**
 * Typing, with the tab suspended. **The strip is bypassed, not changed** — it
 * steps back to `--fg-subtle` and the selected tab gives up its underline,
 * because an underline says "this is what you are looking at" and while a
 * search runs that is not true. The selection survives, so clearing the field
 * gives the person their filter back.
 *
 * Resetting the tab to `All` was the other reading and lost: it spends a choice
 * to make "search reads every job whatever tab is set" true, and then has
 * nothing to restore. Suspending makes the same sentence true for free.
 *
 * **The counts are of what the search matched, not of the board**, so a
 * suspended strip is still a breakdown of what is on screen — and the `Queued`
 * tab going to nothing answers "is there one I have not approved" without
 * pressing anything.
 *
 * Pressing a tab from here clears the search. A suspended control that did
 * nothing when pressed would be a dead one, and the way out has to work.
 */
export const Searching: Story = {
  render: () => (
    <Live
      query="poke"
      tab="running"
      tabs={[
        { id: "all", label: "All", count: 3, shortcut: "1" },
        { id: "needs-you", label: "Needs you", count: 1, shortcut: "2" },
        { id: "running", label: "Running", count: 2, shortcut: "3" },
        { id: "queued", label: "Queued", shortcut: "4" },
        { id: "finished", label: "Finished", shortcut: "5" },
      ]}
    />
  ),
};

/**
 * A tab with nothing behind it renders no count, never `0`. An empty queue is
 * the resting state of a healthy fleet, and a row of zeros trains the eye to
 * skip the number — which is the one thing it must not do when one changes.
 */
export const NothingNeedsYou: Story = {
  render: () => (
    <Live
      tab="needs-you"
      tabs={[
        { id: "all", label: "All", count: 9, shortcut: "1" },
        { id: "needs-you", label: "Needs you", shortcut: "2" },
        { id: "running", label: "Running", count: 6, shortcut: "3" },
        { id: "queued", label: "Queued", count: 3, shortcut: "4" },
        { id: "finished", label: "Finished", shortcut: "5" },
      ]}
    />
  ),
};
