import { connectedTo, PROTOCOL_VERSION, type Connection, type JobSummary } from "@armada/protocol";
import type { RepositorySummary, WorkflowSummary } from "@armada/protocol";
import { headOf, Shell, statementOf, SURFACE } from "@armada/shell";
import { Jobs } from "../../../Jobs";
import { ofPicked } from "../../../board";
import { CREATED_AT, repository } from "../../../fixtures/build/base";

/** The moment every figure on the Board is read at, so a story never moves. */
const NOW = Date.parse("2026-09-10T21:00:00Z");

/** A Fleet that answered, as Bridge holds one once the socket is up. */
export const CONNECTED: Connection = connectedTo(
  { protocolVersion: PROTOCOL_VERSION, pid: 4242, port: 7878, startedAt: CREATED_AT },
  1,
);

const noop = () => {};

/**
 * The app's Board, in the window Bridge draws it in: the shell's rail, its
 * status bar and its head, and the Board screen under them.
 *
 * **Not a drawing of the Board — the Board.** `Shell`, `headOf` and `Jobs` are
 * the three the app renders, called the way `App` calls them, so the head's
 * controls, the tabs, the count and every row come out of the app's own code.
 * What is made up is only the data, and it is typed against the wire.
 */
export function BoardFrom({
  jobs,
  workflows,
  connection = CONNECTED,
  now = NOW,
  repositories = [repository()],
  picked,
}: {
  jobs: readonly JobSummary[];
  workflows: readonly WorkflowSummary[];
  /** What Fleet serves. More than one, on All, names each row's repository. */
  repositories?: readonly RepositorySummary[];
  /** The rail's pick, by root. Absent is All repositories, where Bridge opens. */
  picked?: string;
  connection?: Connection;
  /** When the Board is read. A recording passes its own, or run times go negative. */
  now?: number;
}) {
  const statement = statementOf(connection, now, now);
  const live = connection.state === "connected";
  // `App`'s own reading: the Board's Jobs follow the pick, and the bar reads every Job.
  const pickedRepository = repositories.find((one) => one.root === picked) ?? null;
  const boardJobs = ofPicked(jobs, pickedRepository);
  const head = headOf({
    reading: false,
    composing: false,
    auditing: false,
    clearing: false,
    manifest: false,
    live,
    refreshing: false,
    onCloseComposer: noop,
    onCompose: noop,
    onCloseReports: noop,
    onReadReports: noop,
    onCloseWorktrees: noop,
    onReadWorktrees: noop,
    onOpenLimits: noop,
    onRefresh: noop,
    jobs: boardJobs,
    onClearTerminal: noop,
    onForgetTerminal: noop,
    sweeping: null,
  });
  return (
    <div style={{ height: "100vh", display: "flex", flexDirection: "column" }}>
      <Shell
        connection={connection}
        repositories={repositories}
        listed={live}
        scope={pickedRepository?.root ?? null}
        onScope={noop}
        onCompose={noop}
        onSearch={noop}
        boardJobs={boardJobs}
        stats={{
          rows: [
            { id: "approval", label: "Awaiting approval", value: 0 },
            { id: "review", label: "Needs review", value: 0 },
            { id: "escalated", label: "Escalated", value: 0 },
            { id: "jobs", label: "Jobs", value: jobs.length },
            { id: "drones", label: "Drones", value: "2 of 4" },
            { id: "manifest", label: "Manifest", value: "Current" },
          ],
          open: true,
          onOpenChange: noop,
        }}
        fleet={{ state: "running", label: "Running", detail: statement.detail, open: true, onOpenChange: noop }}
        title={head?.title}
        summary={head?.summary}
        actions={head?.actions}
        showing={SURFACE.board}
      >
        <div className="armada-screen__mounted">
          <Jobs
            jobs={boardJobs}
            stale={!live}
            now={now}
            workflows={workflows}
            served={live ? repositories : null}
            all={pickedRepository === null}
            disconnected={live ? null : statement.headline}
            selected={null}
            onOpen={noop}
            onKill={noop}
            onCompose={noop}
            onCopied={noop}
          />
        </div>
      </Shell>
    </div>
  );
}
