// The setup a person already works with, as the mock's own machine holds it —
// #1491. Its own file rather than a block in `manifest-fleet.ts`, which is at
// its length already.
//
// **No vendor here.** The harness names itself over the wire, and the mock is
// not the adapter.

import type { KitInventory } from "@armada/protocol";

/** What a person on this mock machine already has. */
export const KIT_INVENTORY: KitInventory = {
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
        unreadable: [
          {
            source: "/Users/user/.agent/skills/half/SKILL.md",
            why: "front matter opens and never closes",
          },
        ],
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
            name: "gitnexus",
            says: "a program this machine starts",
            source: "/Users/user/.agent.json",
          },
        ],
        unreadable: [],
      },
    },
    {
      kind: "allowlist",
      read: {
        what: "not_read",
        why: "a rule drawn out of its two tiers reads as a grant, and the tiers are #41",
      },
    },
    {
      kind: "models",
      read: {
        what: "not_read",
        why: "which models a Job may use is resolved against a Manifest, and that is #41",
      },
    },
  ],
};

