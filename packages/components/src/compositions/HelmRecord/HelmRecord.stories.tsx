import type { Meta, StoryObj } from "@storybook/react-vite";
import type { HelmDebugInfo } from "@armada/protocol";
import { expect, userEvent } from "storybook/test";

import { HelmRecord } from "./HelmRecord";
import { helmRecord } from "./record";

/** One Helm session, read before it is copied — #1367. */
const meta: Meta<typeof HelmRecord> = {
  title: "Compositions/Helm record",
  component: HelmRecord,
  parameters: { layout: "fullscreen" },
  render: (args) => (
    <div style={{ position: "relative", height: "100vh", background: "var(--bg-sunken)" }}>
      <HelmRecord {...args} />
    </div>
  ),
};
export default meta;

type Story = StoryObj<typeof HelmRecord>;

const BRIEF = [
  "You are Helm, in Armada. A person asks you about the work in one repository.",
  "",
  "THIS REPOSITORY",
  "",
  "Every question in this conversation is about Manifest armada, read from /work/armada.",
].join("\n");

const record: HelmDebugInfo = {
  manifest_id: "armada",
  checkout: "/Users/user/Development/armada",
  authority: "acting",
  model: "a-model",
  brief: BRIEF,
  door: "armada-fleet",
  tools: ["list_jobs", "get_job", "get_events_since", "start_checkout_run", "ask_the_person", "approve_dispatch"],
  servers: 19,
  session: "63a1f0d2-2f2b-4a07-9f0e-5d0d2c4d1b77",
  thread: [
    { at: "2026-09-17T14:29:40.000Z", line: "asked", text: { text: "Read docs/scope.md and tell me what is out." } },
    { at: "2026-09-17T14:29:44.000Z", line: "called", tool: "Read", detail: "/work/armada/docs/scope.md" },
    { at: "2026-09-17T14:29:46.000Z", line: "refused", tool: "Read", because: "the person refused it" },
    {
      at: "2026-09-17T14:29:52.000Z",
      line: "said",
      text: { text: "I cannot read that file, so I have nothing to answer from." },
    },
    { at: "2026-09-17T14:29:52.000Z", line: "ended", turns: 3, cost_micros: 24_300, refusals: 1 },
  ],
  cut: 12,
  polled: {
    from: 41,
    upto: 58,
    kinds: [
      { kind: "job.state_changed", count: 9 },
      { kind: "drone.spawned", count: 8 },
    ],
  },
  run_id: "01K5RJ0F5H7TZ8QK6M9R1V2WXY",
  protocol_version: { major: 15, minor: 1 },
  at: "2026-09-17T14:31:02.117Z",
};

/**
 * The record a person carries to an issue. The `play` is the whole claim of
 * `#1367`: what is read on screen is what the control copies, and it names the
 * brief, the roster, the authority and the turns.
 */
export const Read: Story = {
  args: { open: true, record },
  play: async ({ canvas }) => {
    const shown = canvas.getByText(/armada helm session/);
    const written = helmRecord(record);
    await expect(shown).toHaveTextContent("acting, on your ask", { normalizeWhitespace: false });
    await expect(shown.textContent).toBe(written);
    // The four things the issue says decide an answer, in the one string.
    await expect(written).toContain("You are Helm, in Armada.");
    await expect(written).toContain("get_events_since");
    await expect(written).toContain("$0.0243 · 3 turns · 1 refused");
    await expect(written).toContain("its last poll was told 17, from cursor 41");
    // A long session says what it cut rather than being quietly short.
    await expect(written).toContain("12 older cut");
  },
};

/**
 * The control and the binding run one function, which is what stops the
 * clipboard and the screen from drifting apart. The `play` presses both and
 * reads what was written.
 */
export const Copied: Story = {
  args: { open: true, record },
  play: async ({ canvas, args }) => {
    const written: string[] = [];
    // `navigator.clipboard` is a getter, so the write is replaced rather than
    // the object — which is also what a headless browser leaves undefined.
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText: (text: string) => (written.push(text), Promise.resolve()) },
    });
    await userEvent.click(canvas.getByRole("button", { name: "Copy debug info" }));
    await expect(written[0]).toBe(helmRecord(args.record!));
    await userEvent.keyboard("c");
    await expect(written).toHaveLength(2);
    await expect(written[1]).toBe(written[0]);
  },
};

/** Fleet was asked and has not answered. */
export const Reading: Story = {
  args: { open: true, reading: true },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("status")).toHaveTextContent("Reading the session…");
  },
};

/** Fleet would not answer, in its own words. */
export const Unreadable: Story = {
  args: { open: true, failed: "Fleet is not connected, so the session cannot be read." },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("status")).toHaveTextContent("Fleet is not connected");
  },
};

/** Nothing has been said yet: the record still names what bounds the answers. */
export const NothingSaidYet: Story = {
  args: { open: true, record: { ...record, thread: [], cut: 0, session: undefined, servers: undefined, polled: undefined } },
  play: async ({ canvas }) => {
    await expect(canvas.getByText(/armada helm session/)).toHaveTextContent(
      "thread — nothing has been said in this conversation yet",
    );
  },
};
