import type { Meta, StoryObj } from "@storybook/react-vite";
import { ToolName } from "./ToolName";

/**
 * One story per family, and one for a tool the roster has no reading for —
 * which is the state that must draw neutral rather than pick a fourth hue.
 *
 * `Log rows` is the reading the colours exist for: the three families down one
 * mono column, where the shape of an hour's work is what shows.
 */
const meta: Meta<typeof ToolName> = {
  title: "Compositions/Tool name",
  component: ToolName,
};
export default meta;

type Story = StoryObj<typeof ToolName>;

export const Looking: Story = { args: { tool: "Read" } };
export const Changing: Story = { args: { tool: "Edit" } };
export const Running: Story = { args: { tool: "Bash" } };
export const Unclassified: Story = { args: { tool: "WebFetch" } };

const CALLS: [string, string][] = [
  ["Grep", "Saw::Called in crates/"],
  ["Read", "crates/store/src/tests/forget.rs"],
  ["Read", "crates/fleet/src/settling.rs"],
  ["Edit", "crates/fleet/src/settling.rs"],
  ["Write", "crates/fleet/src/ending.rs"],
  ["Bash", "cargo test -p fleet"],
  ["WebFetch", "https://docs.rs/tokio"],
];

export const LogRows: StoryObj = {
  name: "Log rows",
  render: () => (
    <div className="flex flex-col gap-1 bg-bg-sunken p-3 font-mono text-2xs text-fg-default">
      {CALLS.map(([tool, detail], at) => (
        <span key={at}>
          <ToolName tool={tool} /> {detail}
        </span>
      ))}
    </div>
  ),
};
