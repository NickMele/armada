import type { Meta, StoryObj } from "@storybook/react-vite";
import type { LucideIcon } from "lucide-react";
import { ShieldMinus } from "lucide-react";
import { Button } from "../../primitives/Button/Button";
import { JobLogReference } from "../../compositions/JobLogReference/JobLogReference";
import { WorkflowRail, type WorkflowRailStep } from "../../compositions/WorkflowRail/WorkflowRail";
import { Absent } from "../absent";

/**
 * Four steps, two of them ungated. No primary action exists on this screen.
 *
 * **The header is `Job detail header actions`, and it is not built.** The
 * drawing names the whole header block — badge, title, job id, the five facts
 * and Kill — as one `data-component`, so the region is named rather than
 * assembled out of a Badge and a Button on the spot.
 *
 * **`Evidence card` is not built either.** The evidence trail on a finished job
 * is; a single submission rendered while the job is still running is its own
 * row in the registry and has no story.
 */
const meta: Meta = {
  title: "Screens/A running job",
};
export default meta;

type Story = StoryObj;

/* The evidence row on an ungated step wants the page-with-a-check outline, and
   `file-check` has no entry in `packages/icons/icons.toml`. The row renders a
   channel short rather than reaching for an unregistered glyph. Reported. */
const NO_GLYPH_IN_REGISTRY = undefined as unknown as LucideIcon;

const steps: WorkflowRailStep[] = [
  {
    id: "plan",
    label: "Plan the change",
    activity: "advanced",
    status: "advanced",
    evidence: { icon: NO_GLYPH_IN_REGISTRY, iconLabel: "Evidence", label: "evidence · 09:14" },
  },
  {
    id: "implement",
    label: "Implement",
    activity: "running",
    status: "running · 6m 12s",
    current: true,
    gates: [
      {
        command: "build · cargo build --workspace",
        result: "not reached",
        icon: ShieldMinus,
        iconLabel: "Not reached",
      },
      {
        command: "diff_nonempty",
        result: "not reached",
        icon: ShieldMinus,
        iconLabel: "Not reached",
      },
    ],
  },
  {
    id: "verify",
    label: "Run tests",
    activity: "not_started",
    status: "not started",
    gates: [
      {
        command: "test · cargo test --workspace",
        result: "not reached",
        icon: ShieldMinus,
        iconLabel: "Not reached",
      },
    ],
  },
  {
    id: "handoff",
    label: "Summarise",
    activity: "not_started",
    status: "not started",
    // The drawing draws no row under Summarise. The rail always draws one,
    // because a blank would read as a gate that failed to render. Reported.
    evidence: { icon: NO_GLYPH_IN_REGISTRY, iconLabel: "Evidence", label: "" },
  },
];

export const RunningJob: Story = {
  render: () => (
    <div className="armada-screen">
      <div className="armada-screen__detail">
        <Absent
          name="Job detail header actions"
          note={
            "Holds the Running badge, static, beside the title “Split the settings " +
            "reducer” and job_2d90bb; then Step 2 of 4 · Branch fix/settings-split · " +
            "Elapsed 11m 03s · Spend, estimated ~$1.80 · Dispatched by you; and Kill at " +
            "the trailing edge, outlined in --status-completed-failed and never filled."
          }
        />

        <div className="armada-screen__split">
          <div className="armada-screen__col">
            <span className="armada-screen__eyebrow">What ran</span>
            <WorkflowRail steps={steps} pulsing />
          </div>

          <div className="armada-screen__col">
            <span className="armada-screen__eyebrow">Evidence so far</span>
            <div className="armada-screen__slot">
              <Absent
                name="Evidence card"
                note={
                  "Holds one submission — Plan the change, 09:14 — on the three fields the " +
                  "Evidence MCP tool requires: Claimed, Shown by, Not claimed. Plan the " +
                  "change is facts_note, so shown_by points at files rather than a command."
                }
              />
            </div>
            <span className="armada-screen__eyebrow" data-spaced>
              Log
            </span>
            <JobLogReference
              rows={[
                {
                  icon: NO_GLYPH_IN_REGISTRY,
                  iconLabel: "Log",
                  value: ".armada/logs/job_2d90bb.jsonl",
                  copyValue: ".armada/logs/job_2d90bb.jsonl",
                  meta: "142 lines · 0 error",
                },
              ]}
              actions={
                <Button ground="sunken" size="sm">
                  Open the log
                </Button>
              }
            >
              Fleet, the drone and Bridge in one order, keyed on this job. It is being written
              now.
            </JobLogReference>
          </div>
        </div>
      </div>

    </div>
  ),
};
