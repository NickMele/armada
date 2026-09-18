/**
 * One Helm session as the text a person pastes into an issue. #1367.
 *
 * **One producer, the way `ErrorNotice/payload.ts` is one.** The sheet renders
 * this exact string in a `<pre>` and the control copies it, so what was read
 * on screen is what arrives in the issue body by construction rather than by
 * two renderings agreeing about field order.
 *
 * **Text and not JSON**, and the columns are `aligned`'s — the two artifacts
 * are the same act on different subjects, so they read the same.
 */

import type { HelmDebugInfo, HelmDebugLine, HelmDebugText } from "@armada/protocol";
import { PROTOCOL_VERSION, spoken } from "@armada/protocol";

import { aligned, COPIED } from "../../errors/ErrorNotice/payload";

/** Every block below the head is indented one step — the payload's own step. */
const INDENT = "  ";

/** Where a list of tool names wraps. Wide enough to read, narrow enough to survive an issue body. */
const COLUMN = 76;

/** The instant on a thread line, as a reader needs it: the clock, not the date. */
function clock(at: string): string {
  const time = at.slice(11, 19);
  return time === "" ? at : time;
}

/** A cut piece of prose, with what it was cut from. A whole one says nothing. */
function prose(text: HelmDebugText): string {
  if (text.of === undefined) return text.text;
  const shown = [...text.text].length;
  return `${text.text}… (showing ${shown} of ${text.of} characters)`;
}

/** `$0.0243 · 3 turns · 1 refused`, the caption the dock's own thread draws. */
function ended(cost_micros: number, turns: number, refusals: number): string {
  const dollars = `$${(cost_micros / 1_000_000).toFixed(4)}`;
  const turned = `${turns} ${turns === 1 ? "turn" : "turns"}`;
  return refusals === 0 ? `${dollars} · ${turned}` : `${dollars} · ${turned} · ${refusals} refused`;
}

/**
 * One line of the thread: when, who, and what.
 *
 * **Three columns and never a wrap**: a reply is prose and keeps its own
 * newlines, which `aligned` indents to the value column rather than folding.
 */
function line(said: HelmDebugLine): [string, string] {
  const at = clock(said.at);
  switch (said.line) {
    case "asked":
      return [`${at}  you`, prose(said.text)];
    case "said":
      return [`${at}  helm`, prose(said.text)];
    case "called":
      return [`${at}  helm`, said.detail === "" ? `called ${said.tool}` : `called ${said.tool} · ${said.detail}`];
    case "refused":
      return [`${at}  helm`, `refused ${said.tool} · ${said.because}`];
    case "ended":
      return [`${at}  turn`, ended(said.cost_micros, said.turns, said.refusals)];
    case "fresh":
      return [`${at}  fleet`, "the stored session was gone, so this reply started a new one"];
    case "unanswered":
      return [`${at}  fleet`, `no reply came: ${said.why}`];
  }
}

/** A comma-joined list, wrapped at [`COLUMN`] and indented one step. */
function wrapped(names: string[]): string[] {
  const lines: string[] = [];
  let held = "";
  for (const name of names) {
    const next = held === "" ? name : `${held}, ${name}`;
    if (next.length > COLUMN && held !== "") {
      lines.push(`${INDENT}${held},`);
      held = name;
      continue;
    }
    held = next;
  }
  if (held !== "") lines.push(`${INDENT}${held}`);
  return lines;
}

/** The thread's heading, which says what it left out rather than being quietly short. */
function threadHeading(kept: number, cut: number): string {
  if (kept === 0 && cut === 0) return "thread — nothing has been said in this conversation yet";
  if (cut === 0) return "thread";
  return `thread — the last ${kept} lines, ${cut} older cut`;
}

/**
 * The record, as the text that goes on the clipboard and onto the screen.
 *
 * Order is fixed and is the order it is read in: what the session is, the
 * brief that bounds it, the roster it held, what was said, and what it had
 * polled. A caller cannot reorder it — two records that differ in row order
 * are two artifacts nobody can diff.
 */
export function helmRecord(record: HelmDebugInfo): string {
  const head: [string, string][] = [
    ["repository", record.manifest_id],
    ["checkout", record.checkout],
    ["authority", record.authority === "acting" ? "acting, on your ask" : "read-only"],
    ["model", record.model],
  ];
  if (record.session !== undefined) head.push(["session", record.session]);
  if (record.servers !== undefined) {
    head.push(["mcp servers", `${record.servers}, as this session came up`]);
  }

  const blocks: string[][] = [["armada helm session"], aligned(head, "")];

  blocks.push(["brief", ...record.brief.split("\n").map((said) => `${INDENT}${said}`)]);

  blocks.push([
    `tools — ${record.tools.length} on the ${record.door} door, each called as ${record.door}__<name>`,
    ...wrapped(record.tools),
  ]);

  const thread = record.thread.map(line);
  blocks.push([
    threadHeading(thread.length, record.cut),
    ...(thread.length === 0 ? [] : aligned(thread, INDENT)),
  ]);

  if (record.polled !== undefined) {
    const { from, upto, kinds, missed } = record.polled;
    const counted = kinds.reduce((total, one) => total + one.count, 0);
    const heading = `events — its last poll was told ${counted}, from cursor ${from} up to ${upto}`;
    const rows: [string, string][] = kinds.map((one) => [one.kind, String(one.count)]);
    if (missed !== undefined) rows.push(["dropped before they were counted", String(missed)]);
    blocks.push([heading, ...(rows.length === 0 ? [`${INDENT}nothing had happened`] : aligned(rows, INDENT))]);
  }

  // The payload's own tail, two spaces apart: one reads as a sentence and
  // three as a table, and this is neither.
  blocks.push([
    [
      `bridge protocol ${spoken(PROTOCOL_VERSION)}`,
      `fleet protocol ${spoken(record.protocol_version)}`,
      `fleet run ${record.run_id}`,
      `taken ${record.at}`,
    ].join("  "),
  ]);

  return blocks.map((block) => block.join("\n")).join("\n\n");
}

/**
 * Put a record on the clipboard, and say so either way.
 *
 * **The one implementation of the act**, so the control and the keyboard do
 * the same thing — `copyDebugInfo`'s rule, one subject over. A clipboard write
 * is silent by nature and a failed one is indistinguishable from a dead
 * control, so the surface is told either way.
 */
export function copyHelmRecord(record: HelmDebugInfo, onCopied?: (what: string) => void): void {
  // `COPIED` is the error payload's own word, not a second one: the toast says
  // the same thing whichever artifact was copied, because it is one act.
  const said = () => onCopied?.(COPIED);
  void navigator.clipboard.writeText(helmRecord(record)).then(said, said);
}

/**
 * What the record holds, said before it is sent. **Stating what is in it, not
 * promising what is not** — the error payload's safety rule, and its sentence
 * cannot be reused: nothing here is a structured field a type bounds. A thread
 * is the person's own words and the model's, and a checkout path names their
 * disk.
 */
export const HELM_SAFETY =
  "This holds the brief, the tool roster, your own messages, Helm's replies and the checkout's path on this machine. " +
  "Nothing bounds what any of that says — read it before you send it.";
