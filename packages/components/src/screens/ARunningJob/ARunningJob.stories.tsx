import type { Meta, StoryObj } from "@storybook/react-vite";
import type { LucideIcon } from "lucide-react";
import { CircleDot, ShieldMinus } from "lucide-react";
import { Button } from "../../primitives/Button/Button";
import { EvidenceCard } from "../../compositions/EvidenceCard/EvidenceCard";
import { JobDetailHeaderActions } from "../../compositions/JobDetailHeaderActions/JobDetailHeaderActions";
import { JobLogReference } from "../../compositions/JobLogReference/JobLogReference";
import { WorkflowRail, type WorkflowRailStep } from "../../compositions/WorkflowRail/WorkflowRail";

/**
 * Four steps, two of them ungated. No primary action exists on this screen.
 *
 * **The pulse is on the rail, so the header badge is static.** One pulse per
 * screen, on the most specific mark present: the rail knows which step is
 * working and the badge only knows the Job is.
 *
 * The header verb comes from the enum→verb map. It is written into the fixture
 * because that map is not generated into TypeScript yet.
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
        <JobDetailHeaderActions
          status="running"
          statusIcon={CircleDot}
          statusLabel="Running"
          headline="Split the settings reducer"
          jobId="job_2d90bb"
          fields={[
            { label: "Step", value: "2 of 4", mono: true },
            {
              label: "Branch",
              value: "fix/settings-split",
              mono: true,
              copyValue: "fix/settings-split",
            },
            { label: "Elapsed", value: "11m 03s", mono: true },
            { label: "Spend, estimated", value: "~$1.80", mono: true },
            { label: "Dispatched by you" },
          ]}
          actions={<Button variant="destructive">Kill</Button>}
        />

        <div className="armada-screen__split">
          <div className="armada-screen__col">
            <span className="armada-screen__eyebrow">What ran</span>
            <WorkflowRail steps={steps} pulsing />
          </div>

          <div className="armada-screen__col">
            <span className="armada-screen__eyebrow">Evidence so far</span>
            <EvidenceCard
              icon={NO_GLYPH_IN_REGISTRY}
              iconLabel="Evidence"
              step="Plan the change"
              time="09:14"
              claimed="settings.rs is split into a reducer and a selector module, with no change in behaviour."
              shownBy="src/settings.rs → src/settings/reducer.rs, src/settings/selectors.rs"
              notClaimed="Nothing about the settings UI, and no new tests — the existing suite is the only cover."
            />
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
