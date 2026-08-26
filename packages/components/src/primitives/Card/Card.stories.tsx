import type { Meta, StoryObj } from "@storybook/react-vite";
import { Eye } from "lucide-react";
import { Badge } from "../Badge/Badge";
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "./Card";

/**
 * The contract names two renderings of a card: its resting state, and a
 * de-emphasised one. There is no card hover, no card selection and no card
 * focus anywhere in the sources, and no shadow at any point — elevation is
 * surface.
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
