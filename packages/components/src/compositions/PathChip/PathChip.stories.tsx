import type { Meta, StoryObj } from "@storybook/react-vite";
import { PathChip } from "./PathChip";

const meta: Meta<typeof PathChip> = {
  title: "Compositions/Path chip",
  component: PathChip,
};
export default meta;

type Story = StoryObj<typeof PathChip>;

/**
 * A path with room for all of it. Nothing truncates, and the two halves are
 * still drawn as two: the directory recedes to `--fg-subtle` so the filename
 * reads first even where both fit.
 */
export const AShortPath: Story = {
  args: {
    directory: "packages/settings/test/",
    basename: "useColumnSelectors.test.ts",
  },
};

/**
 * **The case this component exists for.** A path far wider than the 380px run
 * column, in a box narrower still.
 *
 * The directory loses its start and the filename survives whole. Clipped the
 * ordinary way — from the right — this row would read
 * `.armada/artifacts/job_2d90bb/roo…` and name nothing.
 */
export const ALongPathTruncatingItsDirectory: Story = {
  render: () => (
    <div
      style={{
        width: "calc(var(--space-12) * 4)",
        display: "flex",
        flexDirection: "column",
        gap: "var(--space-1)",
      }}
    >
      <PathChip directory=".armada/artifacts/job_2d90bb/" basename="root_cause.md" />
      <PathChip directory="packages/settings/src/" basename="selectors.ts" />
      <PathChip directory="packages/settings/src/" basename="reducer.ts" />
      <PathChip directory="packages/settings/src/" basename="index.ts" />
    </div>
  ),
};

/**
 * A path with a note. The note is sans against the path's mono, because it is
 * something said about the value rather than more of it — the same split the
 * run tree makes between a fact's name and a fact's value.
 */
export const WithWhatItIs: Story = {
  args: {
    directory: "packages/settings/src/",
    basename: "selectors.ts",
    note: "+61 −4",
  },
};

/**
 * A file at the repository root. No directory, so no slot is drawn — an empty
 * span before the filename would render as a stray separator.
 */
export const AtTheRoot: Story = {
  args: { basename: "armada.yml" },
};
