import type { Meta, StoryObj } from "@storybook/react-vite";
import { Table, TableBody, TableCell, TableRow } from "../Table/Table";
import { ScrollArea } from "./ScrollArea";

/**
 * The one place the contract names a scroll region is the command palette,
 * which is `--palette-width` wide and scrolls past `--palette-max-height`.
 * Both stories are drawn at those tokens because they are the only sizes the
 * sources give a scrolling region.
 */
const meta: Meta<typeof ScrollArea> = {
  title: "Primitives/ScrollArea",
  component: ScrollArea,
};
export default meta;

type Story = StoryObj<typeof ScrollArea>;

const jobs = Array.from({ length: 18 }, (_, i) => ({
  id: `job_${(i + 1).toString().padStart(4, "0")}`,
  branch: `feat/step-${i + 1}`,
  elapsed: `${i + 2}m`,
}));

/** Content past the bound: the scrollbar is the only thing that says so. */
export const Scrolling: Story = {
  render: () => (
    <div
      style={{
        width: "var(--palette-width)",
        border: "var(--border-width) solid var(--border-default)",
        borderRadius: "var(--radius-lg)",
        background: "var(--bg-overlay)",
        overflow: "hidden",
      }}
    >
      <ScrollArea maxHeight="var(--palette-max-height)">
        <Table>
          <TableBody>
            {jobs.map((j) => (
              <TableRow key={j.id}>
                <TableCell variant="mono" copyValue={j.id}>
                  {j.id}
                </TableCell>
                <TableCell variant="mono">{j.branch}</TableCell>
                <TableCell variant="metadata">{j.elapsed}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </ScrollArea>
    </div>
  ),
};

/**
 * Content inside the bound: no scrollbar, and the region ends where its
 * content ends rather than holding open to its maximum.
 */
export const WithinBounds: Story = {
  render: () => (
    <div
      style={{
        width: "var(--palette-width)",
        border: "var(--border-width) solid var(--border-default)",
        borderRadius: "var(--radius-lg)",
        background: "var(--bg-overlay)",
        overflow: "hidden",
      }}
    >
      <ScrollArea maxHeight="var(--palette-max-height)">
        <Table>
          <TableBody>
            {jobs.slice(0, 3).map((j) => (
              <TableRow key={j.id}>
                <TableCell variant="mono" copyValue={j.id}>
                  {j.id}
                </TableCell>
                <TableCell variant="mono">{j.branch}</TableCell>
                <TableCell variant="metadata">{j.elapsed}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </ScrollArea>
    </div>
  ),
};
