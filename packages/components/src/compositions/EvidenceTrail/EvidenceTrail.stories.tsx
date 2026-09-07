import type { Meta, StoryObj } from "@storybook/react-vite";
import type { LucideIcon } from "lucide-react";
import { expect } from "storybook/test";

import { EVIDENCE_TYPE } from "../../generated/vocabulary";
import { EvidenceTrail } from "./EvidenceTrail";

/**
 * The trail at the end of a finished Job, and the two entry shapes it can
 * carry: a gated step with its Check, and an ungated one.
 */
const meta: Meta<typeof EvidenceTrail> = {
  title: "Compositions/Evidence trail",
  component: EvidenceTrail,
};
export default meta;

type Story = StoryObj<typeof EvidenceTrail>;

/**
 * Every entry wants the page-with-a-check outline — the `file-*` family means
 * evidence throughout. **`file-check` has no entry in
 * `packages/icons/icons.toml`**, so the mark renders a channel short rather
 * than reaching for an unregistered glyph. Reported.
 */
const NO_GLYPH_IN_REGISTRY = undefined as unknown as LucideIcon;

/**
 * The provenance line, composed the way `review.ts` composes it: the time, the
 * registry's word for the `evidence_type`, and the Checks that let it pass.
 *
 * **Every call below passes a wire value, never a word.** The `evidence_type`
 * rows were authored for this line — `diff`'s row says so — so a fixture
 * spelling the rendered phrase would be a story that keeps passing while the
 * line goes stale. #468.
 */
function provenance(at: string, type: string, check: string): string {
  return `${at} · ${EVIDENCE_TYPE[type]?.verb ?? type} · ${check}`;
}

/**
 * The whole trail on a finished Job: one entry per step in submission order,
 * each with its `evidence_type` and the Checks that let it pass. It is the
 * largest element on the screen, not a panel to expand.
 */
export const AFinishedJob: Story = {
  args: {
    entries: [
      {
        icon: NO_GLYPH_IN_REGISTRY,
        iconLabel: "Evidence",
        step: "Plan the change",
        provenance: provenance("14:02", "facts_note", "no check"),
        claimed: "The poke loop stops after 3 attempts and the job records how many it spent.",
        shownBy: "core/fleet/src/lease.rs · the loop has no ceiling today",
        notClaimed:
          "Does not change the poke interval, and does not decide what happens at the third failure.",
      },
      {
        icon: NO_GLYPH_IN_REGISTRY,
        iconLabel: "Evidence",
        step: "Implement",
        provenance: provenance("14:11", "diff", "build exit 0 · diff_nonempty passed"),
        claimed: "A drone that stops answering is poked at most 3 times, and the count is on the job record.",
        shownBy: "core/fleet/src/lease.rs +38 −7 · core/model/src/job.rs +14 −0",
        notClaimed:
          "The count is not surfaced in Bridge. Nothing acts on reaching the ceiling yet — the loop exits and the job keeps its status.",
      },
      {
        icon: NO_GLYPH_IN_REGISTRY,
        iconLabel: "Evidence",
        step: "Run tests",
        provenance: provenance("14:16", "test_suite_run", "test exit 0"),
        claimed: "The ceiling holds at 3 and the counter increments once per poke.",
        shownBy:
          "cargo test --workspace · 86 passed 0 failed 5.1s · lease::poke_ceiling_holds, lease::poke_count_increments",
        notClaimed:
          "No test covers a drone that answers on the third poke. The suite was green before this change and is green after, so it does not prove the ceiling is reached in practice.",
      },
      {
        icon: NO_GLYPH_IN_REGISTRY,
        iconLabel: "Evidence",
        step: "Summarise",
        provenance: provenance("14:20", "facts_note", "no check"),
        claimed: "The change is on fix/poke-ceiling and ready to read.",
        shownBy: "3 files +214 −96 · branch fix/poke-ceiling",
        notClaimed:
          "The value 3 is a constant rather than config. Whether it is the right number is not established by anything here.",
      },
    ],
  },
  play: async ({ canvas }) => {
    // The expectation is the registry's own verb and never a phrase typed here.
    // A copy in the assertion is the second source reading `EVIDENCE_TYPE`
    // exists to be rid of, and it is the copy that would let this story pass
    // over a provenance line that had stopped following the rows.
    for (const wire of ["facts_note", "diff", "test_suite_run"]) {
      const verb = EVIDENCE_TYPE[wire]?.verb;
      await expect(verb).toBeTruthy();
      await expect(canvas.getAllByText(new RegExp(` · ${verb!} · `))[0]).toBeVisible();
    }
  },
};

/**
 * An empty `not_claimed` reads "Nothing", not a dash. The field is required and
 * may be empty, and a dash would read as no answer — which is exactly the
 * reading the field exists to rule out.
 */
export const NotClaimedEmpty: Story = {
  args: {
    entries: [
      {
        icon: NO_GLYPH_IN_REGISTRY,
        iconLabel: "Evidence",
        step: "Run tests",
        provenance: provenance("14:16", "test_suite_run", "test exit 0"),
        claimed: "The ceiling holds at 3 and the counter increments once per poke.",
        shownBy: "cargo test --workspace · 86 passed 0 failed 5.1s",
      },
    ],
  },
};

/**
 * One entry. A trail of one is the shape at the first submission of a running
 * Job, where the trail is read as evidence so far rather than as a record.
 */
export const OneEntry: Story = {
  args: {
    entries: [
      {
        icon: NO_GLYPH_IN_REGISTRY,
        iconLabel: "Evidence",
        step: "Plan the change",
        provenance: provenance("09:14", "facts_note", "no check"),
        claimed: "settings.rs is split into a reducer and a selector module, with no change in behaviour.",
        shownBy: "src/settings.rs → src/settings/reducer.rs, src/settings/selectors.rs",
        notClaimed: "Nothing about the settings UI, and no new tests — the existing suite is the only cover.",
      },
    ],
  },
};
