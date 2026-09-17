import type { Meta, StoryObj } from "@storybook/react-vite";
import { Eye } from "lucide-react";
import { Badge } from "../Badge/Badge";
import { Sheet } from "../Sheet/Sheet";
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "./Card";

/**
 * The contract names two renderings of a card: its resting state, and a
 * de-emphasised one. There is no card hover, no card selection and no card
 * focus anywhere in the sources.
 *
 * **Every story here but the last two is a card on the canvas**, which is the
 * card treatment under Depth: the gradient, the edge, the top highlight, the
 * blur and `--shadow-card`. The last two are the same component off the
 * canvas — inside a card and inside a sheet — where it is flat `--bg-raised`.
 * Nothing in this file asks for either; `glass.css` reads the ancestry.
 *
 * Width is a measure in `ch` rather than a token: no card width token exists,
 * and the sheet draws a different pixel width on every card it holds.
 */
const meta: Meta<typeof Card> = {
  title: "Primitives/Card",
  component: Card,
  decorators: [
    (Story) => (
      <div style={{ maxWidth: "56ch" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof Card>;

export const Default: Story = {
  render: () => (
    <Card>
      <CardTitle>Evidence accepted at step 4 of 5</CardTitle>
      <CardDescription>Four criteria resolved. One test added, none removed.</CardDescription>
    </Card>
  ),
};

/** A status badge is the one hue a card carries, and it sits in the header. */
export const WithHeader: Story = {
  render: () => (
    <Card>
      <CardHeader>
        <span className="caps">Evidence</span>
        <Badge status="awaiting-review" icon={Eye}>
          Awaiting review
        </Badge>
      </CardHeader>
      <CardTitle>Evidence accepted at step 4 of 5</CardTitle>
      <CardDescription>Four criteria resolved. One test added, none removed.</CardDescription>
      <CardContent>
        <span style={{ color: "var(--fg-muted)", fontSize: "var(--text-xs)" }}>
          Verification source <span className="mono">Judge</span>
        </span>
      </CardContent>
      <CardFooter>
        <span style={{ color: "var(--fg-subtle)", fontSize: "var(--text-2xs)" }}>
          Read at <span className="mono">14:22</span>
        </span>
      </CardFooter>
    </Card>
  ),
};

/**
 * Dimming is a token, not an alpha: the edge steps to `--border-subtle` and
 * the text to `--fg-subtle`. `opacity` would muddy any status colour inside.
 */
export const Dimmed: Story = {
  render: () => (
    <Card data-dimmed>
      <CardTitle>Superseded at step 2 of 5</CardTitle>
      <CardDescription>The work landed outside this job.</CardDescription>
    </Card>
  ),
};

/**
 * A card inside a card is flat. Glass on glass reads as a seam, and nothing
 * inside a card takes a shadow — so the inner one is `--bg-raised` with a
 * `--border-default` edge, and the outer one carries the treatment alone.
 */
export const InsideACard: Story = {
  render: () => (
    <Card>
      <CardTitle>Cleanup</CardTitle>
      <CardContent>
        <Card>
          <CardTitle>bridge/1353-cards-on-glass</CardTitle>
          <CardDescription>Held for a branch the base does not reach yet.</CardDescription>
        </Card>
      </CardContent>
    </Card>
  ),
};

/**
 * A card inside a sheet is flat, for the same reason Drift and Verify are
 * (#1260): a sheet is not the canvas, so nothing in it sits on the canvas.
 * The blur goes too — `glass.css` turns it off for every card under a scrim,
 * which is where the presses that went missing came from.
 */
export const InsideASheet: Story = {
  render: () => (
    <Sheet open side="right" title="Manifest">
      <Card>
        <CardTitle>Drift</CardTitle>
        <CardDescription>Three entries name a path that no longer exists.</CardDescription>
      </Card>
    </Sheet>
  ),
};
