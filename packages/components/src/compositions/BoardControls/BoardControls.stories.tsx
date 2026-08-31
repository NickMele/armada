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

/** Live, because a control that cannot be operated is a picture of a control. */
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
        onTab={setTab}
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
 * Typing. **The counts are of what the search matched, not of the board**, so
 * the strip says where the matches are as well as how many there are — and the
 * `Queued` tab going to nothing is the answer to "is there one I have not
 * approved" without pressing anything.
 *
 * The tab stays on `All` because the text match is not a state and does not
 * move the state control. What it does is read past it: the surface draws every
 * match whatever tab is set.
 */
export const Searching: Story = {
  render: () => (
    <Live
      query="poke"
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
