/**
 * Drone brief — what Armada told the Drone, in the blocks it was written in.
 *
 * **Named for whose brief it is.** `JobBrief` is the requester's half: what done
 * means and what context the Job carries. This is Fleet's turn into a Drone's
 * context, and a component called `Brief` beside that one would be one name
 * covering two different things.
 *
 * **A blank line is the boundary, and nothing here reads a heading.**
 * `crates/fleet/src/briefing.rs` writes every block as its heading, a blank
 * line, then the body — `JOB BRIEF`, `WHERE YOU ARE`, `STEP:`, `WHAT THIS PART
 * DELIVERS` — so grouping at the blank lines puts each heading in an element of
 * its own without this component deciding what a heading looks like. Marking one
 * as a heading belongs on the wire, on `LogLine.named`, and is therefore
 * `briefing.rs`'s to set: #306 says so, and that is a protocol change rather
 * than a rendering one.
 *
 * **A block keeps its own line breaks, because those newlines carry the
 * meaning.** The parts rail is one part per line, indented, with `STOP.` under
 * the part the Drone is on. Joined into a paragraph that boundary lands
 * mid-sentence, and the parts list, the delivery path and the stop — the three
 * things a person opens this chapter to find — end up buried in prose. That is
 * the whole of #306, and it was a default `white-space` discarding newlines
 * that were on the wire the entire time.
 *
 * **No font size, for `Prose`'s reason.** The type scale belongs to
 * `docs/contracts/design-system.md`; this draws inside a chapter body at
 * `--text-xs` and reads at whatever size the surface around it sets.
 */
export type DroneBriefProps = {
  /**
   * The turn's payload, one line per element, in the order Fleet wrote them.
   *
   * **Lines rather than one string**, because that is the shape the wire
   * already carries: a turn's payload is lines, and `story.ts` splits it with a
   * doc comment saying the newlines are the author's. Handing this a string to
   * split again would be one reading done twice.
   */
  lines: readonly string[];
};

export function DroneBrief({ lines }: DroneBriefProps) {
  const blocks = briefBlocks(lines);
  if (blocks.length === 0) return null;
  return (
    <div className="armada-brief">
      {blocks.map((block, at) => (
        <p className="armada-brief__block" key={at}>
          {widenIndent(block)}
        </p>
      ))}
    </div>
  );
}

/**
 * Every leading space, doubled.
 *
 * **This is a deliberate divergence from the text a Drone was sent, and it has
 * a cost worth stating.** `briefing.rs` indents a part of the rail by two
 * spaces and the stop under it by five, which is legible in a monospaced
 * context window and is not legible at `--text-xs` in a 602px panel — two
 * spaces there is roughly seven pixels, and the list read as prose that had
 * been nudged. Widened here, the same brief now has one shape for a person and
 * another for the model, so a person reading this rail and a person reading the
 * transcript Fleet sent are not looking at identical strings.
 *
 * **The alternative was widening it in `briefing.rs`, and that is worse.** The
 * indent would then be chosen for a panel it is never drawn in, and
 * `docs/contracts/agent-prompt.md` refuses shape rules in the baseline. A
 * rendering decision belongs on the surface that renders.
 *
 * A multiplier rather than a fixed pad, so relative depth survives: the stop
 * stays deeper than the part it sits under, at whatever depths Fleet wrote.
 */
export function widenIndent(block: string): string {
  return block
    .split("\n")
    .map((line) => {
      // Spaces only. A tab is not something `briefing.rs` writes, and
      // `trimStart` would eat one and change the line's width silently.
      const deep = /^ */.exec(line)?.[0].length ?? 0;
      return " ".repeat(deep * INDENT) + line.slice(deep);
    })
    .join("\n");
}

/** What one of Fleet's spaces of indent is drawn as. The smallest step that
    reads as a list at `--text-xs`, found in the story rather than derived. */
const INDENT = 2;

/**
 * The lines, grouped into blocks at the blank ones.
 *
 * **A run of blank lines is one boundary.** An empty block draws as an empty
 * element, which is a gap a reader cannot tell from a block that failed to
 * render.
 *
 * Exported because it is arithmetic and is tested as arithmetic — in
 * `packages/screens`, in node, where a hundred briefs cost what one costs. A
 * `play` that computed this would be a unit test paying a browser's price.
 */
export function briefBlocks(lines: readonly string[]): string[] {
  const blocks: string[] = [];
  let held: string[] = [];
  for (const line of lines) {
    if (line.trim() === "") {
      if (held.length > 0) blocks.push(held.join("\n"));
      held = [];
      continue;
    }
    held.push(line);
  }
  if (held.length > 0) blocks.push(held.join("\n"));
  return blocks;
}
