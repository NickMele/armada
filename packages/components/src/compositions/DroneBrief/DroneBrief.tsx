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
 * heading has one this draws one foldable section per heading, titled with
 * the heading's own words, rather than a flat run of headings and paragraphs.
 * **Where `kind` is absent from every heading — a turn from a Fleet built
 * before 9.7, or one with no headed blocks at all — this draws exactly as it
 * always has**, flat, because pairing the first few kinds and guessing at the
 * rest is worse than not pairing any.
 *
 * **The kind decides three things, and never the title.** Whether a section
 * opens or folds by default; whether `steps` or `checks` draws Bridge's own
 * reading of data it already holds, in place of the section's raw text; and
 * whether the section reads as `standing` — the same on every Job — which
 * folds shut and says so in its meta. The title is always the heading's own
 * words, sentence-cased. Two vocabularies for one heading is the cost a
 * `kind → label` map would have added for no fact the heading's own text does
 * not already carry; `sentenceCase` below is a text transform, not a second
 * naming of the thing.
 *
 * **Sectioned or flat, nothing the Drone was told disappears.** A folded
 * section keeps its body in the DOM; the `steps` and `checks` sections draw a
 * structured reading built from data Bridge already holds, but the words Fleet
 * actually sent stay reachable underneath it, folded by default — never
 * replaced, only led with.
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

/** `BriefStep.position`, in the words the mock draws beside a step's name. */
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
        <BriefSectionView key={at} section={section} steps={steps} checks={checks} />
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

/** A section's own body blocks, drawn as body — never its heading, which is
 *  the section's title now and would otherwise read twice. */
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
 * A shouted heading, in sentence case — `JOB BRIEF` reads `Job brief`.
 *
 * **A text transform, not a lookup.** A heading already mixed case, like
 * `STEP: Plan the change`, is left exactly as Fleet wrote it: the design
 * system bars ALL CAPS as UI chrome, and a heading that is not shouting in
 * the first place is not the case this transform exists for. Testing against
 * the string's own upper-cased form, rather than assuming every heading is
 * shouted, is what tells the two apart without a second vocabulary deciding
 * which headings count.
 */
export function sentenceCase(heading: string): string {
  if (heading !== heading.toUpperCase()) return heading;
  const lower = heading.toLowerCase();
  return lower.length === 0 ? lower : lower.charAt(0).toUpperCase() + lower.slice(1);
}

/** The one title used where a section has no heading of its own to draw —
 *  the unheaded opening, read as `standing` by position. Not a `kind → label`
 *  map: this is the single case with no words of its own to sentence-case. */
const UNHEADED_OPENING_TITLE = "Standing instructions";

/** What a folded `standing` section says collapsed — the fact Bridge holds
 *  about every section of this kind, regardless of what its words are. */
const STANDING_META = "the same on every Job";

/** Whether a section opens by default. Every kind but `standing` does. */
const SECTION_OPEN: Record<BlockKind, boolean> = {
  about_this_job: true,
  standing: false,
  steps: true,
  checks: true,
};

/** How many lines a heading's own prose holds before it clamps, on the one
 *  kind left unbounded by the fold — a person's or a model's words, at
 *  whatever length either wrote them. */
const ABOUT_THIS_JOB_LINES = 8;

function BriefSectionView({
  section,
  steps,
  checks,
}: {
  section: BriefSection;
  steps: readonly BriefStep[] | undefined;
  checks: readonly string[] | undefined;
}) {
  const kind = section.kind ?? "standing";
  const [open, setOpen] = useState(SECTION_OPEN[kind]);
  const title = section.heading === undefined ? UNHEADED_OPENING_TITLE : sentenceCase(section.heading);
  return (
    <Chapter
      name={title}
      meta={metaFor(kind, steps, checks)}
      tone={kind === "standing" ? "muted" : "neutral"}
      open={open}
      onToggle={() => setOpen((was) => !was)}
    >
      {kind === "steps" && steps !== undefined ? (
        <StepsRead steps={steps} raw={section.blocks} />
      ) : kind === "checks" && checks !== undefined ? (
        <ChecksRead checks={checks} raw={section.blocks} />
      ) : kind === "about_this_job" ? (
        <Clamped lines={ABOUT_THIS_JOB_LINES}>
          <BriefBody blocks={section.blocks} />
        </Clamped>
      ) : (
        <BriefBody blocks={section.blocks} />
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
  if (kind === "standing") return STANDING_META;
  if (kind === "steps" && steps !== undefined) {
    const at = steps.findIndex((step) => step.position === "current");
    return at === -1 ? undefined : `part ${at + 1} of ${steps.length}`;
  }
  if (kind === "checks" && checks !== undefined) {
    return checks.length === 1 ? "1 check" : `${checks.length} checks`;
  }
  // `about_this_job` carries no field Bridge holds independently of its own
  // words — the section's title is what there is to say about it collapsed.
  return undefined;
}

function StepsRead({ steps, raw }: { steps: readonly BriefStep[]; raw: readonly BriefBlock[] }) {
  return (
    <>
      <ol className="armada-brief__steps">
        {steps.map((step, at) => (
          <li className="armada-brief__step" key={step.id}>
            {/* The mark's own accessible name is the state, matching
                `WorkflowRail`'s row exactly — the step's name is drawn
                visibly beside it and would otherwise be announced twice. */}
            <StepActivityMark
              activity={MARK_FOR[step.position]}
              ordinal={at + 1}
              label={SAID_FOR[step.position]}
              says={`${step.label}, ${SAID_FOR[step.position]}`}
            />
            <span className="armada-brief__step-name">{step.label}</span>
            <span className="armada-brief__step-said" aria-hidden>
              {SAID_FOR[step.position]}
            </span>
          </li>
        ))}
      </ol>
      <RawWords blocks={raw} />
    </>
  );
}

function ChecksRead({ checks, raw }: { checks: readonly string[]; raw: readonly BriefBlock[] }) {
  return (
    <>
      <div className="armada-brief__checks">
        {checks.map((check, at) => (
          <FactChip key={`${check}-${at}`}>{check}</FactChip>
        ))}
      </div>
      <RawWords blocks={raw} />
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
 *
 * **Folded by default.** The structured reading is the one this section leads
 * with; showing both at once by default reads as the same fact said twice
 * rather than as one led by the other.
 */
function RawWords({ blocks }: { blocks: readonly BriefBlock[] }): ReactNode {
  const [open, setOpen] = useState(false);
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

/** One section of a brief: one heading, its own kind, and its own body. */
export type BriefSection = {
  /** `undefined` is the unheaded opening, read as `standing` by position. */
  kind: BlockKind | undefined;
  /** The heading's own text, exactly as Fleet wrote it. `undefined` for the
   *  opening, which has none — `UNHEADED_OPENING_TITLE` draws in its place. */
  heading: string | undefined;
  /** The body blocks under this heading and before the next one. */
  blocks: readonly BriefBlock[];
};

/**
 * The payload as sections, one per heading, or `undefined` where the wire
 * gave no kind to pair with — a turn from a Fleet built before protocol 9.7,
 * or one with no headed blocks at all. **The caller draws flat in that case**,
 * which is `DroneBrief`'s own fallback, not a decision made twice.
 *
 * **One section per heading, in the order they arrived.** Two headings that
 * share a kind — `about_this_job` covers several separate facts Fleet may
 * write as separate blocks, what was asked and what a person said among them
 * — draw as two sections rather than one merged under an invented title;
 * `kind` decides how each opens and what it draws, never how many sections
 * there are.
 */
export function briefSections(lines: readonly (string | BriefLine)[]): BriefSection[] | undefined {
  const blocks = briefBlocks(lines);
  const rawLines = lines.map((line) => (typeof line === "string" ? { text: line } : line));
  const headingKinds = rawLines.filter((line) => line.named === "heading").map((line) => line.kind);
  if (headingKinds.length === 0 || headingKinds.some((kind) => kind === undefined)) return undefined;

  const sections: BriefSection[] = [];
  const opening: BriefBlock[] = [];
  let current: BriefSection | undefined;
  let headingAt = 0;
  let openingSettled = false;

  for (const block of blocks) {
    if (block.heading) {
      if (!openingSettled) {
        if (opening.length > 0) sections.push({ kind: "standing", heading: undefined, blocks: opening });
        openingSettled = true;
      }
      const kind = headingKinds[headingAt];
      headingAt += 1;
      if (kind === undefined) continue; // unreachable given the guard above
      current = { kind, heading: block.text, blocks: [] };
      sections.push(current);
    } else if (current === undefined) {
      opening.push(block);
    } else {
      (current.blocks as BriefBlock[]).push(block);
    }
  }
  return sections;
}
