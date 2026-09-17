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

/** Kept as its address, which is a fact rather than a place this node navigates to. */
export const Link: Story = {
  args: { kind: "link", title: "Studio proposal board", facts: ["miro.com/app/board/uXjVK"] },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("miro.com/app/board/uXjVK")).toBeVisible();
    await expect(canvas.queryByRole("link")).toBeNull();
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
