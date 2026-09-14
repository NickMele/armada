// Helm's conversation, folded from the wire into what `HelmThread` draws —
// #944. `story.ts`'s own shape, one subject over: the fold is written once
// here so a Storybook fixture and a live socket agree about what a row says.
//
// **Two voices, not three.** `asked` is a person's own words, and every `row`
// is Helm's — nobody redirects Helm the way Armada redirects a Drone, so
// `by` on the wire is read for nothing here and every reply row draws as
// `helm`.

import type { HelmThreadItem } from "@armada/protocol";
import type { HelmThreadRow } from "@armada/components";
import { clock } from "./duration";
import { money } from "./facts";

/** One item, as a row — or nothing, for the events a chat has no use for. */
export function helmRowsOf(items: readonly HelmThreadItem[]): HelmThreadRow[] {
  const rows: HelmThreadRow[] = [];
  for (const item of items) {
    const row = helmRowOf(item);
    if (row !== null) rows.push(row);
  }
  return rows;
}

function helmRowOf(item: HelmThreadItem): HelmThreadRow | null {
  const id = item.id;
  if (item.kind === "asked") {
    return { id, at: clock(item.ts), actor: "you", message: item.text };
  }
  if (item.kind === "fresh") {
    return {
      id,
      at: clock(item.ts),
      actor: "helm",
      message: "The stored session could not be resumed. This reply starts a new one.",
    };
  }
  if (item.kind === "unanswered") {
    return { id, at: clock(item.ts), actor: "helm", message: `No reply came. ${item.why}` };
  }
  // item.kind === "row"
  const saw = item.turn.saw;
  const at = clock(item.turn.ts);
  switch (saw.event) {
    case "said":
      return { id, at, actor: "helm", message: saw.text };
    case "called":
      return { id, at, actor: "helm", mono: true, message: `${saw.tool} ${saw.detail}`.trim() };
    case "ended":
      return { id, at, actor: "helm", message: costOf(saw.turns, saw.cost_micros) };
    case "refused":
      return { id, at, actor: "helm", mono: true, message: `${saw.tool} refused: ${saw.because}` };
    // Internal to the session: nothing a person reading a chat needs to see.
    case "started":
    case "answered":
    case "background_work":
      return null;
    default:
      return { id, at, actor: "helm", message: `Helm ${saw.event}` };
  }
}

/** "$0.02 · 3 turns", the reply's own cost — decided with the owner, every reply shows it. */
function costOf(turns: number, costMicros: number): string {
  return `${money(costMicros)} · ${turns} ${turns === 1 ? "turn" : "turns"}`;
}
