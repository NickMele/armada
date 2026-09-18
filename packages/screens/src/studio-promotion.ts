// What a person may promote, off the Studio and the selection — #1291. No React: every rule here is
// a function of the Studio Fleet sent and which nodes are picked on the whiteboard, so it is tested
// as one.
//
// **It offers, and refuses nothing.** Fleet holds every rule and answers with its own refusal; what
// is decided here is which acts to draw, so a person is not offered a button that always fails.

import type { Studio, StudioNode, StudioPosition } from "@armada/protocol";

/** The kinds a rung writes up. `docs/concepts/studio.md`, Promotion. */
const WRITABLE_UP = ["note", "cluster", "contradiction", "outline"] as const;

/**
 * The kinds that keep an address — #1394. **Read off the kind, never off the
 * address**: which host is the forge is `crates/adapters`' to know and the gate
 * refuses its name here, so a rule about forge links written in this file could
 * not be written at all.
 */
const KEEPS_AN_ADDRESS = ["link", "issue", "pull_request", "epic"] as const;

/** The three an address on a forge makes. A Link is one nothing recognised. */
const ON_A_FORGE = ["issue", "pull_request", "epic"] as const;

const isOneOf = (kinds: readonly string[], node: StudioNode) => kinds.includes(node.kind);

/** Whether the node carries an address of its own — a Link, an Issue, a Pull request or an Epic. */
export const keepsAnAddress = (node: StudioNode): boolean => isOneOf(KEEPS_AN_ADDRESS, node);

/** Which acts the current selection offers. Each is one control on the whiteboard's aside. */
export type StudioActs = {
  /** Two or more Notes: they can be accepted as one Cluster. */
  cluster: boolean;
  /** Two or more nodes of any kind: they can be read in order as one Outline. */
  outline: boolean;
  /** One node of a kind a rung writes up. */
  writeUp: boolean;
  /** One node: anything raised on it can be put off. */
  defer: boolean;
  /** One Issue draft: its title and body are a person's to change. */
  edit: boolean;
  /** One node with an address: the line beside it is a person's to change. #1378. */
  editLink: boolean;
  /**
   * One Issue draft, or one Issue, Pull request or Epic: its text or its
   * address goes through the Job proposer — #1379, #1394.
   */
  dispatch: boolean;
  /** One Contradiction that has not ended yet. */
  settle: boolean;
  /** One node with an address. Fleet decides whether it is a source; this only offers. */
  readIn: boolean;
};

const NOTHING: StudioActs = {
  cluster: false,
  outline: false,
  writeUp: false,
  defer: false,
  edit: false,
  editLink: false,
  dispatch: false,
  settle: false,
  readIn: false,
};

/** The selected nodes, in the order the whiteboard reports them, skipping any the Studio has lost. */
export function selectedNodes(studio: Studio, selected: readonly string[]): StudioNode[] {
  return selected.flatMap((id) => studio.nodes.filter((node) => node.id === id));
}

/**
 * What `selected` offers. **A Contradiction that has already ended offers nothing but an Outline**:
 * it ends once, and the rung that ended it drew the node that says where it went.
 */
export function actsOn(studio: Studio, selected: readonly string[]): StudioActs {
  const nodes = selectedNodes(studio, selected);
  if (nodes.length > 1) {
    return {
      ...NOTHING,
      outline: true,
      cluster: nodes.every((node) => node.kind === "note"),
    };
  }
  const [one] = nodes;
  if (one === undefined) return NOTHING;
  const ended = one.kind === "contradiction" && one.state !== "reported";
  return {
    ...NOTHING,
    writeUp: (WRITABLE_UP as readonly string[]).includes(one.kind) && !ended,
    defer: !ended,
    edit: one.kind === "issue_draft",
    editLink: keepsAnAddress(one),
    dispatch: dispatchedAs(one) !== null,
    settle: one.kind === "contradiction" && !ended,
    // **Offered on every node with an address**, because which addresses are
    // sources is `crates/adapters`' to know and a rule copied here would drift
    // from it. A Link to a board comes back refused, which is the answer.
    readIn: keepsAnAddress(one),
  };
}

/** How far to the right of what made it a promoted node is placed, in canvas units. */
const ACROSS = 320;

/**
 * Where a rung's new node goes: to the right of the rightmost node it was made from, level with the
 * topmost. **Placed rather than dropped at the origin**, because a Studio is laid out by hand and a
 * node landing under another is a move a person has to make before they can read either.
 */
export function placedBeside(studio: Studio, selected: readonly string[]): StudioPosition {
  const nodes = selectedNodes(studio, selected);
  if (nodes.length === 0) return { x: 0, y: 0 };
  return {
    x: Math.max(...nodes.map((node) => node.position.x)) + ACROSS,
    y: Math.min(...nodes.map((node) => node.position.y)),
  };
}

/** The first line of a body, for a node whose title is prose. */
function firstLine(body: string): string {
  return (
    body
      .split("\n")
      .find((line) => line.trim() !== "")
      ?.trim() ?? body
  );
}

/** What a node says, as one node's contribution to a write-up. */
export function saidBy(node: StudioNode): string {
  switch (node.kind) {
    case "note":
      return node.said;
    case "cluster":
      return node.title;
    case "contradiction":
      return `${node.first}\n${node.second}`;
    case "outline":
      return node.body;
    case "issue_draft":
      return `${node.title}\n\n${node.body}`;
    default:
      return "";
  }
}

/**
 * The draft a write-up opens with: the node's own words, for a person to edit. **A first draft and
 * not a summary** — nothing here shortens what was captured, because what is dispatched has to be
 * what the person read.
 */
export function aWriteUp(studio: Studio, node: StudioNode): { title: string; body: string } {
  const said = saidBy(node);
  const from = studio.edges
    .filter((edge) => edge.kind === "produced" && edge.to === node.id)
    .flatMap((edge) => studio.nodes.filter((one) => one.id === edge.from))
    .map(saidBy)
    .filter((words) => words !== "");
  const body = from.length === 0 ? said : `${said}\n\n${from.map((words) => `- ${words}`).join("\n")}\n`;
  return { title: firstLine(said), body };
}

/**
 * What dispatching this node sends, and `null` on one that dispatches nothing.
 *
 * **An Issue draft sends its own words; an Issue, a Pull request and an Epic
 * send their address** — the draft's title, a blank line, its body, which is
 * `crates/core-model`'s own shape; and for the other three the address alone,
 * because what it names is already filed and the Job proposer takes such a
 * link as a request (#1379, #1394).
 *
 * **Read off the kind, and a Link dispatches nothing**: a Link is an address
 * no adapter recognised — a board, a page, a session — so there is nothing
 * filed to dispatch against. Which workflow each of the three runs under is
 * the proposer's answer off each definition's `for_requests` line, not this
 * file's.
 */
export function dispatchedAs(node: StudioNode): string | null {
  if (node.kind === "issue_draft") return `${node.title}\n\n${node.body}`;
  return isOneOf(ON_A_FORGE, node) && "address" in node ? node.address : null;
}
