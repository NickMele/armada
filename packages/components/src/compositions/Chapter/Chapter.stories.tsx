import type { Meta, StoryObj } from "@storybook/react-vite";
import { Button } from "../../primitives/Button/Button";
import { Kbd } from "../../primitives/Kbd/Kbd";
import { Chapter } from "./Chapter";

const meta: Meta<typeof Chapter> = {
  title: "Compositions/Chapter",
  component: Chapter,
  decorators: [
    // The panel a chapter lives in. A well on the canvas would be judged
    // against the wrong ground: the chapter is --bg-sunken precisely because
    // the panel around it is --bg-raised.
    (Story) => (
      <div
        style={{
          width: "calc(var(--space-12) * 12)",
          padding: "var(--space-4)",
          borderRadius: "var(--radius-md)",
          border: "var(--border-width) solid var(--border-default)",
          background: "var(--bg-raised)",
        }}
      >
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof Chapter>;

/**
 * Open. The header carries the number, the name and the meta; the body carries
 * the chapter, and the accent line at its foot is the one thing in it that
 * leads anywhere.
 */
export const Open: Story = {
  args: {
    ordinal: 1,
    name: "Drone instructions",
    meta: "14:22:07",
    open: true,
    onToggle: () => {},
    bodyId: "chapter-open",
    children: (
      <p style={{ margin: 0, fontSize: "var(--text-xs)", lineHeight: "var(--leading-xs)", color: "var(--fg-muted)" }}>
        Move the selector block into its own module so the tests can import it without constructing
        the store. Do not change reducer behaviour.
      </p>
    ),
    moreLabel: "Criteria and what it was given — 2 and 2",
    onMore: () => {},
  },
};

/**
 * Collapsed to its header line. **Not collapsed to nothing** — the number, the
 * name and the meta stay, so what happened in the step is readable at a glance
 * while you are deep in one part of it.
 *
 * The rule under the header goes with the body. A one-line chapter with a rule
 * along its bottom reads as a body that failed to render.
 */
export const CollapsedToItsHeader: Story = {
  args: {
    ordinal: 1,
    name: "Drone instructions",
    meta: "14:22:07",
    open: false,
    onToggle: () => {},
    bodyId: "chapter-collapsed",
    children: <p>Never seen while the chapter is shut.</p>,
  },
};

/**
 * The meta a person actually reads a collapsed chapter for. `3 files · +94 −31
 * · all inside the plan` is three claims in a line, and the last of them is
 * the one worth having before the chapter is opened.
 */
export const WithHeaderMeta: Story = {
  args: {
    ordinal: 3,
    name: "Produced",
    meta: "3 files · +94 −31 · all inside the plan",
    open: false,
    onToggle: () => {},
    bodyId: "chapter-meta",
  },
};

/**
 * The activity log, streaming. The dot is what says the chapter is live rather
 * than a snapshot — a count says how many entries there are and only the dot
 * says they are still arriving.
 *
 * It does not pulse. The pulse is one per screen and it belongs on the step
 * the Drone is working.
 */
export const Live: Story = {
  args: {
    ordinal: 2,
    name: "Activity log",
    live: true,
    meta: "live · 47 entries",
    open: false,
    onToggle: () => {},
    bodyId: "chapter-live",
  },
};

/**
 * The chapter that asks rather than reports, on a step stopped at a human
 * gate. Amber on the header alone: what is inside it is not a warning, and a
 * tinted body would render a step stopped with nothing wrong as a failure.
 */
export const TheChapterThatNeedsYou: Story = {
  args: {
    ordinal: 4,
    name: "Your decision",
    meta: "nothing advances until you answer",
    tone: "waiting",
    open: true,
    bodyId: "chapter-waiting",
    children: (
      <p style={{ margin: 0, fontSize: "var(--text-xs)", lineHeight: "var(--leading-xs)", color: "var(--fg-muted)" }}>
        Every Check passed and both criteria were met. This workflow asks for a person at this step
        whatever the gates came to.
      </p>
    ),
  },
};

/**
 * The header line at the width `.armada-inside`'s own arithmetic leaves the
 * panel at `--window-floor` (`screens.css`, `380px against --window-floor's
 * 768 leaves 363 for the panel`) — not a number invented for this story.
 *
 * **The regression this guards.** The name and the act used to be two
 * unshrinkable neighbours on one flex row, with the header's meta caught
 * between them: at this width the turn count printed over the act
 * (`1mOpen the log`) and the name — which chapter this is — drew nothing at
 * all. The name never shrinks now; the meta ellipsises first, and the act
 * moves to a line of its own rather than share one it no longer fits on.
 */
export const AtTheFloor: Story = {
  name: "At the floor width",
  render: () => (
    <div style={{ width: "calc(var(--window-floor) - var(--w-run-column) - var(--space-6))" }}>
      <Chapter
        ordinal={2}
        name="Working"
        meta="105 turns · 1m 46s"
        open
        onToggle={() => {}}
        bodyId="chapter-floor"
        act={
          <Button variant="ghost" size="sm" onClick={() => {}}>
            Open the log
            <Kbd>L</Kbd>
          </Button>
        }
      >
        <p style={{ margin: 0, fontSize: "var(--text-xs)", lineHeight: "var(--leading-xs)", color: "var(--fg-muted)" }}>
          The activity log's own rows, folded here — this story is about the header line above them.
        </p>
      </Chapter>
    </div>
  ),
};

/**
 * **What opening one actually does.** The chapter you open grows in place and
 * the others fall back to one line each — the order never changes, the story
 * stays on screen, and one thing is long at a time.
 *
 * One open at a time is the region's rule, not the chapter's: a component
 * cannot enforce a constraint about its siblings, and this story is the region
 * doing it.
 */
export const OneOpenAtATime: Story = {
  render: () => (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
      <Chapter
        ordinal={1}
        name="Drone instructions"
        meta="14:22:07"
        open={false}
        onToggle={() => {}}
        bodyId="one-1"
      />
      <Chapter
        ordinal={2}
        name="Activity log"
        live
        meta="47 entries"
        open
        onToggle={() => {}}
        bodyId="one-2"
        moreLabel="Close"
        moreCloses
        onMore={() => {}}
      >
        <p
          style={{
            margin: 0,
            fontSize: "var(--text-xs)",
            lineHeight: "var(--leading-xs)",
            color: "var(--fg-muted)",
          }}
        >
          Forty-seven entries, in the order they happened.
        </p>
      </Chapter>
      <Chapter
        ordinal={3}
        name="Produced"
        meta="3 files · +94 −31"
        open={false}
        onToggle={() => {}}
        bodyId="one-3"
      />
    </div>
  ),
};
