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

/**
 * Every item, folded to rows — or nothing, for what a chat has no use for.
 *
 * **A reply is one bubble, however many rows it took.** `said` streams as
 * several turns, with a tool call or a refusal sitting between them where
 * Helm reached for something along the way; none of that is a fact the
 * person reading needs — Helm's own reply text says what it could or could
 * not do. So `said` text accumulates onto one open reply, `ended` closes it
 * with the cost as a quiet trailing line rather than a bubble of its own, and
 * every event between — `called`, `refused`, `started`, `answered`,
 * `background_work`, `unrecognised` and anything else the wire ever adds —
 * draws nothing and does not break the reply open.
 */
export function helmRowsOf(items: readonly HelmThreadItem[]): HelmThreadRow[] {
  const rows: HelmThreadRow[] = [];
  let open: { id: string; at: string; texts: string[] } | null = null;

  const flush = (meta?: string) => {
    if (open === null) return;
    rows.push({
      id: open.id,
      at: open.at,
      actor: "helm",
      message: open.texts.join("\n\n"),
      ...(meta === undefined ? {} : { meta }),
    });
    open = null;
  };

  for (const item of items) {
    if (item.kind === "asked") {
      flush();
      rows.push({ id: item.id, at: clock(item.ts), actor: "you", message: item.text });
      continue;
    }
    if (item.kind === "fresh") {
      flush();
      rows.push({
        id: item.id,
        at: clock(item.ts),
        actor: "helm",
        message: "The stored session could not be resumed. This reply starts a new one.",
      });
      continue;
    }
    if (item.kind === "unanswered") {
      flush();
      rows.push({ id: item.id, at: clock(item.ts), actor: "helm", message: `No reply came. ${item.why}` });
      continue;
    }
    // item.kind === "row"
    const saw = item.turn.saw;
    if (saw.event === "said") {
      open ??= { id: item.id, at: clock(item.turn.ts), texts: [] };
      open.texts.push(saw.text);
      continue;
    }
    if (saw.event === "ended") {
      flush(costOf(saw.turns, saw.cost_micros));
      continue;
    }
    // A tool call, a refusal, or a session event nobody watching a chat
    // needs — leave the open reply as it is and read on.
  }
  flush();
  return rows;
}

/** "$0.02 · 3 turns", the reply's own cost — decided with the owner, every reply shows it. */
function costOf(turns: number, costMicros: number): string {
  return `${money(costMicros)} · ${turns} ${turns === 1 ? "turn" : "turns"}`;
}
