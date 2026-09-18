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

/** `ask_person_to_approve`'s name on the wire — the inventory's key. `#1041`. */
export const APPROVAL_ASK_TOOL = "ask_person_to_approve";

/**
 * Whether a `called` row's `tool` is `ask_person_to_approve` — bare, the
 * inventory's own spelling, or as Helm's session actually names it,
 * `mcp__<server>__ask_person_to_approve`. The server name is `ipc::door::SERVER`,
 * named once in `crates/fleet/src/helm/hosting.rs` and never assumed here: a
 * suffix match reads right whatever it is spelled, which a Drone's own eight
 * tools reach through a different door and never collide with. The one place
 * this fold decides it, so a caller never repeats the rule.
 */
function isApprovalAsk(tool: string): boolean {
  return tool === APPROVAL_ASK_TOOL || tool.endsWith(`__${APPROVAL_ASK_TOOL}`);
}

/**
 * One `ask_person_to_approve` call, read off its own `called` row: the call's
 * id, for React's key, and the Job it named. Nothing else — resolving this
 * into a drawable card, off `BridgeState.jobs`, is the caller's, never this
 * fold's. `#1041`.
 */
export type HelmApprovalAsk = { id: string; jobId: string };

/** `helmRowsOf`'s own row shape: `HelmThreadRow` with `cards` — which only a caller holding `BridgeState.jobs` can resolve — replaced by the bare asks it would resolve into. */
export type HelmFoldedRow = Omit<HelmThreadRow, "cards"> & { asks?: HelmApprovalAsk[] };

/**
 * Every item, folded to rows — or nothing, for what a chat has no use for.
 *
 * **A reply is one bubble, however many rows it took.** `said` streams as
 * several turns, with a tool call or a refusal sitting between them where
 * Helm reached for something along the way; none of that is a fact the
 * person reading needs — Helm's own reply text says what it could or could
 * not do. So `said` text accumulates onto one open reply, `ended` closes it
 * with the cost as a quiet trailing line rather than a bubble of its own, and
 * every event between — `refused`, `started`, `answered`, `background_work`,
 * `unrecognised` and anything else the wire ever adds — draws nothing and
 * does not break the reply open. `called` is the one exception: where the
 * tool is the approval ask, bare or prefixed (`isApprovalAsk`), its Job joins
 * the open reply's `asks` rather than being dropped with the rest.
 */
export function helmRowsOf(items: readonly HelmThreadItem[]): HelmFoldedRow[] {
  const rows: HelmFoldedRow[] = [];
  let open: { id: string; at: string; texts: string[]; asks: HelmApprovalAsk[] } | null = null;

  const flush = (meta?: string) => {
    if (open === null) return;
    rows.push({
      id: open.id,
      at: open.at,
      actor: "helm",
      message: open.texts.join("\n\n"),
      ...(open.asks.length === 0 ? {} : { asks: open.asks }),
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
        // **Lower case, because Fleet's own sentences sit in this column** —
        // `crates/fleet/src/helm/unanswered.rs` writes them that way and
        // cannot do otherwise, since some of them lead with the program that
        // would not start and the lexicon refuses a program name promoted to
        // an actor. One clause and not two sentences, which is the record's
        // own wording for this line (`HelmRecord/record.ts`). The rule is in
        // `docs/contracts/design-system.md`, *Prose rules*.
        message: "the stored session could not be resumed, so this reply starts a new one",
      });
      continue;
    }
    if (item.kind === "unanswered") {
      flush();
      // Fleet's sentence whole and unframed. A lead-in here was a second
      // producer of one sentence, and the reply-budget timeout said the same
      // thing twice; `crates/fleet/src/helm/unanswered.rs` owns it now.
      rows.push({ id: item.id, at: clock(item.ts), actor: "helm", message: item.why });
      continue;
    }
    // item.kind === "row"
    const saw = item.turn.saw;
    if (saw.event === "said") {
      open ??= { id: item.id, at: clock(item.turn.ts), texts: [], asks: [] };
      open.texts.push(saw.text);
      continue;
    }
    if (saw.event === "called" && isApprovalAsk(saw.tool)) {
      const jobId = saw.detail.trim();
      if (jobId !== "") {
        open ??= { id: item.id, at: clock(item.turn.ts), texts: [], asks: [] };
        open.asks.push({ id: saw.call, jobId });
      }
      continue;
    }
    if (saw.event === "ended") {
      flush(costOf(saw.turns, saw.cost_micros));
      continue;
    }
    // A tool call this fold has no use for, a refusal, or a session event
    // nobody watching a chat needs — leave the open reply as it is and read on.
  }
  flush();
  return rows;
}

/** "$0.02 · 3 turns", the reply's own cost — decided with the owner, every reply shows it. */
function costOf(turns: number, costMicros: number): string {
  return `${money(costMicros)} · ${turns} ${turns === 1 ? "turn" : "turns"}`;
}
