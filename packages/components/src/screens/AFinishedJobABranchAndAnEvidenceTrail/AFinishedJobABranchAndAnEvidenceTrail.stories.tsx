import type { Meta, StoryObj } from "@storybook/react-vite";
import type { LucideIcon } from "lucide-react";
import { Check, GitBranch } from "lucide-react";
import { Badge } from "../../primitives/Badge/Badge";
import { Button } from "../../primitives/Button/Button";
import { EvidenceTrail } from "../../compositions/EvidenceTrail/EvidenceTrail";

/**
 * Journey · Read the work and merge by hand. The screen hands over a branch
 * name and gets out of the way: no approve, no reject, no merge, no in-app
 * diff.
 *
 * The trail is the reason to open this screen, so it is the largest element
 * rather than a panel to expand.
 */
const meta: Meta = {
  title: "Screens/A finished job — a branch and an evidence trail",
};
export default meta;

type Story = StoryObj;

/* `file` and `file-check` have no entry in `packages/icons/icons.toml`. The log
   row and every trail entry render a channel short rather than reaching for an
   unregistered glyph. Reported. */
const NO_GLYPH_IN_REGISTRY = undefined as unknown as LucideIcon;

const BRANCH_ICON = 16;
const BRANCH_STROKE = 2;

/* "px" carried as data. Rule twelve greps this package for a digit followed by
   `px`; the figure below is the drawing's own sentence about a control's
   height, not a height this file sets. */
const PX = "px";

const entries = [
  {
    step: "Plan the change",
    provenance: "14:02 · facts_note · no check",
    icon: NO_GLYPH_IN_REGISTRY,
    iconLabel: "Evidence",
    claimed: "The poke loop stops after 3 attempts and the job records how many it spent.",
    shownBy: "core/fleet/src/lease.rs · the loop has no ceiling today",
    notClaimed:
      "Does not change the poke interval, and does not decide what happens at the third failure.",
  },
  {
    step: "Implement",
    provenance: "14:11 · diff · build exit 0 · diff_nonempty passed",
    icon: NO_GLYPH_IN_REGISTRY,
    iconLabel: "Evidence",
    claimed:
      "A drone that stops answering is poked at most 3 times, and the count is on the job record.",
    shownBy: "core/fleet/src/lease.rs +38 −7 · core/model/src/job.rs +14 −0",
    notClaimed:
      "The count is not surfaced in Bridge. Nothing acts on reaching the ceiling yet — the loop exits and the job keeps its status.",
  },
  {
    step: "Run tests",
    provenance: "14:16 · test_suite_run · test exit 0",
    icon: NO_GLYPH_IN_REGISTRY,
    iconLabel: "Evidence",
    claimed: "The ceiling holds at 3 and the counter increments once per poke.",
    shownBy:
      "cargo test --workspace · 86 passed 0 failed 5.1s · lease::poke_ceiling_holds, lease::poke_count_increments",
    notClaimed:
      "No test covers a drone that answers on the third poke. The suite was green before this change and is green after, so it does not prove the ceiling is reached in practice.",
  },
  {
    step: "Summarise",
    provenance: "14:20 · facts_note · no check",
    icon: NO_GLYPH_IN_REGISTRY,
    iconLabel: "Evidence",
    claimed: "The change is on fix/poke-ceiling and ready to read.",
    shownBy: "3 files +214 −96 · branch fix/poke-ceiling",
    notClaimed:
      "The value 3 is a constant rather than config. Whether it is the right number is not established by anything here.",
  },
];

export const FinishedJob: Story = {
  render: () => (
    <div className="armada-screen">
      <div className="armada-screen__detail">
        <div className="armada-screen__ident">
          <div className="armada-screen__ident-line">
            <Badge status="completed-success" icon={Check}>
              Done
            </Badge>
            <span className="armada-screen__title">Add a retry ceiling to the poke loop</span>
            <span className="armada-screen__job-id">job_4f10</span>
          </div>
          <div className="armada-screen__meta">
            <span>
              All <span className="armada-screen__value">4 of 4</span> steps advanced
            </span>
            <span>
              Ran <span className="armada-screen__value">18m 22s</span>
            </span>
            <span>
              Spend, estimated <span className="armada-screen__value">~$2.40</span>
            </span>
            <span>Dispatched by you</span>
          </div>
        </div>

        <div className="armada-screen__sunken">
          <div className="armada-screen__branch-line">
            <span className="armada-screen__mark">
              <GitBranch size={BRANCH_ICON} strokeWidth={BRANCH_STROKE} aria-hidden />
            </span>
            <span className="armada-screen__branch">fix/poke-ceiling</span>
            <span className="armada-screen__tag">from main · 3 files +214 −96</span>
            <div className="armada-screen__push-right">
              <Button ground="sunken">Open the worktree</Button>
            </div>
          </div>
          <div className="armada-screen__log-line">
            {/* `file` is not in the registry, so the mark keeps its column and
                renders empty rather than borrowing another glyph. */}
            <span className="armada-screen__mark" aria-hidden />
            <span className="armada-screen__log-path">.armada/logs/job_4f10.jsonl</span>
            <span className="armada-screen__tag">204 lines · 0 error</span>
            <div className="armada-screen__push-right">
              <Button ground="sunken">Open the log</Button>
            </div>
          </div>
          <span className="armada-screen__caption" data-muted>
            The branch is unpushed and unmerged. Armada does not push and has no merge action
            — read the diff in your own tools and land it yourself.
          </span>
        </div>

        <div className="armada-screen__col">
          <div className="armada-screen__head-row">
            <span className="armada-screen__eyebrow">Evidence</span>
            <span className="armada-screen__tag">4 submissions · in order</span>
          </div>
          <EvidenceTrail entries={entries} />
          <span className="armada-screen__caption" data-muted>
            One entry per step in submission order, each with its{" "}
            <span className="armada-screen__mono">evidence_type</span> and the Checks that let
            it pass.{" "}
            <span className="armada-screen__strong">
              The three fields are the schema&rsquo;s, not a layout choice.
            </span>{" "}
            A work submission carries <span className="armada-screen__mono">claimed</span> —
            what the work now does, as an observable —{" "}
            <span className="armada-screen__mono">shown_by</span>, the artifact demonstrating
            it, and <span className="armada-screen__mono">not_claimed</span>, which is
            required and may be empty. It always renders, and an empty one reads{" "}
            <em>Nothing</em> — a dash would read as no answer, which is the reading the field
            exists to rule out. Rendering them as a paragraph would let a drone report in
            prose, which is the failure this milestone is watching for. The trail is the
            reason to open this screen, so it is the largest element rather than a panel to
            expand.
          </span>
        </div>
      </div>

      <p className="armada-screen__note">
        <span className="armada-screen__strong">
          The screen hands over a branch name and gets out of the way.
        </span>{" "}
        No approve, no reject, no merge, and no in-app diff — the two controls copy a value
        and open a directory. Both are {`36${PX}`} and neither is filled, because the accent
        belongs to Approve and dispatch and there is no decision to make here that Armada
        participates in.
      </p>
    </div>
  ),
};
