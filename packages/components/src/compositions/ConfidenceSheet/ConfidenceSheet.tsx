import type { ReactNode } from "react";
import { useState } from "react";
import { ChevronDown, ChevronRight, Eye } from "lucide-react";
import type {
  AreaRow,
  FindingRow,
  FollowedRow,
  JobConfidence,
  OpenedBecause,
  TestsSection,
  ViewStepRow,
} from "@armada/protocol";
import { Button } from "../../primitives/Button/Button";
import { SplitButton } from "../../primitives/SplitButton/SplitButton";

/**
 * Armada's review of a change, drawn above the verdict sheet at the gate. #903.
 *
 * **The verdict and Needs you are always open**, because they are what a person is here for.
 * Every other section folds to one summary line. Tests in the change opens itself when a test
 * was removed or loosened with no reason, under a callout naming it, with its row marked.
 */
export type ConfidenceSheetProps = {
  confidence: JobConfidence;
  /** Opens a row's View. #904. Absent leaves every View button out. */
  onView?: (view: ConfidenceView) => void;
  /** The pull request's CI, and what a person can do about it. #905. Absent draws no row. */
  ci?: ConfidenceCi;
  /** What a For context finding can become. #906. Absent leaves its buttons out. */
  followUp?: ConfidenceFollowUp;
};

/** The Pull request CI row: the forge's own CI, never totalled with Armada's Checks. #905. */
export type ConfidenceCi = {
  kind: string;
  /** The row's one line. */
  said: string;
  /** What the forge calls each check that failed. */
  failed: string[];
  /** The branch conflicts with main, which outranks a failed run. */
  conflicted: boolean;
  onInvestigate?: () => void;
  onRerun?: () => void;
  onResolve?: () => void;
  disabled?: boolean;
};

/** Queue after this lands and File an issue, on a For context finding. #906. */
export type ConfidenceFollowUp = {
  onQueueAfter: (finding: string) => void;
  onFileIssue: (finding: string) => void;
  /** Opens the issue a finding became. Absent says it was filed, with nothing to press. */
  onOpenIssue?: (finding: string) => void;
  disabled?: boolean;
};

/** What a View button hands over: what the View is of, and its steps. */
export type ConfidenceView = {
  title: string;
  steps: ViewStepRow[];
  /** The finding's own words, where the View is of a finding rather than an area. #907. */
  finding?: string;
};

type OnView = ConfidenceSheetProps["onView"];

export function ConfidenceSheet({ confidence, onView, ci, followUp }: ConfidenceSheetProps) {
  const { says, reasons, areas, tests, needs_you, small_fixes, for_context } = confidence;
  const dismissed = confidence.dismissed ?? [];
  return (
    <section className="armada-confidence" aria-label="Armada's review">
      <div className="armada-confidence__block">
        <span className="armada-confidence__label">Armada&rsquo;s review</span>
        <span className="armada-confidence__says" data-says={says}>
          {says === "confident" ? "Confident" : "Not confident"}
        </span>
        {reasons.length > 0 && (
          <ul className="armada-confidence__reasons">
            {reasons.map((reason) => (
              <li key={reason}>{reason}</li>
            ))}
          </ul>
        )}
      </div>
      {ci === undefined ? null : (
        <div className="armada-confidence__block">
          <span className="armada-confidence__label">Pull request CI</span>
          <div className="armada-confidence__ci">
            <span className="armada-confidence__said" data-ci={ci.kind}>
              {ci.said}
            </span>
            {ci.conflicted && ci.onResolve !== undefined ? (
              <Button size="sm" disabled={ci.disabled} onClick={ci.onResolve}>
                Resolve conflicts
              </Button>
            ) : ci.failed.length > 0 && ci.onInvestigate !== undefined ? (
              <SplitButton
                items={
                  ci.onRerun === undefined
                    ? []
                    : [{ label: "Re-run the failed runs", onSelect: ci.onRerun }]
                }
                disabled={ci.disabled}
                onAction={ci.onInvestigate}
                menuLabel="More about CI"
              >
                Investigate
              </SplitButton>
            ) : null}
          </div>
          {ci.failed.length === 0 ? null : (
            <ul className="armada-confidence__failed" aria-label="Failed checks">
              {ci.failed.map((name) => (
                <li key={name}>
                  <code className="armada-confidence__path">{name}</code>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
      {areas.length > 0 && (
        <Fold title="Shape of the change" summary={areaCount(areas)}>
          <Areas areas={areas} onView={onView} />
        </Fold>
      )}
      {tests !== undefined && (
        <Fold
          title="Tests in the change"
          summary={testsSummary(tests)}
          {...(tests.opened_because === undefined
            ? {}
            : { opened: openedLine(tests.opened_because) })}
        >
          <Tests tests={tests} />
        </Fold>
      )}
      <div className="armada-confidence__block">
        <span className="armada-confidence__label">
          Needs you <span className="armada-confidence__count">{needs_you.length}</span>
        </span>
        {needs_you.length === 0 ? (
          <p className="armada-confidence__empty">Nothing needs you.</p>
        ) : (
          <Findings findings={needs_you} marked onView={onView} />
        )}
      </div>
      {small_fixes.length > 0 && (
        <Fold title="Small fixes for a Drone" summary={findingCount(small_fixes)}>
          <Findings findings={small_fixes} onView={onView} />
        </Fold>
      )}
      {for_context.length > 0 && (
        <Fold title="For context" summary={findingCount(for_context)}>
          <Findings
            findings={for_context}
            onView={onView}
            followed={confidence.followed ?? []}
            {...(followUp === undefined ? {} : { followUp })}
          />
        </Fold>
      )}
      {dismissed.length > 0 && (
        <Fold
          title="Dismissed"
          summary={dismissed.length === 1 ? "1 finding" : `${dismissed.length} findings`}
        >
          <table
            className="armada-confidence__table"
            data-shape="prose"
            aria-label="Dismissed findings"
          >
            <thead>
              <tr>
                <th scope="col">Finding</th>
                <th scope="col">Why it was dismissed</th>
              </tr>
            </thead>
            <tbody>
              {dismissed.map((row) => (
                <tr key={row.finding}>
                  <td>{withCode(row.finding)}</td>
                  <td className="armada-confidence__muted">{withCode(row.reason)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </Fold>
      )}
    </section>
  );
}

/** One section folded to its summary line. `opened` is why it starts open, shown as a callout. */
function Fold({
  title,
  summary,
  opened,
  children,
}: {
  title: string;
  summary: ReactNode;
  opened?: ReactNode;
  children: ReactNode;
}) {
  const [open, setOpen] = useState(opened !== undefined);
  const Mark = open ? ChevronDown : ChevronRight;
  return (
    <div className="armada-confidence__block">
      <button
        type="button"
        className="armada-confidence__fold"
        aria-expanded={open}
        onClick={() => setOpen((was) => !was)}
      >
        <Mark size={12} aria-hidden="true" className="armada-confidence__mark" />
        <span className="armada-confidence__label">{title}</span>
        <span className="armada-confidence__summary">{summary}</span>
      </button>
      {opened !== undefined && (
        <p className="armada-confidence__callout" role="note">
          <Eye size={12} aria-hidden="true" className="armada-confidence__eye" />
          <span>{opened}</span>
        </p>
      )}
      {/* `hidden`, not unmounted, on `DroneBrief`'s rule: a folded section stays in the page. */}
      <div className="armada-confidence__body" hidden={!open}>
        {children}
      </div>
    </div>
  );
}

/** The View button's cell, and its header, where any row in the table has a View. */
function viewColumn(rows: readonly { view?: ViewStepRow[] }[], onView: OnView): boolean {
  return onView !== undefined && rows.some((row) => (row.view?.length ?? 0) > 0);
}

function ViewCell({
  title,
  view,
  onView,
  finding,
}: {
  title: string;
  view: ViewStepRow[] | undefined;
  onView: OnView;
  /** The finding's own words, so a dismissal names what the review served. #907. */
  finding?: string;
}) {
  return (
    <td className="armada-confidence__act">
      {onView === undefined || view === undefined || view.length === 0 ? null : (
        <Button size="sm" onClick={() => onView({ title, steps: view, ...(finding === undefined ? {} : { finding }) })}>
          View
        </Button>
      )}
    </td>
  );
}

function Areas({ areas, onView }: { areas: readonly AreaRow[]; onView: OnView }) {
  const viewable = viewColumn(areas, onView);
  return (
    <table className="armada-confidence__table">
      <thead>
        <tr>
          <th scope="col">Area</th>
          <th scope="col">What changed</th>
          <th scope="col">Files</th>
          {viewable && <th scope="col" className="armada-confidence__act" aria-label="View" />}
        </tr>
      </thead>
      <tbody>
        {areas.map((area) => (
          <tr key={area.name}>
            <td className="armada-confidence__muted">{area.name}</td>
            <td>{area.what}</td>
            <td className="armada-confidence__files">
              {area.files.map((file) => (
                <code key={file} className="armada-confidence__path">
                  {file}
                </code>
              ))}
            </td>
            {viewable && <ViewCell title={area.name} view={area.view} onView={onView} />}
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function Tests({ tests }: { tests: TestsSection }) {
  return (
    <div className="armada-confidence__tests">
      {tests.proves.length > 0 && (
        <table className="armada-confidence__table">
          <thead>
            <tr>
              <th scope="col">Area</th>
              <th scope="col">What the tests prove</th>
              <th scope="col" className="armada-confidence__number">
                Tests
              </th>
            </tr>
          </thead>
          <tbody>
            {tests.proves.map((row) => (
              <tr key={`${row.area}-${row.what}`}>
                <td className="armada-confidence__muted">{row.area}</td>
                <td>{row.what}</td>
                <td className="armada-confidence__number">{row.tests}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      {tests.changed.length > 0 && (
        <table className="armada-confidence__table" aria-label="Changed or removed tests">
          <thead>
            <tr>
              <th scope="col">Test</th>
              <th scope="col">What changed</th>
              <th scope="col">Why</th>
            </tr>
          </thead>
          <tbody>
            {tests.changed.map((row) => (
              <tr key={row.name} {...(row.flagged ? { "data-flagged": "" } : {})}>
                <td>
                  <code className="armada-confidence__path">{row.name}</code>
                </td>
                <td>
                  {row.change === "loosened" ? "Loosened" : "Removed"}
                  {row.replaced_by !== undefined && (
                    <>
                      {", replaced by "}
                      <code className="armada-confidence__path">{row.replaced_by}</code>
                    </>
                  )}
                </td>
                <td className="armada-confidence__muted">{row.why ?? "No reason given"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      {tests.untested.length > 0 && (
        <table className="armada-confidence__table" data-shape="prose" aria-label="Not tested">
          <thead>
            <tr>
              <th scope="col">Changed code no test reaches</th>
              <th scope="col">Why it matters</th>
            </tr>
          </thead>
          <tbody>
            {tests.untested.map((row) => (
              <tr key={row.code}>
                <td>{row.code}</td>
                <td className="armada-confidence__muted">{row.why}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}

function Findings({
  findings,
  marked,
  onView,
  followed = [],
  followUp,
}: {
  findings: readonly FindingRow[];
  marked?: boolean;
  onView: OnView;
  followed?: readonly FollowedRow[];
  followUp?: ConfidenceFollowUp;
}) {
  const viewable = viewColumn(findings, onView);
  return (
    <table
      className="armada-confidence__table"
      data-shape="prose"
      {...(marked ? { "data-marked": "" } : {})}
    >
      <thead>
        <tr>
          <th scope="col">Finding</th>
          <th scope="col">Why it is here</th>
          {viewable && <th scope="col" className="armada-confidence__act" aria-label="View" />}
        </tr>
      </thead>
      <tbody>
        {findings.map((row) => (
          <tr key={row.finding}>
            <td>{withCode(row.finding)}</td>
            <td className="armada-confidence__muted">
              {withCode(row.why)}
              {/* Under the reason rather than in a column of its own, which clipped it. */}
              {followUp && <FollowUp finding={row.finding} followed={followed} followUp={followUp} />}
            </td>
            {viewable && (
              <ViewCell
                title={row.finding.replaceAll("`", "")}
                finding={row.finding}
                view={row.view}
                onView={onView}
              />
            )}
          </tr>
        ))}
      </tbody>
    </table>
  );
}

/** What a For context finding can become, or what it became. #906. */
function FollowUp({
  finding,
  followed,
  followUp,
}: {
  finding: string;
  followed: readonly FollowedRow[];
  followUp: ConfidenceFollowUp;
}) {
  const queued = followed.some((row) => row.finding === finding && row.job !== undefined);
  const filed = followed.some((row) => row.finding === finding && row.issue !== undefined);
  if (!queued && !filed) {
    return (
      <div className="armada-confidence__follow">
        <SplitButton
          items={[{ label: "File an issue", onSelect: () => followUp.onFileIssue(finding) }]}
          disabled={followUp.disabled}
          onAction={() => followUp.onQueueAfter(finding)}
          menuLabel="More ways to follow up"
        >
          Queue after this lands
        </SplitButton>
      </div>
    );
  }
  return (
    <div className="armada-confidence__follow">
      <span className="armada-confidence__became">
        {queued ? "Job queued. It starts when this one lands." : null}
        {!filed ? null : followUp.onOpenIssue === undefined ? (
          "Issue filed."
        ) : (
          <Button size="sm" onClick={() => followUp.onOpenIssue?.(finding)}>
            Open the issue
          </Button>
        )}
      </span>
    </div>
  );
}

/** A reviewer writes a name in backticks. Every second piece between them is code. */
function withCode(text: string): ReactNode {
  return text.split("`").map((piece, at) =>
    at % 2 === 1 ? (
      <code key={at} className="armada-confidence__path">
        {piece}
      </code>
    ) : (
      piece
    ),
  );
}

function openedLine(because: OpenedBecause): ReactNode {
  return (
    <>
      {because.kind === "test_loosened" ? "A test was loosened: " : "A test was removed: "}
      <code className="armada-confidence__path">{because.name}</code>
    </>
  );
}

function areaCount(areas: readonly AreaRow[]): string {
  return areas.length === 1 ? "1 area" : `${areas.length} areas`;
}

function findingCount(findings: readonly FindingRow[]): string {
  return findings.length === 1 ? "1 finding" : `${findings.length} findings`;
}

function testsSummary(tests: TestsSection): string {
  const proved = tests.proves.reduce((sum, row) => sum + row.tests, 0);
  const parts = [`${proved} ${proved === 1 ? "test proves" : "tests prove"} it`];
  if (tests.changed.length > 0) parts.push(`${tests.changed.length} changed or removed`);
  if (tests.untested.length > 0) parts.push(`${tests.untested.length} untested`);
  return parts.join(" · ");
}
