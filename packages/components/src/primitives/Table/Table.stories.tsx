import type { Meta, StoryObj } from "@storybook/react-vite";
import type { ReactNode } from "react";
import { Table, TableBody, TableCell, TableHead, TableHeaderCell, TableRow } from "./Table";

/**
 * The Doctor health grid, which is what the sheet draws a table as: tabular
 * data, never a list of jobs. A job is always the stacked row, on the Board,
 * in Alerts, in Reviews and in Active jobs — and that row is a composition,
 * not this.
 *
 * Column tracks are the composition's, not the primitive's, so nothing here
 * declares one and the table lays itself out. There is no track width token to
 * declare one with, and a pixel width written into this file is an arbitrary
 * value the gate refuses.
 */
const meta: Meta<typeof Table> = {
  title: "Primitives/Table",
  component: Table,
};
export default meta;

type Story = StoryObj<typeof Table>;

type Condition = {
  module: string;
  result: string;
  detail: string;
  checked: string;
};

const conditions: Condition[] = [
  { module: "Fleet daemon", result: "ok", detail: "pid 4417, uptime 6d", checked: "2m ago" },
  { module: "Git", result: "ok", detail: "worktrees clean", checked: "2m ago" },
  { module: "Disk", result: "low", detail: "4% free where Fleet writes", checked: "2m ago" },
];

function Grid({ children }: { children: ReactNode }) {
  return <div style={{ maxWidth: "88ch" }}>{children}</div>;
}

function Header() {
  return (
    <TableHead>
      <TableRow>
        <TableHeaderCell>Module</TableHeaderCell>
        <TableHeaderCell>Result</TableHeaderCell>
        <TableHeaderCell>Detail</TableHeaderCell>
        <TableHeaderCell>Checked</TableHeaderCell>
      </TableRow>
    </TableHead>
  );
}

/** Resting: 32px header, 36px rows, a `--border-subtle` rule, no striping. */
export const Default: Story = {
  render: () => (
    <Grid>
      <Table>
        <Header />
        <TableBody>
          {conditions.map((c) => (
            <TableRow key={c.module}>
              <TableCell>{c.module}</TableCell>
              <TableCell variant="mono">{c.result}</TableCell>
              <TableCell variant="mono" truncates>
                {c.detail}
              </TableCell>
              <TableCell variant="metadata">{c.checked}</TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </Grid>
  ),
};

/**
 * Focused and selected are different states and coexist. Focus is a 2px
 * `--accent` left edge over `--bg-hover`; selection is an `--accent-muted`
 * fill. The third row is both.
 */
export const FocusedAndSelected: Story = {
  render: () => (
    <Grid>
      <Table>
        <Header />
        <TableBody>
          <TableRow focused>
            <TableCell>Fleet daemon</TableCell>
            <TableCell variant="mono">ok</TableCell>
            <TableCell variant="secondary">Focused — the keyboard cursor</TableCell>
            <TableCell variant="metadata">2m ago</TableCell>
          </TableRow>
          <TableRow selected>
            <TableCell>Git</TableCell>
            <TableCell variant="mono">ok</TableCell>
            <TableCell variant="secondary">Selected</TableCell>
            <TableCell variant="metadata">2m ago</TableCell>
          </TableRow>
          <TableRow focused selected>
            <TableCell>Disk</TableCell>
            <TableCell variant="mono">low</TableCell>
            <TableCell variant="secondary">Both at once</TableCell>
            <TableCell variant="metadata">2m ago</TableCell>
          </TableRow>
        </TableBody>
      </Table>
    </Grid>
  ),
};

/**
 * A machine value copies on click and goes to `--accent` on hover. No `copy`
 * glyph, and no button beside it. `onCopied` is where a surface hangs the
 * toast, because a clipboard write is silent by nature.
 */
export const MonoValuesCopy: Story = {
  render: () => (
    <Grid>
      <Table>
        <TableBody>
          <TableRow>
            <TableCell variant="secondary">Job</TableCell>
            <TableCell variant="mono" copyValue="job_8f2a1c">
              job_8f2a1c
            </TableCell>
          </TableRow>
          <TableRow>
            <TableCell variant="secondary">Branch</TableCell>
            <TableCell variant="mono" copyValue="feat/auth-refresh">
              feat/auth-refresh
            </TableCell>
          </TableRow>
          <TableRow>
            <TableCell variant="secondary">Path</TableCell>
            <TableCell variant="mono" copyValue="auth/session.rs">
              auth/session.rs
            </TableCell>
          </TableRow>
        </TableBody>
      </Table>
    </Grid>
  ),
};

/**
 * A de-emphasised row steps to `--border-subtle` and `--fg-subtle`. Never
 * `opacity`, which muddies any status colour in the row.
 */
export const Dimmed: Story = {
  render: () => (
    <Grid>
      <Table>
        <Header />
        <TableBody>
          <TableRow>
            <TableCell>Fleet daemon</TableCell>
            <TableCell variant="mono">ok</TableCell>
            <TableCell variant="secondary">Reads at full weight</TableCell>
            <TableCell variant="metadata">2m ago</TableCell>
          </TableRow>
          <TableRow dimmed>
            <TableCell>Disk</TableCell>
            <TableCell variant="mono">not checked</TableCell>
            <TableCell variant="secondary">Stepped down a token</TableCell>
            <TableCell variant="metadata">2m ago</TableCell>
          </TableRow>
        </TableBody>
      </Table>
    </Grid>
  ),
};

/**
 * The row height is a floor, not a cap: content that grows past 36px grows the
 * row rather than clipping, and the floor is what keeps rows aligned down a
 * column when the content is short.
 */
export const RowsGrowWithContent: Story = {
  render: () => (
    <Grid>
      <Table>
        <Header />
        <TableBody>
          <TableRow>
            <TableCell>Git</TableCell>
            <TableCell variant="mono">ok</TableCell>
            <TableCell variant="secondary">
              The branch was deleted outside Armada, so the worktree Fleet cut for this job no
              longer resolves and two jobs are blocked behind it.
            </TableCell>
            <TableCell variant="metadata">2m ago</TableCell>
          </TableRow>
          <TableRow>
            <TableCell>Disk</TableCell>
            <TableCell variant="mono">ok</TableCell>
            <TableCell variant="secondary">Short.</TableCell>
            <TableCell variant="metadata">2m ago</TableCell>
          </TableRow>
        </TableBody>
      </Table>
    </Grid>
  ),
};
