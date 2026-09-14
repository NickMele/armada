import type { ReactNode } from "react";
import { SplitButton } from "../../primitives/SplitButton/SplitButton";
import { FramesShown, type ShownFrame } from "../FramesShown/FramesShown";
import type { ConfidenceCi } from "./ConfidenceSheet";
import { withCode } from "./withCode";

/** One thing the verdict rests on: what it came to, and what that was measured on. */
export type GroundRow = {
  /** A phrase, not a sentence: `7 of 7 passed`. */
  result: string;
  /** `not_met` marks the row and opens the section. */
  tone: "met" | "not_met" | "quiet";
  /** Backticks mark code. */
  detail?: string;
};

/** What the verdict rests on besides the pull request's CI, which `ci` carries. */
export type ConfidenceGrounds = {
  checks?: GroundRow;
  judge?: GroundRow;
  evidence?: GroundRow;
  plan?: GroundRow;
};

/** What the Job captured: the frames its harness kept, and the Drone's own claim. */
export type ConfidenceCaptured = {
  frames: ShownFrame[];
  claim?: { claimed: string; shownBy: string; notClaimed?: string };
};

type Line = { source: string; short: string; row: GroundRow; ci?: ConfidenceCi };

/** The rows in the order a person checks them, **the order is this file's**, not the caller's. */
export function groundLines(grounds: ConfidenceGrounds | undefined, ci: ConfidenceCi | undefined): Line[] {
  const lines: Line[] = [];
  if (grounds?.checks) lines.push({ source: "Checks", short: "Checks", row: grounds.checks });
  if (ci) lines.push({ source: "Pull request CI", short: "CI", row: ciRow(ci), ci });
  if (grounds?.judge) lines.push({ source: "Judge", short: "Judge", row: grounds.judge });
  if (grounds?.evidence) lines.push({ source: "Drone's evidence", short: "Evidence", row: grounds.evidence });
  if (grounds?.plan) lines.push({ source: "Plan", short: "Plan", row: grounds.plan });
  return lines;
}

function ciRow(ci: ConfidenceCi): GroundRow {
  const failing = ci.conflicted || ci.failed.length > 0;
  return {
    result: ci.said,
    tone: failing ? "not_met" : ci.kind === "all_passed" ? "met" : "quiet",
    ...(ci.conflicted || ci.failed.length === 0
      ? {}
      : { detail: ci.failed.map((name) => `\`${name}\``).join(", ") }),
  };
}

/** `Checks 7 of 7 passed · CI 4 of 4 passed · Evidence matches the diff`. */
export function groundsSummary(lines: readonly Line[]): string {
  return lines.map(({ short, row }) => `${short} ${lowerFirst(row.result)}`).join(" · ");
}

/** Why the section opened itself, where a row did not hold: `Pull request CI: 1 of 4 failed`. */
export function groundsOpened(lines: readonly Line[]): string | undefined {
  const failing = lines.filter((line) => line.row.tone === "not_met");
  if (failing.length === 0) return undefined;
  return failing.map((line) => `${line.source}: ${line.row.result}`).join(" · ");
}

export function Grounds({ lines }: { lines: readonly Line[] }) {
  return (
    <table className="armada-confidence__table" aria-label="What the verdict rests on">
      <thead>
        <tr>
          <th scope="col">Source</th>
          <th scope="col">Result</th>
          <th scope="col">Detail</th>
        </tr>
      </thead>
      <tbody>
        {lines.map((line) => {
          const act = line.ci === undefined ? null : ciAct(line.ci);
          return (
            <tr key={line.source} {...(line.row.tone === "not_met" ? { "data-flagged": "" } : {})}>
              <td className="armada-confidence__muted">{line.source}</td>
              <td className="armada-confidence__result" data-tone={line.row.tone}>
                {line.row.result}
              </td>
              <td className="armada-confidence__muted">
                {line.row.detail === undefined ? null : withCode(line.row.detail)}
                {/* Under the detail, as a follow-up sits under its reason: a column clipped it. */}
                {act === null ? null : <div className="armada-confidence__follow">{act}</div>}
              </td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}

/**
 * A conflict has nothing to press here — Fleet already sends the Drone back
 * on its own, `#1131` — so this row draws an act only for a failed CI run.
 */
function ciAct(ci: ConfidenceCi): ReactNode {
  if (ci.conflicted) return null;
  if (ci.failed.length > 0 && ci.onInvestigate !== undefined) {
    return (
      <SplitButton
        items={ci.onRerun === undefined ? [] : [{ label: "Re-run the failed runs", onSelect: ci.onRerun }]}
        disabled={ci.disabled}
        onAction={ci.onInvestigate}
        menuLabel="More about CI"
      >
        Investigate
      </SplitButton>
    );
  }
  return null;
}

export function capturedSummary(captured: ConfidenceCaptured): string {
  const counted = captured.frames.length === 0 ? "Nothing captured" : `${captured.frames.length} captured`;
  return captured.claim === undefined ? counted : `${counted} · the Drone's claim`;
}

export function Captured({ captured }: { captured: ConfidenceCaptured }) {
  const { frames, claim } = captured;
  return (
    <div className="armada-confidence__tests">
      {frames.length > 0 && <FramesShown frames={frames} />}
      {claim === undefined ? null : (
        <>
          <span className="armada-confidence__sublabel">The Drone&rsquo;s claim</span>
          <dl className="armada-confidence__claim">
            <dt>What it did</dt>
            <dd>{withCode(claim.claimed)}</dd>
            <dt>Shown by</dt>
            <dd>{withCode(claim.shownBy)}</dd>
            {claim.notClaimed === undefined ? null : (
              <>
                <dt>What it left alone</dt>
                <dd>{withCode(claim.notClaimed)}</dd>
              </>
            )}
          </dl>
        </>
      )}
    </div>
  );
}

function lowerFirst(text: string): string {
  return text.charAt(0).toLowerCase() + text.slice(1);
}
