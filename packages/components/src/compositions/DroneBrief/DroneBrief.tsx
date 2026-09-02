/**
 * Drone brief — what Armada told the Drone, in the blocks it was written in.
 *
 * **Named for whose brief it is.** `JobBrief` is the requester's half: what done
 * means and what context the Job carries. This is Fleet's turn into a Drone's
 * context, and a component called `Brief` beside that one would be one name
 * covering two different things.
 *
 * **A heading is a heading because the wire says which line it is.**
 * `crates/fleet/src/briefing.rs` writes every block as its heading, a blank
 * line, then the body, and it names the heading's line number on
 * `Saw.instructed.headings` as it writes it. So this draws a heading without
 * deciding what one looks like — no first line of a block, no line in capitals.
 * Both of those guesses are wrong on briefs Fleet already writes: the baseline
 * opens with prose, and what the part before produced opens its block with a
 * sentence. Before the marker existed the gap above a heading was the only
 * thing marking it, which is #318.
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
 * `--text-xs` and reads at whatever size the surface around it sets. A heading
 * here is weight and colour and never a size, which is also what keeps it from
 * competing with the chapter's own title above it.
 */
export type DroneBriefProps = {
  /**
   * The turn's payload, one line per element, in the order Fleet wrote them.
   *
   * **Lines rather than one string**, because that is the shape the wire
   * already carries: a turn's payload is lines, and `story.ts` splits it with a
   * doc comment saying the newlines are the author's. Handing this a string to
   * split again would be one reading done twice.
   *
   * **A bare string is a line nothing is said about**, which is every line of
   * every turn but the opening brief. A caller holding the payload passes the
   * payload; a caller holding only text passes text and gets a brief with no
   * headings marked, which is what this drew before the marker existed.
   */
  lines: readonly (string | BriefLine)[];
};

/**
 * One line of the payload, as the log's own `LogLine` shapes it.
 *
 * **`named` is a string here and a closed set there.** `screens` owns that
 * vocabulary — the echoed command, the result, the trailer — and this reads
 * exactly one of its values. Restating the set would be a second copy of it in
 * the package that does not decide it, and narrowing `named` to `"heading"`
 * would refuse the payload every caller actually holds.
 *
 * **Every other value draws as body, deliberately.** `passed` and `failed` are
 * what a Check's run came to, and a brief has no outcomes in it — a component
 * that hued by this field would colour a block of instructions as a result.
 */
export type BriefLine = {
  text: string;
  named?: string;
};

export function DroneBrief({ lines }: DroneBriefProps) {
  const blocks = briefBlocks(lines);
  if (blocks.length === 0) return null;
  return (
    <div className="armada-brief">
      {blocks.map((block, at) =>
        block.heading ? (
          <h4 className="armada-brief__heading" key={at}>
            {block.text}
          </h4>
        ) : (
          <p className="armada-brief__block" key={at}>
            {widenIndent(block.text)}
          </p>
        ),
      )}
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

/** One block of a brief, and whether the wire named it as a heading. */
export type BriefBlock = { text: string; heading: boolean };

/**
 * The lines, grouped into blocks at the blank ones.
 *
 * **A run of blank lines is one boundary.** An empty block draws as an empty
 * element, which is a gap a reader cannot tell from a block that failed to
 * render.
 *
 * **A block is a heading when it is one line and that line is named one.** Every
 * heading `briefing.rs` writes has a blank line under it, so grouping already
 * leaves each one alone in its block and there is nothing to split. A named
 * line that turned up with body beside it stays body rather than dragging the
 * body into a heading — the marker says which line, and this says which block,
 * and neither guesses.
 *
 * Exported because it is arithmetic and is tested as arithmetic — in
 * `packages/screens`, in node, where a hundred briefs cost what one costs. A
 * `play` that computed this would be a unit test paying a browser's price.
 */
export function briefBlocks(lines: readonly (string | BriefLine)[]): BriefBlock[] {
  const blocks: BriefBlock[] = [];
  let held: BriefLine[] = [];
  const close = () => {
    if (held.length === 0) return;
    blocks.push({
      text: held.map((line) => line.text).join("\n"),
      heading: held.length === 1 && held[0].named === "heading",
    });
    held = [];
  };
  for (const line of lines) {
    const entry = typeof line === "string" ? { text: line } : line;
    if (entry.text.trim() === "") {
      close();
      continue;
    }
    held.push(entry);
  }
  close();
  return blocks;
}
