import type { ReactNode } from "react";
import { useState } from "react";

import type { BlockKind } from "@armada/protocol";
import { Chapter } from "../Chapter/Chapter";
import { Clamped } from "../Clamped/Clamped";
import { FactChip } from "../FactChip/FactChip";
import { StepActivityMark, type StepActivity } from "../StepActivityMark/StepActivityMark";

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
 * **Since protocol 9.7, a heading also carries a `kind`**, and where every
 * heading has one this draws a section per kind rather than a flat run of
 * headings and paragraphs — a foldable "Where you are" instead of a rail
 * buried in the twentieth line of prose. **Where `kind` is absent from every
 * heading — a turn from a Fleet built before 9.7, or one with no headed
 * blocks at all — this draws exactly as it always has**, flat, because pairing
 * the first few kinds and guessing at the rest is worse than not pairing any.
 *
 * **Sectioned or flat, nothing the Drone was told disappears.** A folded
 * section keeps its body in the DOM; the `steps` and `checks` sections draw a
 * structured reading built from data Bridge already holds, but the words Fleet
 * actually sent stay reachable underneath it, one press away — never replaced,
 * only led with.
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
  /**
   * The Job's steps, done or still ahead of the one the Drone is on, for the
   * `steps` section's structured reading.
   *
   * **Absent draws the section's own text instead.** Fleet's rail is already
   * words a Drone can read; this is Bridge's own reading of the same fact, off
   * data a person already has open elsewhere on the screen, and a caller with
   * nothing to hand here has lost nothing by not handing it.
   */
  steps?: readonly BriefStep[];
  /**
   * This step's declared Checks, by name, for the `checks` section's
   * structured reading. Absent draws the section's own text instead, on the
   * same reasoning as `steps`.
   */
  checks?: readonly string[];
};

/**
 * One of the Job's steps, as the `steps` section reads it.
 *
 * **Three positions, because that is what Fleet's own rail ever says.** The
 * brief a Drone reads never carries a step's retries or its verdict — it says
 * a part is behind you, the one you are on, or ahead of you — so this reads
 * the same three off `current_step_id` rather than borrowing `StepActivity`'s
 * fuller vocabulary, which answers a monitoring question nobody is asking
 * here.
 */
export type BriefStep = {
  id: string;
  label: string;
  position: "done" | "current" | "not_yours";
};

/** `BriefStep.position` to the mark `StepActivityMark` already draws. */
const MARK_FOR: Record<BriefStep["position"], StepActivity> = {
  done: "advanced",
  current: "running",
  not_yours: "not_started",
};

/** `BriefStep.position` in words, for the mark's tooltip. */
const SAID_FOR: Record<BriefStep["position"], string> = {
  done: "done",
  current: "you are here",
  not_yours: "not yours",
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
 *
 * **`kind` is read only beside `named: "heading"`.** It is the wire's
 * `BlockKind`, paired by `story.ts`; a body line carries none, and the type
 * says so with an optional field rather than a second union.
 */
export type BriefLine = {
  text: string;
  named?: string;
  kind?: BlockKind;
};

export function DroneBrief({ lines, steps, checks }: DroneBriefProps) {
  const sections = briefSections(lines);
  if (sections === undefined) return <FlatBrief lines={lines} />;
  if (sections.length === 0) return null;
  return (
    <div className="armada-brief armada-brief--sectioned">
      {sections.map((section, at) => (
        <BriefSectionView key={at} ordinal={at + 1} section={section} steps={steps} checks={checks} />
      ))}
    </div>
  );
}

/** The brief exactly as it drew before protocol 9.7 — flat, no folding. */
function FlatBrief({ lines }: { lines: readonly (string | BriefLine)[] }) {
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

/** What a section's own blocks draw as, absent any structured reading. */
function BriefBody({ blocks }: { blocks: readonly BriefBlock[] }) {
  return (
    <>
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
    </>
  );
}

/**
 * The lines a heading's own words to be found where the mock's plain English
 * has replaced them. **Sentence case, per `docs/contracts/design-system.md`**
 * — Fleet's own headings are shouted (`JOB BRIEF`), which is content and stays
 * that way inside a section's body; a section's own title is Bridge's UI
 * chrome and the prose rules bind it like every other label on screen.
 *
 * **A second vocabulary, and the cost is named rather than hidden.** Fleet
 * decides which kind a heading is; this decides what a person reading Bridge
 * calls it. Two rows that decide the same word would be one place this can go
 * stale, so this map is the one place, and nothing downstream restates it.
 */
const SECTION_TITLE: Record<BlockKind, string> = {
  about_this_job: "About this job",
  standing: "Standing instructions",
  steps: "Where you are",
  checks: "What this part has to pass",
};

/** Whether a section opens by default. Every kind but `standing` does. */
const SECTION_OPEN: Record<BlockKind, boolean> = {
  about_this_job: true,
  standing: false,
  steps: true,
  checks: true,
};

/** How many lines `about_this_job`'s own prose holds before it clamps.
 *  The rest of the brief is bounded by its own fold now; this is the one
 *  section whose length is a person's or a model's, at whatever it wrote. */
const ABOUT_THIS_JOB_LINES = 8;

function BriefSectionView({
  ordinal,
  section,
  steps,
  checks,
}: {
  ordinal: number;
  section: BriefSection;
  steps: readonly BriefStep[] | undefined;
  checks: readonly string[] | undefined;
}) {
  const kind = section.kind ?? "standing";
  const [open, setOpen] = useState(SECTION_OPEN[kind]);
  return (
    <Chapter
      ordinal={ordinal}
      name={SECTION_TITLE[kind]}
      meta={metaFor(kind, steps, checks)}
      tone={kind === "standing" ? "muted" : "neutral"}
      open={open}
      onToggle={() => setOpen((was) => !was)}
    >
      {kind === "steps" && steps !== undefined ? (
        <StepsRead steps={steps} raw={section} />
      ) : kind === "checks" && checks !== undefined ? (
        <ChecksRead checks={checks} raw={section} />
      ) : kind === "about_this_job" ? (
        <Clamped lines={ABOUT_THIS_JOB_LINES}>
          <BriefBody blocks={sectionBlocks(section)} />
        </Clamped>
      ) : (
        <BriefBody blocks={sectionBlocks(section)} />
      )}
    </Chapter>
  );
}

/** The header's trailing fact, where Bridge already holds one for this kind. */
function metaFor(
  kind: BlockKind,
  steps: readonly BriefStep[] | undefined,
  checks: readonly string[] | undefined,
): string | undefined {
  if (kind === "steps" && steps !== undefined) {
    const at = steps.findIndex((step) => step.position === "current");
    return at === -1 ? undefined : `part ${at + 1} of ${steps.length}`;
  }
  if (kind === "checks" && checks !== undefined) {
    return checks.length === 1 ? "1 check" : `${checks.length} checks`;
  }
  // `about_this_job` and `standing` carry no field Bridge holds independently
  // of the words themselves — the section's name is what there is to say
  // about it collapsed.
  return undefined;
}

function StepsRead({ steps, raw }: { steps: readonly BriefStep[]; raw: BriefSection }) {
  return (
    <>
      <ol className="armada-brief__steps">
        {steps.map((step, at) => (
          <li key={step.id} className="armada-brief__step">
            <StepActivityMark
              activity={MARK_FOR[step.position]}
              ordinal={at + 1}
              label={step.label}
              says={`${step.label}, ${SAID_FOR[step.position]}`}
            />
          </li>
        ))}
      </ol>
      <RawWords raw={raw} />
    </>
  );
}

function ChecksRead({ checks, raw }: { checks: readonly string[]; raw: BriefSection }) {
  return (
    <>
      <div className="armada-brief__checks">
        {checks.map((check, at) => (
          <FactChip key={`${check}-${at}`}>{check}</FactChip>
        ))}
      </div>
      <RawWords raw={raw} />
    </>
  );
}

/**
 * The section's own words, still reachable beneath a structured reading.
 *
 * **Nothing the Drone was told may disappear.** `steps` and `checks` lead with
 * a reading built from data Bridge already has, but what Fleet actually sent
 * is what the Drone read, and it stays in the DOM behind one more press rather
 * than being replaced by Bridge's own rendering of the same fact.
 */
function RawWords({ raw }: { raw: BriefSection }): ReactNode {
  const [open, setOpen] = useState(false);
  const blocks = sectionBlocks(raw);
  if (blocks.length === 0) return null;
  return (
    <div className="armada-brief__raw">
      <button
        type="button"
        className="armada-brief__raw-toggle"
        aria-expanded={open}
        onClick={() => setOpen((was) => !was)}
      >
        {open ? "Hide the words Armada sent" : "Read the words Armada sent"}
      </button>
      {/* `hidden`, not unmounted — on `Chapter`'s own rule: what the Drone was
          told stays in the DOM whether or not this disclosure is open. */}
      <div className="armada-brief__raw-body" hidden={!open}>
        <BriefBody blocks={blocks} />
      </div>
    </div>
  );
}

/**
 * A section's own blocks, with its heading restored to the front of them.
 *
 * **The one place `heading` and `blocks` are read back together.** They are
 * split on `BriefSection` because a section's title on screen is
 * `SECTION_TITLE`, not Fleet's own words — but Fleet's words are still owed
 * somewhere, and this is where every caller that draws a section's raw
 * content puts them back at the top of it, exactly where they sat on the
 * wire.
 */
function sectionBlocks(section: BriefSection): BriefBlock[] {
  return section.heading === undefined
    ? [...section.blocks]
    : [{ text: section.heading, heading: true }, ...section.blocks];
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
      heading: held.length === 1 && held[0]?.named === "heading",
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

/** One section of a brief: a heading's own kind, its own words, and its body. */
export type BriefSection = {
  /** `undefined` is the unheaded opening, read as `standing` by position. */
  kind: BlockKind | undefined;
  /** The heading's own text, exactly as Fleet wrote it. `undefined` for the opening. */
  heading: string | undefined;
  /** The body blocks between this heading and the next, or before the first. */
  blocks: readonly BriefBlock[];
};

/**
 * The payload as sections, grouped by heading, or `undefined` where the wire
 * gave no kind to pair with — a turn from a Fleet built before protocol 9.7,
 * or one with no headed blocks at all. **The caller draws flat in that case**,
 * which is `DroneBrief`'s own fallback, not a decision made twice.
 *
 * **Headings that share a kind share a section.** `about_this_job` covers
 * several separate facts Fleet may write as separate blocks — what was asked,
 * what a person said, what the branch looked like — and none of them is
 * information this component has to tell apart on its own; they fold and
 * unfold together, each keeping its own heading inside the one section, rather
 * than this inventing a title per occurrence for words it cannot tell apart.
 */
export function briefSections(lines: readonly (string | BriefLine)[]): BriefSection[] | undefined {
  const blocks = briefBlocks(lines);
  const rawLines = lines.map((line) => (typeof line === "string" ? { text: line } : line));
  const headingKinds = rawLines.filter((line) => line.named === "heading").map((line) => line.kind);
  if (headingKinds.length === 0 || headingKinds.some((kind) => kind === undefined)) return undefined;

  const byKind = new Map<BlockKind, BriefSection>();
  const order: BriefSection[] = [];
  let headingAt = 0;
  // The unheaded opening, held here until it is known to carry anything. **Not
  // inserted into `order` up front** — a document with no opening prose and a
  // `standing` heading partway through must place that section where its
  // heading actually sits, not at the top reserved for prose that never came.
  let current: BriefSection = { kind: "standing", heading: undefined, blocks: [] };
  let openingSettled = false;

  for (const block of blocks) {
    if (block.heading) {
      if (!openingSettled) {
        if (current.blocks.length > 0) {
          byKind.set("standing", current);
          order.push(current);
        }
        openingSettled = true;
      }
      const kind = headingKinds[headingAt];
      headingAt += 1;
      if (kind === undefined) continue; // unreachable given the guard above
      const existing = byKind.get(kind);
      if (existing === undefined) {
        current = { kind, heading: block.text, blocks: [] };
        byKind.set(kind, current);
        order.push(current);
      } else {
        // A second heading of a kind already open joins it as another line of
        // the same section, keeping its own words rather than losing them.
        current = existing;
        (current.blocks as BriefBlock[]).push({ text: block.text, heading: true });
      }
    } else {
      (current.blocks as BriefBlock[]).push(block);
    }
  }
  return order;
}
