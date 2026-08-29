import type { Meta, StoryObj } from "@storybook/react-vite";
import { Dialog } from "../../primitives/Dialog/Dialog";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { GamingFlags } from "./GamingFlags";

const meta: Meta<typeof GamingFlags> = {
  title: "Compositions/Gaming flags",
  component: GamingFlags,
};
export default meta;

type Story = StoryObj<typeof GamingFlags>;

/**
 * The rail's row. One line per flag, the pattern in mono and the citation
 * clipped with the rest in the title — a rail is a column of pointers and has
 * no width to give.
 */
export const OnTheRail: Story = {
  args: {
    citation: "clipped",
    said: "the gaming check flagged this evidence",
    flags: [
      { pattern: "check_config_edited", cited: "armada.yml — checks.tests.command" },
      {
        pattern: "assertion_weakened",
        cited:
          "crates/api/src/tests/served.rs:214 — served_every_operation counts the filtered set",
      },
    ],
  },
};

/**
 * **The case that produced #197.** Two flags, each with a citation running to
 * several lines and carrying a path and an expression.
 *
 * Rendered as one assembled sentence — "It flagged X in Y, Z in W." — this is
 * the paragraph the owner could not read, and the block that pushed the reason
 * field and the confirm control off the bottom of the window. The wire has
 * carried `pattern` and `cited` as separate fields the whole time.
 *
 * The pattern stays mono because it is a value. The citation is sans, because
 * it is a sentence a model wrote, and `Prose` puts the mono back on the paths
 * and the expression inside it.
 */
export const TwoFlagsReadInFull: Story = {
  args: {
    citation: "whole",
    said: "What it flagged",
    flags: [
      {
        pattern: "check_config_edited",
        cited:
          "`armada.yml` — the `checks.tests.command` key was changed in the same step whose " +
          "evidence it gates. The command it was changed **to** exits 0 on an empty test set:\n\n" +
          "```\ncargo nextest run --workspace -E 'test(served_every_operation)'\n```\n\n" +
          "The previous command ran the whole workspace, so the step's evidence is a green run " +
          "of a narrower set than the one the criterion names.",
      },
      {
        pattern: "assertion_weakened",
        cited:
          "`crates/api/src/tests/served.rs:214` — `served_every_operation` walks the route " +
          "table once per operation and the loop skips `forget_job`:\n\n" +
          '```\nlet routes: Vec<&Route> = ROUTES.iter().filter(|r| r.operation != "forget_job").collect();\n' +
          "assert_eq!(routes.len(), served.len());\n```\n\n" +
          "The count the assertion compares against was lowered by the same edit that made it " +
          "pass, so the one operation the step was about is the one operation never read.",
      },
    ],
  },
};

/**
 * A flag Fleet raised and cited nothing for. **The slot is not drawn rather
 * than drawn empty** — an uncited flag is unactionable, and rendering a blank
 * where the finding goes says the citation is missing rather than that it was
 * never given.
 */
export const FlaggedAndNotCited: Story = {
  args: {
    citation: "whole",
    flags: [{ pattern: "evidence_reused" }],
  },
};

/**
 * **The whole of #197, in one render.** Two flags with long citations, in the
 * dialog the owner was looking at when he reported it.
 *
 * What changed, and each of the three is visible here:
 *
 * - the body scrolls, so the explanation no longer runs off the window;
 * - the reason field and both controls are pinned below it, reachable at any
 *   window height — shorten the preview and they stay;
 * - the flags are drawn from their two fields instead of being assembled into
 *   "It flagged X in Y, Z in W.", and the citations are rendered rather than
 *   printed, so a path is mono and an expression is a block that wraps.
 *
 * The confirm control is disabled because the reason is blank, which is the
 * 422 Fleet would answer — the dialog refuses it here rather than on the press.
 */
export const OverrulingTwoFlags: Story = {
  args: TwoFlagsReadInFull.args,
  render: (args) => (
    <Dialog
      open
      tone="neutral"
      width="wide"
      title="Overrule the gaming flag on this step?"
      confirmLabel="Overrule the flag"
      confirmDisabled
      field={<Textarea label="Why the flag is wrong" rows={3} />}
    >
      <p>
        The gaming check flagged the evidence for Regression check. It did not refuse the work — it
        says the evidence for it is not to be trusted. Overruling says a person has read that
        evidence and takes responsibility for it; the step advances still recorded as failed
        against the flag.
      </p>
      <GamingFlags {...args} />
      <p>
        It is not the last step, so the job carries on at the next one. Your reason is written to
        this job&apos;s log and stays there — the log is append-only, and nothing takes an override
        back. It is not sent to the drone, which did nothing wrong and is told only that the step
        was accepted.
      </p>
    </Dialog>
  ),
};
