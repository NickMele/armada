import type { Meta, StoryObj } from "@storybook/react-vite";
import type { LucideIcon } from "lucide-react";
import { EvidenceCard } from "./EvidenceCard";

/**
 * One submission, on the three fields `crates/verification/src/submission.rs`
 * takes: `claimed`, `shown_by`, `not_claimed`. The stories are the shapes the
 * schema allows — an artifact that is a file set, an artifact that is a
 * command, and an empty `not_claimed`.
 */
const meta: Meta<typeof EvidenceCard> = {
  title: "Compositions/Evidence card",
  component: EvidenceCard,
};
export default meta;

type Story = StoryObj<typeof EvidenceCard>;

/**
 * The card wants the page-with-a-check outline — the `file-*` family means
 * evidence throughout. **`file-check` has no entry in
 * `packages/icons/icons.toml`**, so the mark renders its channel empty rather
 * than reaching for an unregistered glyph. Reported.
 */
const NO_GLYPH_IN_REGISTRY = undefined as unknown as LucideIcon;

/**
 * Evidence so far, on a running job. `Plan the change` is a `facts_note`, the
 * one evidence type whose work product is the submission itself — so
 * `shown_by` points at files rather than at a command.
 */
export const PlanTheChange: Story = {
  args: {
    icon: NO_GLYPH_IN_REGISTRY,
    iconLabel: "Evidence",
    step: "Plan the change",
    time: "09:14",
    claimed: "settings.rs is split into a reducer and a selector module, with no change in behaviour.",
    shownBy: "src/settings.rs → src/settings/reducer.rs, src/settings/selectors.rs",
    notClaimed: "Nothing about the settings UI, and no new tests — the existing suite is the only cover.",
  },
};

/**
 * A gated step. The artifact is a command and its result, which is what
 * `shown_by` holds everywhere except `facts_note` — and the reason the field
 * is mono rather than prose.
 */
export const AnArtifactThatIsACommand: Story = {
  args: {
    icon: NO_GLYPH_IN_REGISTRY,
    iconLabel: "Evidence",
    step: "Run tests",
    time: "14:16",
    claimed: "The ceiling holds at 3 and the counter increments once per poke.",
    shownBy: "cargo test --workspace · 86 passed 0 failed 5.1s · lease::poke_ceiling_holds",
    notClaimed:
      "No test covers a drone that answers on the third poke. The suite was green before this change and is green after.",
  },
};

/**
 * An empty `not_claimed` reads "Nothing", not a dash. The field is not an
 * `Option` in the schema: empty is a legal value and absent is not a value at
 * all, so a dash would render as no answer — exactly the reading the field
 * exists to rule out.
 */
export const NotClaimedEmpty: Story = {
  args: {
    icon: NO_GLYPH_IN_REGISTRY,
    iconLabel: "Evidence",
    step: "Summarise",
    time: "14:20",
    claimed: "The change is on fix/poke-ceiling and ready to read.",
    shownBy: "3 files +214 −96 · branch fix/poke-ceiling",
  },
};
