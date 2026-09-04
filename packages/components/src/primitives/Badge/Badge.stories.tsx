import type { Meta, StoryObj } from "@storybook/react-vite";
import type { LucideIcon } from "lucide-react";
import {
  Archive,
  ArrowUpToLine,
  Ban,
  Check,
  CircleDot,
  Clock,
  Cpu,
  Eye,
  FileQuestionMark,
  Link,
  Megaphone,
  OctagonAlert,
  Power,
  RefreshCw,
  ShieldX,
  Split,
  Stamp,
  Terminal,
  Unplug,
  UserCheck,
  Wrench,
  X,
} from "lucide-react";
import { Badge } from "./Badge";

/**
 * One story per status `packages/tokens/src/status.css` declares, because that
 * file is the roster and hue below Job level exists only where it says so.
 * Labels are sentence case and would come from the enum→verb map; that map is
 * not generated yet, so they are written here and nowhere that ships.
 */
const meta: Meta<typeof Badge> = {
  title: "Primitives/Badge",
  component: Badge,
};
export default meta;

type Story = StoryObj<typeof Badge>;

/**
 * `not_started` has a status token and no glyph in `packages/icons/icons.toml`.
 * The contract makes an icon mandatory on every state, so it renders short a
 * channel — visibly ragged in a column, which is the failure being reported
 * rather than papered over.
 *
 * **`escalated` was the second one and is not any more.** It draws `megaphone`
 * in `needs you` since #400: it is a status a surface can be handed with no
 * escalation reason beside it, and a status that renders only its reason had
 * nothing to draw there.
 */
const NO_GLYPH_IN_REGISTRY = undefined as unknown as LucideIcon;

export const NotStarted: Story = {
  args: { status: "not-started", icon: NO_GLYPH_IN_REGISTRY, children: "Not started" },
};

export const Queued: Story = {
  args: { status: "not-started", icon: Clock, children: "Queued" },
};

/**
 * `queued` renders grey whatever its reason, and the reason's glyph replaces
 * `clock`. A Job out of headroom reads "waiting on resources" rather than
 * falling through unrendered.
 */
export const QueuedOutOfHeadroom: Story = {
  args: { status: "not-started", icon: Cpu, children: "Waiting on resources" },
};

export const QueuedBlockedByDependency: Story = {
  args: { status: "not-started", icon: Link, children: "Blocked by a dependency" },
};

export const AwaitingApproval: Story = {
  args: { status: "awaiting-approval", icon: UserCheck, children: "Awaiting approval" },
};

export const Running: Story = {
  args: { status: "running", icon: CircleDot, children: "Running" },
};

/**
 * The running mark on the focused row of a list: the inner dot of `circle-dot`
 * moves on opacity and scale at `--duration-pulse`, and the ring holds still.
 * One per screen, and never where a rail carries a more specific mark.
 */
export const RunningPulsing: Story = {
  args: { status: "running", icon: CircleDot, children: "Running", pulsing: true },
};

export const Piloted: Story = {
  args: { status: "piloted", icon: Terminal, children: "Piloted" },
};

export const AwaitingReview: Story = {
  args: { status: "awaiting-review", icon: Eye, children: "Awaiting review" },
};

export const AwaitingAttestation: Story = {
  args: { status: "awaiting-attestation", icon: Stamp, children: "Awaiting attestation" },
};

/**
 * Drawn immediately above `escalated`, which is where the pair can be read
 * against each other: `status.css` aliases this hue from that one, so the only
 * channel telling the two apart is the glyph, and a spanner beside a horn is
 * the whole of Iconography rule 4 working or not working.
 */
export const AwaitingRepair: Story = {
  args: { status: "awaiting-repair", icon: Wrench, children: "Needs repair" },
};

/**
 * The status's own badge, which stands where no escalation reason is set — the
 * reason's verb and glyph replace it the moment one is, the same handover
 * `queued` has above.
 */
export const Escalated: Story = {
  args: { status: "escalated", icon: Megaphone, children: "Needs you" },
};

export const CompletedSuccess: Story = {
  args: { status: "completed-success", icon: Check, children: "Completed" },
};

export const CompletedFailed: Story = {
  args: { status: "completed-failed", icon: X, children: "Failed" },
};

export const Rejected: Story = {
  args: { status: "rejected", icon: Ban, children: "Rejected" },
};

export const Killed: Story = {
  args: { status: "killed", icon: Power, children: "Killed" },
};

export const Superseded: Story = {
  args: { status: "superseded", icon: Archive, children: "Superseded" },
};

/**
 * The escalation reasons share one hue and differentiate by label and icon —
 * a column of oranges would be unreadable. Drawn together because the rule is
 * about the set, not about any one of them: categorically different outlines,
 * none depending on interior detail surviving 12px.
 *
 * Three labels are the contract's own — stalled, churning, evidence disputed.
 * The other four have no sanctioned copy anywhere in the repository and are
 * written from the registry's own wording for the glyph. Reported.
 */
export const EscalationReasons: StoryObj = {
  render: () => (
    <div style={{ display: "flex", flexDirection: "column", alignItems: "flex-start", gap: "var(--space-2)" }}>
      <Badge status="escalated" icon={OctagonAlert}>
        Stalled
      </Badge>
      <Badge status="escalated" icon={RefreshCw}>
        Churning
      </Badge>
      <Badge status="escalated" icon={FileQuestionMark}>
        Evidence disputed
      </Badge>
      <Badge status="escalated" icon={ShieldX}>
        Check failed
      </Badge>
      <Badge status="escalated" icon={Split}>
        Fanned out
      </Badge>
      <Badge status="escalated" icon={Unplug}>
        Connection lost
      </Badge>
      <Badge status="escalated" icon={ArrowUpToLine}>
        Reached its ceiling
      </Badge>
    </div>
  ),
};
