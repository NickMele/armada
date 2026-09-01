/**
 * The debug info an error carries — one artifact, quotable.
 *
 * **This is what a person pastes into an issue, a message or a terminal**, and
 * the only thing that makes it worth having is that it is complete without the
 * screen it came from. Nothing here is decorative and nothing here is prose:
 * every line is a fact the machine had.
 *
 * # Why text rather than JSON
 *
 * It is read by a person before it is read by anything else. Aligned columns
 * in mono at 11pm are readable; a JSON object with escaped newlines in the
 * stack is not, and neither is a fenced block pasted into a terminal. **No
 * fences** — the same characters have to survive an issue body, a chat message
 * and a shell scrollback, and a fence only helps in one of the three.
 *
 * # Why one string, not two renderings
 *
 * `debugInfo` is the only thing that formats a payload. The expanded view
 * renders this exact string in a `<pre>`, so what is on screen is what is on
 * the clipboard by construction rather than by two renderers agreeing about
 * field order. The issue asked for "the same order in the clipboard"; one
 * producer is how that stops being something to keep in step.
 *
 * # Absent is absent
 *
 * Every field below is omitted from the output when it is not there. A failure
 * that precedes a Job shows no `job_id` row rather than a blank one, and an
 * error with no structured fields shows no `fields` block.
 *
 * **`code` is the single exception, and it renders `none`.** The error
 * treatment guarantees a code on every error and shows it always, so a reader
 * meeting a payload without one cannot tell whether the failure carried none
 * or whether the paste was truncated — and the payload is read away from the
 * screen that would have answered that. `none` is a statement of fact, not a
 * minted code: nothing in Bridge mints one, because a code's declaration lives
 * beside the variant that raises it and an invented one would be in no
 * manifest.
 *
 * Two failures reach here with no code, and neither is a defect: a renderer
 * exception never went near the wire, and `UnreadableJob` is a row-level fault
 * Fleet sends as a bare sentence rather than as a `WireError`.
 */

/**
 * One structured field, already flattened to a string.
 *
 * `key` is the wire's spelling and is **not** rewritten into sentence case —
 * it is what somebody greps a log for, and a prettified label joins to
 * nothing.
 */
export type DebugField = {
  key: string;
  value: string;
};

/**
 * Everything the payload can carry. A rendering of `ipc::WireError` plus the
 * three facts that are not on the wire and are the first thing anyone asks:
 * both protocol versions and when it was taken.
 */
export type DebugPayload = {
  /** The `code` off the wire. Absent where the failure carried none. */
  code?: string;
  /** What failed. Always present — every failure has at least a sentence. */
  message: string;
  /** The emitting process's instance. Absent on a failure raised in Bridge. */
  run_id?: string;
  /** Absent on a failure that precedes a Job. */
  job_id?: string;
  /** A retry is a second Drone id under one Job. */
  drone_id?: string;
  /** From the WorkflowDef, never generated. */
  step_id?: string;
  /** The wire's `fields`, in the order they should be read. May be empty. */
  fields?: DebugField[];
  /** The flattened cause chain, outermost cause first. May be empty. */
  chain?: string[];
  /**
   * The protocol Bridge speaks, written `5.2`. **Not an application version** —
   * Bridge holds none, and labelling a protocol version as the app's would put
   * a wrong fact in the one artifact whose whole use is being quoted.
   */
  bridgeProtocol: string;
  /** The protocol Fleet speaks. Absent where Bridge never read a runtime file. */
  fleetProtocol?: string;
  /**
   * When the payload was taken, ISO 8601, on the caller's clock.
   *
   * **Taken, not raised**, and the tail says so. Nothing on the wire carries
   * when a failure happened — `WireError` has no timestamp — and a bare
   * instant appended to an error reads as the moment it broke. A banner is a
   * standing condition and can be copied hours after the thing it reports.
   */
  at: string;
};

/** The value column starts three spaces past the longest label in its block. */
const GAP = 3;

/** Every block below the guaranteed rows is indented one step. */
const INDENT = "  ";

/** What the `code` row says where the failure carried none. */
const NO_CODE = "none";

/**
 * A two-column block, aligned on the longest label.
 *
 * A value containing newlines keeps them, and its continuation lines are
 * indented to the value column rather than collapsed onto one line. Losing a
 * newline out of a machine value to keep a column tidy is the wrong trade in
 * an artifact that exists to be complete.
 */
function aligned(rows: [string, string][], indent: string): string[] {
  const width = Math.max(...rows.map(([label]) => label.length)) + GAP;
  return rows.map(([label, value]) => {
    const pad = " ".repeat(indent.length + width);
    const [head, ...rest] = value.split("\n");
    const first = `${indent}${label}${" ".repeat(width - label.length)}${head ?? ""}`;
    return [first, ...rest.map((line) => `${pad}${line}`)].join("\n");
  });
}

/**
 * The chain, as an ordered list.
 *
 * **Expanded, never folded.** It is the one part that explains the code, and
 * three lines do not earn a disclosure. Numbers are right-aligned so a
 * ten-entry chain does not step its own text sideways at the tenth.
 */
function ordered(chain: string[]): string[] {
  const width = String(chain.length).length;
  return chain.map((entry, at) => `${INDENT}${String(at + 1).padStart(width)}  ${entry}`);
}

/**
 * The payload, as the text that goes on the clipboard and onto the screen.
 *
 * Order is fixed and is the order it is read in: the guaranteed fields, then
 * the structured fields, then the chain, then the versions and the time. A
 * caller cannot reorder it, because the shape it takes is not the caller's
 * business — two payloads that differ in row order are two artifacts nobody
 * can diff.
 */
export function debugInfo(payload: DebugPayload): string {
  const guaranteed: [string, string][] = [
    ["code", payload.code ?? NO_CODE],
    ["message", payload.message],
  ];
  if (payload.run_id !== undefined) guaranteed.push(["run_id", payload.run_id]);
  if (payload.job_id !== undefined) guaranteed.push(["job_id", payload.job_id]);
  if (payload.drone_id !== undefined) guaranteed.push(["drone_id", payload.drone_id]);
  if (payload.step_id !== undefined) guaranteed.push(["step_id", payload.step_id]);

  const blocks: string[][] = [["armada error"], aligned(guaranteed, "")];

  const fields = payload.fields ?? [];
  if (fields.length > 0) {
    blocks.push([
      "fields",
      ...aligned(
        fields.map((field): [string, string] => [field.key, field.value]),
        INDENT,
      ),
    ]);
  }

  const chain = payload.chain ?? [];
  if (chain.length > 0) blocks.push(["chain", ...ordered(chain)]);

  // Appended, never on the wire. Two spaces between them: one reads as a
  // sentence and three as a table, and this is neither.
  const tail = [`bridge protocol ${payload.bridgeProtocol}`];
  if (payload.fleetProtocol !== undefined) tail.push(`fleet protocol ${payload.fleetProtocol}`);
  tail.push(`taken ${payload.at}`);
  blocks.push([tail.join("  ")]);

  return blocks.map((block) => block.join("\n")).join("\n\n");
}

/**
 * Put a payload on the clipboard, and say so either way.
 *
 * **The one implementation of the act**, so the control and the keyboard do the
 * same thing rather than two things that agree today. `c` is `copy debug info`
 * in the contextual key map, and a binding that reimplemented the write would
 * be a second artifact the moment either side changed.
 *
 * It copies `debugInfo(payload)` — the exact string the expanded view renders,
 * with the same instant in its tail. What is read on screen is what arrives in
 * the issue body, including when it was taken.
 *
 * `onCopied` is called on failure as well as on success, and is given the name
 * of what was copied rather than the fifteen lines of it. **A clipboard write
 * is silent by nature**, and a failed one is indistinguishable from a dead
 * control, so the surface is told either way and raises the same toast.
 */
export function copyDebugInfo(payload: DebugPayload, onCopied?: (what: string) => void): void {
  const said = () => onCopied?.(COPIED);
  void navigator.clipboard.writeText(debugInfo(payload)).then(said, said);
}

/** What the toast says was copied. A noun: the artifact is a block, not a value. */
export const COPIED = "The debug info";

/**
 * Why the payload is safe to quote, and where that stops.
 *
 * **Two sentences, because one cannot be written that is true.** The first half
 * is exactly right about `fields`: `ipc::WireValue` is five primitive variants,
 * `Secret<T>` implements no `Display` and no `Serialize`, so formatting a
 * credential into a field does not compile. Getting one in needs an explicit
 * `expose()`, which is a deliberate act and greppable in one search.
 *
 * It says nothing about the rest of the payload, and the payload is not only
 * `fields`. `message` and `chain` are prose written by whatever error `Display`
 * impl raised them, and no type bounds what an author put there. Rendering the
 * bounded claim over the whole artifact would promise an outcome the mechanism
 * does not reach — and the mechanism is what the reader was owed.
 *
 * It also makes no claim about the wider context: a credential sitting in a
 * repository file was never a `Secret<T>` and the type system was never
 * involved.
 *
 * **Here rather than beside the expanded view, because two surfaces say it.**
 * The expanded view shows it under the payload, and the file-an-issue dialog
 * shows it against the envelope row — where it is the whole reason that row
 * cannot be marked bounded and waved through. Two copies would be two claims
 * about one mechanism, and the day the mechanism changed only one would move.
 */
export const SAFETY =
  "Structured fields carry primitives only, and a credential does not compile into one. " +
  "Nothing bounds the message or the chain, which are prose an error wrote — read them before you send this.";

/**
 * What the act is called, wherever it appears — the control, the palette and
 * the tooltip beside its binding.
 *
 * **Taken from the contextual key map verbatim**, in sentence case. The map
 * binds `c` to `copy debug info`, and an action carries one verb: a control
 * that says something else is a second name for one act, and the palette
 * displays the map's.
 */
export const COPY_DEBUG_INFO = "Copy debug info";
