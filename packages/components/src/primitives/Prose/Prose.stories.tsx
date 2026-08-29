import type { Meta, StoryObj } from "@storybook/react-vite";
import { Prose } from "./Prose";

const meta: Meta<typeof Prose> = {
  title: "Primitives/Prose",
  component: Prose,
};
export default meta;

type Story = StoryObj<typeof Prose>;

/**
 * The wall this exists for. A Judge's `consequence` arrives as several
 * paragraphs with a path and an expression in it, and rendered as one string
 * it is the block the override dialog was reported for.
 *
 * Read what is **not** here: no heading size, no link colour, no syntax
 * highlighting. Every treatment on the page is one the token set already
 * carried.
 */
export const AJudgesConsequence: Story = {
  args: {
    text:
      "The route table is walked once per operation and the loop skips `forget_job`, so the " +
      "assertion that every operation is served passes without ever reading the one operation " +
      "the step was about.\n\n" +
      "The skip is in `crates/api/src/tests/served.rs` and reads:\n\n" +
      "```\nif route.operation == \"forget_job\" { continue; }\n```\n\n" +
      "Nothing else in the file narrows the set, so the count the assertion compares against " +
      "was lowered by the same edit that made it pass.",
  },
};

/**
 * A citation on a gaming flag. **One string on the wire and three things to
 * read** — what the check found, where, and the line it found it on. Inline
 * code stays inline; the fenced block wraps rather than clipping, because the
 * failing render broke an expression mid-token.
 */
export const AFlagsCitation: Story = {
  args: {
    text:
      "`crates/api/src/tests/served.rs:214` — the `served_every_operation` assertion counts " +
      "`ROUTES.len()` after the filter rather than before it:\n\n" +
      "```\nlet routes: Vec<&Route> = ROUTES.iter().filter(|r| r.operation != \"forget_job\").collect();\n" +
      "assert_eq!(routes.len(), served.len());\n```",
  },
};

/**
 * The structure a model writes when it is listing findings. A list is rows and
 * carries no marker glyph — a bullet is decorative iconography, and the indent
 * already says what it would have.
 *
 * The `#` line renders at `--weight-medium` and full contrast. **Not at
 * `--text-lg`**: panel headings own that step, and a renderer that took it
 * would put a second type scale inside the first.
 */
export const AListAndAHeading: Story = {
  args: {
    text:
      "# What the check read\n\n" +
      "Three things, and the third is the one that matters:\n\n" +
      "- the diff touches `armada.yml`\n" +
      "- the touched key is `checks.tests.command`\n" +
      "- the command it was changed **to** exits 0 on an empty test set\n\n" +
      "The first two alone are ordinary. Together with the third they are the pattern.",
  },
};

/**
 * The refusals, drawn so they can be seen. A link, an image, a table and a
 * blockquote render as the characters they are.
 *
 * **Refused rather than unimplemented.** Bridge's CSP reaches `'self'` and
 * nothing else and this text arrives from a model over the wire, so a link
 * that renders is a link that can be clicked. A table and a blockquote would
 * each need a treatment the contract does not draw.
 */
export const WhatItWillNotDraw: Story = {
  args: {
    text:
      "A link is written [like this](https://example.invalid/x) and stays written that way.\n\n" +
      "> A blockquote is a paragraph that opens with a caret.\n\n" +
      "| so | is | a table |",
  },
};
