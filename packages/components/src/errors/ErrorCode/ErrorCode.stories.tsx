import type { Meta, StoryObj } from "@storybook/react-vite";
import { X } from "lucide-react";

import { Badge } from "../../primitives/Badge/Badge";
import { ErrorCode } from "./ErrorCode";

const meta: Meta<typeof ErrorCode> = {
  title: "Errors/Error code",
  component: ErrorCode,
};
export default meta;

type Story = StoryObj<typeof ErrorCode>;

/**
 * A fault: Armada could not do the thing. Solid `--error`, which is
 * `--status-completed-failed` under an alias rather than a ninth hue.
 */
export const Fault: Story = {
  args: { kind: "fault", code: "fleet.approve.refused" },
};

/**
 * Degraded: Armada cannot refresh what it is showing. Still a solid chip,
 * because solid is what says error rather than status and that is true of
 * both classes — but neutral, because only a fault is red.
 */
export const Degraded: Story = {
  args: { kind: "degraded", code: "bridge.stream.dropped" },
};

/**
 * The comparison the treatment rests on, and the reason this component exists
 * at all. A failed Job is Armada working; an error is Armada failing. Both are
 * the same red, so the row reads left to right on shape alone: a 12% tint with
 * a glyph and a verb, then a solid fill with a code and no glyph.
 */
export const AgainstAStatusBadge: Story = {
  render: () => (
    <div style={{ display: "flex", alignItems: "center", gap: "var(--space-3)" }}>
      <Badge status="completed-failed" icon={X}>
        Failed
      </Badge>
      <ErrorCode kind="fault" code="fleet.approve.refused" />
    </div>
  ),
};
