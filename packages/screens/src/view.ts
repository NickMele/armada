// The code a View step names, found in the patch the Job page already reads. #904.
//
// **By reference, never copied.** A step names its hunk by the `@@` header, so a
// branch that moved after the review reads as a hunk the patch no longer holds.

import type { DiffLine, ViewSheetStep } from "@armada/components";
import type { Diff, ViewStepRow } from "@armada/protocol";
import { drawnOf } from "./review";

/** Each step of a View, with its hunk's lines out of this Job's patch. */
export function viewStepsOf(diff: Diff, jobId: string, steps: readonly ViewStepRow[]): ViewSheetStep[] {
  const patch = diff.state === "read" && diff.jobId === jobId ? diff.work?.patch : undefined;
  const files = patch === undefined ? [] : drawnOf(patch, () => "").files;
  return steps.map((step) => ({
    file: step.file,
    summary: step.summary,
    ...(step.tie_to_next === undefined ? {} : { tieToNext: step.tie_to_next }),
    lines: hunkOf(files.find((file) => file.path === step.file)?.lines ?? [], step.hunk),
  }));
}

/**
 * One hunk out of a file's lines: its header and every line up to the next header.
 * `null` where no hunk starts with that header. Matched from its start, as Fleet matches it.
 */
export function hunkOf(lines: readonly DiffLine[], hunk: string): DiffLine[] | null {
  const header = hunk.trim();
  if (!header.startsWith("@@")) return null;
  const start = lines.findIndex((line) => line.kind === "hunk" && line.text.startsWith(header));
  const head = lines[start];
  if (start === -1 || head === undefined) return null;
  const rest = lines.slice(start + 1);
  const end = rest.findIndex((line) => line.kind === "hunk");
  return [head, ...(end === -1 ? rest : rest.slice(0, end))];
}
