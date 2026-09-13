import type { ReactNode } from "react";
import { useState } from "react";
import { ChevronDown, ChevronRight, Eye } from "lucide-react";
import type {
  AreaRow,
  FindingRow,
  JobConfidence,
  OpenedBecause,
  TestsSection,
} from "@armada/protocol";

/**
 * Armada's review of a change, drawn above the verdict sheet at the gate. #903.
 *
 * **The verdict and Needs you are always open**, because they are what a person is here for.
 * Every other section folds to one summary line. Tests in the change opens itself when a test
 * was removed or loosened with no reason, under a callout naming it, with its row marked.
 */
export type ConfidenceSheetProps = {
  confidence: JobConfidence;
};

export function ConfidenceSheet({ confidence }: ConfidenceSheetProps) {
  const { says, reasons, areas, tests, needs_you, small_fixes, for_context } = confidence;
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
      {areas.length > 0 && (
        <Fold title="Shape of the change" summary={areaCount(areas)}>
          <Areas areas={areas} />
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
          <Findings findings={needs_you} marked />
        )}
      </div>
      {small_fixes.length > 0 && (
        <Fold title="Small fixes for a Drone" summary={findingCount(small_fixes)}>
          <Findings findings={small_fixes} />
        </Fold>
      )}
      {for_context.length > 0 && (
        <Fold title="For context" summary={findingCount(for_context)}>
          <Findings findings={for_context} />
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

function Areas({ areas }: { areas: readonly AreaRow[] }) {
  return (
    <table className="armada-confidence__table">
      <thead>
        <tr>
          <th scope="col">Area</th>
          <th scope="col">What changed</th>
          <th scope="col">Files</th>
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
        <table className="armada-confidence__table" aria-label="Not tested">
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

function Findings({ findings, marked }: { findings: readonly FindingRow[]; marked?: boolean }) {
  return (
    <table className="armada-confidence__table" {...(marked ? { "data-marked": "" } : {})}>
      <thead>
        <tr>
          <th scope="col">Finding</th>
          <th scope="col">Why it is here</th>
        </tr>
      </thead>
      <tbody>
        {findings.map((row) => (
          <tr key={row.finding}>
            <td>{withCode(row.finding)}</td>
            <td className="armada-confidence__muted">{withCode(row.why)}</td>
          </tr>
        ))}
      </tbody>
    </table>
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
