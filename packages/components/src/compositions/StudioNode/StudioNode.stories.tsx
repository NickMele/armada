import type { Meta, StoryObj } from "@storybook/react-vite";
import type { ReactNode } from "react";
import { expect } from "storybook/test";

import { JOB_STATUS } from "../../generated/vocabulary";
import { StudioNode } from "./StudioNode";

/**
 * One story per kind on `docs/concepts/studio.md`, each drawing every state the
 * page gives that kind. Run and Job are the only ones in colour.
 */
const meta: Meta<typeof StudioNode> = {
  title: "Compositions/Studio node",
  component: StudioNode,
};
export default meta;

type Story = StoryObj<typeof StudioNode>;

/**
 * A stand-in screenshot, at a screen's shape. **A named colour, and it is not a
 * design value**: what is inside a frame is a photograph of a window, and no
 * token of this design system applies to one — `FramesShown` says the same.
 */
function shot(fill: string, said: string): string {
  const svg =
    `<svg xmlns="http://www.w3.org/2000/svg" width="640" height="400">` +
    `<rect width="640" height="400" fill="${fill}"/>` +
    `<text x="24" y="48" fill="gainsboro" font-family="monospace" font-size="22">` +
    `${said}</text>` +
    `</svg>`;
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

function Row({ children }: { children: ReactNode }) {
  return (
    <div style={{ display: "flex", flexWrap: "wrap", gap: "var(--space-4)", alignItems: "flex-start" }}>
      {children}
    </div>
  );
}

/** Running, passed, failed and stopped, each in its `--run-*` hue, which aliases a Job colour. */
export const Run: Story = {
  render: () => (
    <Row>
      <StudioNode kind="run" state="running" title="pnpm test" facts={["test", "1m 12s"]} />
      <StudioNode kind="run" state="passed" title="cargo xtask verify-foundations" facts={["exit 0", "48s"]} />
      <StudioNode kind="run" state="failed" title="pnpm -C packages/components test" facts={["exit 1", "2m 04s"]} />
      <StudioNode kind="run" state="stopped" title="pnpm dev" facts={["6m 40s"]} />
    </Row>
  ),
  play: async ({ canvas }) => {
    // Only the running one is still working, and it says so to a screen reader.
    const busy = canvas.getByText("pnpm test").closest("[aria-busy]");
    await expect(busy).not.toBeNull();
    await expect(canvas.getByText("cargo xtask verify-foundations").closest("[aria-busy]")).toBeNull();
    // A run reads as a run, never with a Job's verb.
    for (const words of ["running", "passed", "failed", "stopped"]) {
      await expect(canvas.getByText(words)).toBeVisible();
    }
    await expect(canvas.queryByText("done")).toBeNull();
  },
};

/** No state: fixed at capture, and nothing writes to it. */
export const Note: Story = {
  args: {
    kind: "note",
    title: "The legend under the step bar is unreadable at this width",
    facts: ["Board", "JobRowStacked"],
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Note")).toBeVisible();
    // A kind with no state draws no badge and no state word.
    await expect(canvas.queryByText(/proposed|draft|open|frozen/)).toBeNull();
  },
};

/**
 * The picture a Note kept — #1352 — small on the card, still being read, and
 * missing for a reason. **A Note that kept none draws no plate at all**: a
 * board of dashed boxes would say every Note was meant to have one.
 */
export const NoteFrames: Story = {
  render: () => (
    <Row>
      <StudioNode
        kind="note"
        title="The legend under the step bar is unreadable at this width"
        frame={{ src: shot("darkslategray", "Job Board") }}
      />
      <StudioNode kind="note" title="It wraps at 720 wide" frame={{}} />
      <StudioNode
        kind="note"
        title="Queued and preparing read the same at a glance"
        frame={{ why: "This frame is on the Note and no longer on disk." }}
      />
      <StudioNode kind="note" title="No picture was taken for this one" />
    </Row>
  ),
  play: async ({ canvas }) => {
    // The picture is named by what it is, not by a description of what it shows.
    await expect(canvas.getByRole("img", { name: /captured from/ })).toBeVisible();
    // A read in flight says so; a settled absence says why instead.
    await expect(canvas.getByText("reading…")).toBeVisible();
    await expect(canvas.getByText(/no longer on disk/)).toBeVisible();
    // Four Notes, one picture: a Note that kept none draws nothing at all.
    await expect(canvas.getAllByRole("img")).toHaveLength(1);
  },
};

export const Cluster: Story = {
  args: { kind: "cluster", title: "The Board's legend is illegible", facts: ["3 notes"] },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Cluster")).toBeVisible();
    await expect(canvas.getByText("3 notes")).toBeVisible();
  },
};

/** Proposed, gathering and frozen. Gathering is the one that pulses — with no hue. */
export const Finding: Story = {
  render: () => (
    <Row>
      <StudioNode kind="finding" state="proposed" title="What writes the runtime file?" facts={["about $0.40"]} />
      <StudioNode kind="finding" state="gathering" title="Where the legend's colours come from" facts={["$0.12", "14 files read"]} />
      <StudioNode kind="finding" state="frozen" title="Fleet writes fleet.json once, at start" facts={["$0.31", "22 files read"]} />
    </Row>
  ),
  play: async ({ canvas }) => {
    await expect(canvas.getByText("gathering").closest("[aria-busy]")).not.toBeNull();
    await expect(canvas.getByText("proposed").closest("[aria-busy]")).toBeNull();
    await expect(canvas.getByText("frozen").closest("[aria-busy]")).toBeNull();
  },
};

/** Reported, then each of its four outcomes. */
export const Contradiction: Story = {
  render: () => (
    <Row>
      <StudioNode kind="contradiction" state="reported" title="bridge.md and react.md disagree on the pulse" facts={["bridge.md", "react.md"]} />
      <StudioNode kind="contradiction" state="issue_draft" title="The runtime file's path" facts={["2 sources"]} />
      <StudioNode kind="contradiction" state="deferral" title="Who owns the log's tail" facts={["2 sources"]} />
      <StudioNode kind="contradiction" state="not_a_problem" title="Width on the Board" facts={["2 sources"]} />
      <StudioNode kind="contradiction" state="resolved_here" title="Where Run colours are declared" facts={["2 sources"]} />
    </Row>
  ),
  play: async ({ canvas }) => {
    for (const words of ["reported", "issue draft", "deferral", "not a problem", "resolved here"]) {
      await expect(canvas.getByText(words)).toBeVisible();
    }
  },
};

export const Sketch: Story = {
  args: { kind: "sketch", state: "frozen", title: "The whiteboard's rail", facts: ["diagram"] },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Sketch")).toBeVisible();
    await expect(canvas.getByText("frozen")).toBeVisible();
  },
};

/** A GitHub address of the length the owner actually pasted, and then some. */
const LONG_ADDRESS =
  "https://github.com/NickMele/armada/issues/1378#issuecomment-2847190034-a-pasted-link-does-nothing";

const SHORT_ADDRESS = "https://github.com/NickMele/armada/issues/1378";

/**
 * The line a person wrote over the address they kept — #1378. Kept as its
 * address, which is a fact rather than a place this node navigates to.
 */
export const Link: Story = {
  args: {
    kind: "link",
    address: "https://miro.com/app/board/uXjVK",
    title: "The board the Studio proposal came off",
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("The board the Studio proposal came off")).toBeVisible();
    await expect(canvas.getByText("https://miro.com/app/board/uXjVK")).toBeVisible();
    await expect(canvas.queryByRole("link")).toBeNull();
  },
};

/**
 * A Link nobody wrote a line on, and one whose address is far wider than the
 * card. **Neither overflows** — #1378, where a pasted address ran past the
 * node's own width. The `play` reads what a person can see rather than a
 * measurement; the screenshot is what catches the overflow, so the assertion
 * here is that the whole address is still reachable on the title.
 */
export const LinkWithNoLine: Story = {
  render: () => (
    <Row>
      <StudioNode kind="link" address={SHORT_ADDRESS} title={SHORT_ADDRESS} />
      <StudioNode kind="link" address={LONG_ADDRESS} title={LONG_ADDRESS} />
      <StudioNode kind="link" address={LONG_ADDRESS} title="Where the review comments on the retry land" />
    </Row>
  ),
  play: async ({ canvas }) => {
    const clipped = canvas.getAllByTitle(LONG_ADDRESS);
    await expect(clipped.length).toBe(2);
    await expect(canvas.getByText("Where the review comments on the retry land")).toBeVisible();
  },
};

export const Deferral: Story = {
  render: () => (
    <Row>
      <StudioNode kind="deferral" state="open" title="Does a Studio need a Workspace?" facts={["blocks 1"]} />
      <StudioNode kind="deferral" state="answered" title="Does a scout read uncommitted changes?" />
    </Row>
  ),
  play: async ({ canvas }) => {
    await expect(canvas.getByText("open")).toBeVisible();
    await expect(canvas.getByText("answered")).toBeVisible();
  },
};

export const Outline: Story = {
  render: () => (
    <Row>
      <StudioNode kind="outline" state="draft" title="Run colours, then the pulse" facts={["4 parts"]} />
      <StudioNode kind="outline" state="frozen" title="Capture on Bridge" facts={["3 parts"]} />
    </Row>
  ),
  play: async ({ canvas }) => {
    await expect(canvas.getByText("draft")).toBeVisible();
    await expect(canvas.getByText("frozen")).toBeVisible();
  },
};

export const IssueDraft: Story = {
  args: { kind: "issue_draft", state: "draft", title: "A failed run and a passed one look the same" },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Issue draft")).toBeVisible();
    await expect(canvas.getByText("draft")).toBeVisible();
  },
};

/** Every Job status the registry carries, each read from it rather than retyped. */
export const Job: Story = {
  render: () => (
    <Row>
      {Object.keys(JOB_STATUS).map((status) => (
        <StudioNode key={status} kind="job" state={status} title="Run colours on the run sheet" facts={["j-1277"]} />
      ))}
    </Row>
  ),
  play: async ({ canvas }) => {
    for (const rendering of Object.values(JOB_STATUS)) {
      if (rendering?.verb) await expect(canvas.getAllByText(rendering.verb).length).toBeGreaterThan(0);
    }
    const running = JOB_STATUS.running?.verb ?? "running";
    await expect(canvas.getByText(running).closest("[aria-busy]")).not.toBeNull();
    await expect(canvas.getAllByText("Job").filter((kind) => kind.closest("[aria-busy]"))).toHaveLength(1);
  },
};

/** A status the registry has no verb for renders its wire spelling and says so. */
export const JobInAStatusWithNoVerb: Story = {
  args: { kind: "job", state: "paused_for_lunch", title: "A status Bridge has not heard of" },
  play: async ({ canvas }) => {
    await expect(canvas.getByTitle("No verb in the registry for paused_for_lunch")).toBeVisible();
  },
};

/**
 * A Run and a Job not read yet. A Studio holds a reference and never the state, so until the run
 * or the Job is read the card says no state rather than a guessed one, and nothing pulses.
 */
export const RunAndJobNotRead: Story = {
  render: () => (
    <Row>
      <StudioNode kind="run" title="01RUN00000000000000000000A" />
      <StudioNode kind="job" title="01JOB00000000000000000000B" />
    </Row>
  ),
};
