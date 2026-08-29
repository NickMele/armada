import { Fragment, type ReactNode } from "react";

/**
 * Text a model wrote, drawn as the structure it carries rather than as one
 * paragraph.
 *
 * **This is the half of "markdown or something" that markdown is the answer
 * to, and it is the smaller half.** Nearly everything Bridge shows is
 * assembled from typed values — a flag is a pattern and a citation, a verdict
 * is a criterion and a ruling — and those are drawn from their fields. What is
 * left is genuinely free text arriving as prose: a Judge's `produced` and
 * `consequence`, the citation on a gaming flag, a Drone's turn. Those are the
 * walls, and a renderer helps with those and with nothing else. Running this
 * over an assembled sentence would be parsing structure back out of prose that
 * had it thrown away.
 *
 * # The subset, and why it stops where it does
 *
 * **The scale is the surface's, not this component's.** Nothing here declares
 * a font size. A renderer that ships its own heading sizes, link colours and
 * code-block styling is a second design system inside the first, and
 * `docs/contracts/design-system.md` owns the type scale. So the constructs
 * that would need one are refused rather than approximated:
 *
 * | Written | Drawn |
 * |---|---|
 * | Paragraphs, blank-line separated | `<p>`, at the surrounding size |
 * | `` `inline code` `` | mono, in a `--bg-sunken` well |
 * | Fenced blocks | a mono block that wraps rather than clips — the failing render was an expression broken mid-token |
 * | `- ` and `* ` lists | one row per item, no marker glyph |
 * | `**bold**`, `*italic*` | weight and slant, both already tokens |
 * | `# heading` | the line at `--weight-medium` and full contrast — the structure without a second scale |
 * | Links, images, tables, blockquotes, raw HTML | the characters, literally |
 *
 * **Links are refused rather than unimplemented.** Bridge's CSP reaches
 * `'self'` and nothing else, an anchor needs a colour the accent is spoken for
 * by, and this text arrives from a model over the wire. A link that renders is
 * a link that can be clicked.
 *
 * **No dependency.** `packages/components` depends on React, lucide-react and
 * the token set. A markdown library brings a parser, a sanitiser and its own
 * default stylesheet, and the third is the thing this file exists to keep out.
 * The subset above is what the text actually contains.
 */
export type ProseProps = {
  /** The text as it arrived. Empty draws nothing rather than an empty block. */
  text: string;
};

export function Prose({ text }: ProseProps) {
  const blocks = read(text);
  if (blocks.length === 0) return null;
  return (
    <div className="armada-prose">
      {blocks.map((block, at) => (
        <Fragment key={at}>{drawn(block, at)}</Fragment>
      ))}
    </div>
  );
}

/** One block of the subset. A block is what a blank line, a fence or a bullet
 * separates; everything else is inline. */
type Block =
  | { kind: "paragraph"; lines: string[] }
  | { kind: "said"; line: string }
  | { kind: "code"; lines: string[] }
  | { kind: "list"; items: string[] };

/** A fence, opening or closing. The info string after it is ignored: this
 * renderer does not highlight, so the language names nothing it could use. */
const FENCE = "```";

/** What a bullet is written as. Both spellings, because a model uses both. */
const BULLETS = ["- ", "* "];

/**
 * The text, read into blocks.
 *
 * **A fence wins over everything inside it**, which is what stops a `#` or a
 * `-` in a diff from being read as a heading or a bullet. An unclosed fence
 * runs to the end rather than falling back to prose — a half-written code
 * block is still a code block, and re-reading it as paragraphs would reflow
 * the one thing whose line breaks are load-bearing.
 */
function read(text: string): Block[] {
  const blocks: Block[] = [];
  const lines = text.split("\n");
  let at = 0;
  while (at < lines.length) {
    const line = lines[at] ?? "";
    const trimmed = line.trim();
    if (trimmed.startsWith(FENCE)) {
      const held: string[] = [];
      at += 1;
      while (at < lines.length && !(lines[at] ?? "").trim().startsWith(FENCE)) {
        held.push(lines[at] ?? "");
        at += 1;
      }
      at += 1;
      blocks.push({ kind: "code", lines: held });
      continue;
    }
    if (trimmed === "") {
      at += 1;
      continue;
    }
    if (bulleted(trimmed)) {
      const items: string[] = [];
      while (at < lines.length && bulleted((lines[at] ?? "").trim())) {
        items.push((lines[at] ?? "").trim().slice(2));
        at += 1;
      }
      blocks.push({ kind: "list", items });
      continue;
    }
    if (heading(trimmed)) {
      blocks.push({ kind: "said", line: trimmed.replace(/^#+\s+/, "") });
      at += 1;
      continue;
    }
    const held: string[] = [];
    while (at < lines.length) {
      const next = (lines[at] ?? "").trim();
      if (next === "" || next.startsWith(FENCE) || bulleted(next) || heading(next)) break;
      held.push(next);
      at += 1;
    }
    blocks.push({ kind: "paragraph", lines: held });
  }
  return blocks;
}

function bulleted(line: string): boolean {
  return BULLETS.some((mark) => line.startsWith(mark));
}

function heading(line: string): boolean {
  return /^#{1,6}\s+/.test(line);
}

/** One block, in the treatment its kind already has on this surface. */
function drawn(block: Block, at: number): ReactNode {
  if (block.kind === "code") {
    return (
      <pre className="armada-prose__block">
        <code>{block.lines.join("\n")}</code>
      </pre>
    );
  }
  if (block.kind === "list") {
    return (
      <ul className="armada-prose__list">
        {block.items.map((item, i) => (
          <li className="armada-prose__item" key={i}>
            {inline(item, `${at}-${i}`)}
          </li>
        ))}
      </ul>
    );
  }
  if (block.kind === "said") {
    return <p className="armada-prose__said">{inline(block.line, `${at}`)}</p>;
  }
  // A soft line break inside a paragraph is a wrap the writer's editor made,
  // not one this surface has to keep — the width here is not the width there.
  return <p className="armada-prose__paragraph">{inline(block.lines.join(" "), `${at}`)}</p>;
}

/**
 * The inline pass. **Code first**, so a `*` inside a code span is a character
 * rather than emphasis — which is the whole reason a path or an expression is
 * backticked in the first place.
 */
function inline(text: string, key: string): ReactNode[] {
  const out: ReactNode[] = [];
  text.split(/(`[^`]+`)/g).forEach((part, i) => {
    if (part === "") return;
    if (part.length > 2 && part.startsWith("`") && part.endsWith("`")) {
      out.push(
        <code className="armada-prose__code" key={`${key}-c${i}`}>
          {part.slice(1, -1)}
        </code>,
      );
      return;
    }
    out.push(...emphasised(part, `${key}-${i}`));
  });
  return out;
}

/** Weight and slant, which the token set already carries. Nothing else. */
function emphasised(text: string, key: string): ReactNode[] {
  return text.split(/(\*\*[^*]+\*\*|\*[^*\s][^*]*\*|_[^_\s][^_]*_)/g).flatMap((part, i) => {
    if (part === "") return [];
    if (part.startsWith("**") && part.endsWith("**")) {
      return [
        <strong className="armada-prose__strong" key={`${key}-b${i}`}>
          {part.slice(2, -2)}
        </strong>,
      ];
    }
    if (
      (part.startsWith("*") && part.endsWith("*")) ||
      (part.startsWith("_") && part.endsWith("_"))
    ) {
      return [<em key={`${key}-i${i}`}>{part.slice(1, -1)}</em>];
    }
    return [<Fragment key={`${key}-t${i}`}>{part}</Fragment>];
  });
}
