/**
 * A tool's name, in the colour of what the call does.
 *
 * A long stretch of looking, then a burst of changing, then a run: that is the
 * shape of a Drone's hour, and in a mono column of one colour it is invisible.
 *
 * **A family is never a status**, and its hues are not status hues —
 * `packages/tokens/src/tools.css` carries why they are declared apart.
 */

/** What a call does. The roster below is what decides which. */
export type ToolFamily = "looking" | "changing" | "running";

/**
 * Tool to family, on the harness's own spellings.
 *
 * The line is `scouting.rs`'s: `Read`, `Grep` and `Glob` are what a scout is
 * given, and anything that edits, writes or runs is on the other side of it.
 * `Bash` is alone in `running` because it is the one tool whose call makes the
 * machine do something, which is what a person scans for.
 */
const FAMILY: Record<string, ToolFamily> = {
  Read: "looking",
  Grep: "looking",
  Glob: "looking",
  Edit: "changing",
  MultiEdit: "changing",
  Write: "changing",
  NotebookEdit: "changing",
  Bash: "running",
};

/**
 * What a call does, or nothing for a tool the roster has no reading for — hue
 * is scarce, and an unclassified name draws neutral rather than inventing a
 * fourth family.
 */
export function toolFamily(tool: string): ToolFamily | undefined {
  return FAMILY[tool];
}

export type ToolNameProps = {
  /** The tool as the wire spells it — `Edit`, `Bash`. */
  tool: string;
};

/** The tool's name, hued by its family. No glyph and no chip. */
export function ToolName({ tool }: ToolNameProps) {
  return (
    <span className="armada-tool" data-family={toolFamily(tool)}>
      {tool}
    </span>
  );
}
