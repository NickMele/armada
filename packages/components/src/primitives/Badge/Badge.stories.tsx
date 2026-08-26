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
  OctagonAlert,
  Power,
  RefreshCw,
  ShieldX,
  Split,
  Stamp,
  Terminal,
  Unplug,
  UserCheck,
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
 * `not_started` and `escalated` have a status token and no glyph in
 * `packages/icons/icons.toml`. The contract makes an icon mandatory on every
 * state, so these two render short a channel — visibly ragged in a column,
 * which is the failure being reported rather than papered over.
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

export const Escalated: Story = {
  args: { status: "escalated", icon: NO_GLYPH_IN_REGISTRY, children: "Escalated" },
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
