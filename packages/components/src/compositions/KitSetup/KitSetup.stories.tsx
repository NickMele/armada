import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";

import { KitSetup, type KitSetupRead } from "./KitSetup";

/**
 * The setup a person already works with, drawn beside what Armada itself holds
 * — #1491. A reading and not a control: nothing here reaches a drone, and
 * nothing here can be switched on.
 */
const meta: Meta<typeof KitSetup> = {
  title: "Compositions/Kit setup",
  component: KitSetup,
};
export default meta;

type Story = StoryObj<typeof KitSetup>;

const FULL: KitSetupRead = {
  harness: "An agent CLI",
  home: "/Users/user/.agent",
  present: true,
  kinds: [
    {
      kind: "skills",
      read: {
        what: "read",
        items: [
          {
            name: "humanizer",
            says: "Rewrite AI-sounding text so it reads like the writer without changing what it says.",
            source: "/Users/user/.agent/skills/humanizer",
          },
          {
            name: "impact-analysis",
            says: "Use when the user wants to know what will break if they change something.",
            source: "/Users/user/.agent/skills/impact-analysis",
          },
        ],
        unreadable: [],
      },
    },
    {
      kind: "plugins",
      read: {
        what: "read",
        items: [
          {
            name: "code-simplifier@official",
            says: "version 1.0.0",
            source: "/Users/user/.agent/plugins/cache/code-simplifier/1.0.0",
          },
        ],
        unreadable: [],
      },
    },
    {
      kind: "agent_file",
      read: {
        what: "read",
        items: [
          {
            name: "AGENTS.md",
            says: "21 lines, opening # Global Instructions",
            source: "/Users/user/.agent/AGENTS.md",
          },
        ],
        unreadable: [],
      },
    },
    { kind: "sub_agents", read: { what: "read", items: [], unreadable: [] } },
    { kind: "commands", read: { what: "read", items: [], unreadable: [] } },
    {
      kind: "mcp_servers",
      read: {
        what: "read",
        items: [
          {
            name: "tracker",
            says: "a program this machine starts",
            source: "/Users/user/.agent.json",
          },
        ],
        unreadable: [],
      },
    },
    {
      kind: "allowlist",
      read: { what: "not_read", why: "a rule drawn out of its two tiers reads as a grant, and both tiers are #41" },
    },
    {
      kind: "models",
      read: { what: "not_read", why: "which models a Job may use is resolved against a Manifest, and that is #41" },
    },
  ],
};

/** Nothing read yet. Not an empty setup — nobody has asked one. */
export const Reading: Story = {
  name: "Reading",
  args: { setup: undefined },
};

/**
 * A setup with things in it. **The counts are the point**: this screen used to
 * open on *Nothing in your Kit yet* with all of this one directory away.
 */
export const WhatSomebodyHas: Story = {
  name: "What somebody has",
  args: { setup: FULL },
  play: async ({ canvasElement }) => {
    const kit = within(canvasElement);
    await expect(kit.getByText("humanizer", { exact: true })).toBeVisible();
    // A kind nothing reads yet is named, never drawn as empty.
    await expect(kit.getAllByText(/Not read yet/)).toHaveLength(2);
    // The server is seen and is handed to nobody.
    await expect(kit.getByText(/handed none of them/)).toBeVisible();
  },
};

/** A file the person has and Armada cannot describe. Named, with the reason. */
export const SomethingWillNotRead: Story = {
  name: "Something will not read",
  args: {
    setup: {
      ...FULL,
      kinds: [
        {
          kind: "skills",
          read: {
            what: "read",
            items: [],
            unreadable: [
              {
                source: "/Users/user/.agent/skills/broken/SKILL.md",
                why: "front matter opens and never closes",
              },
            ],
          },
        },
      ],
    },
  },
  play: async ({ canvasElement }) => {
    const kit = within(canvasElement);
    await expect(kit.getByText(/would not read/)).toBeVisible();
  },
};

/** A machine where the harness has never run. Said, rather than drawn empty. */
export const NothingThereYet: Story = {
  name: "Nothing there yet",
  args: { setup: { ...FULL, present: false, kinds: [] } },
  play: async ({ canvasElement }) => {
    const kit = within(canvasElement);
    await expect(kit.getByText(/Nothing is there yet/)).toBeVisible();
  },
};
