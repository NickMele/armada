import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { Sheet } from "../../primitives/Sheet/Sheet";
import { Button } from "../../primitives/Button/Button";
import { Alert } from "../../primitives/Alert/Alert";
import { Radio, RadioGroup } from "../../primitives/Radio/Radio";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { FactChip } from "../FactChip/FactChip";
import { CheckRuns, type CheckRun } from "../CheckRuns/CheckRuns";
import { ConsoleOutput, type ConsoleOutputProps } from "../ConsoleOutput/ConsoleOutput";
import { ChangedFiles, type ChangedFile } from "../ChangedFiles/ChangedFiles";

/**
 * The run sheet — Journey 9's *Running one inside a Job*, on the layer that
 * can hold it.
 *
 * **It rehearses the Manifest the Job froze, in that Job's own worktree.**
 * There is no throwaway copy and no "Where" control — a run from here goes in
 * the Job's own tree, because a throwaway copy waits on
 * `runedit-adhoc-run-location` and a copy control was deferred outright.
 *
 * **A run from this sheet writes no Evidence and moves nothing on the Job.**
 * The result line and the run list both stay unhued for that reason: a run
 * from here is a rehearsal, not a verdict, and the Job state machine is the
 * only thing status colour ever maps onto.
 *
 * **Data in, callbacks out.** This component holds no protocol type and no
 * fetch — a caller resolves the Manifest, starts the run and streams the
 * output; this only draws what it is handed and reports what was pressed.
 *
 * **Built on `Sheet`, `ConsoleOutput`, `CheckRuns`, `ChangedFiles`.** The rail
 * is new — Setup, Checks and Commands have no existing composition, since
 * nothing before this drew a Manifest as something a person runs rather than
 * something Fleet reads.
 */

export type RunSheetGroupKind = "setup" | "checks" | "commands";

export type RunSheetEntry = {
  id: string;
  /** The Check or Command's own name in the Manifest. Sans, sentence case. */
  name: ReactNode;
  /** Its `run` line, exactly as the Manifest spells it. Mono. */
  run: string;
  /**
   * A plain sentence, never a bare tag — `Runs fmt first.` where `requires`
   * names a prerequisite Command.
   *
   * **Never the reason a `when` skips this Check for this Job.** That was a
   * long prose sentence naming every path pattern, and Nick's own reading —
   * this panel is for running things by hand — is that a row here is read at
   * a glance, not audited for why the gate passed it over.
   */
  note?: ReactNode;
  /** Present where the Check declares `narrow`. The command a narrowed run resolves to. */
  narrowRun?: string;
  /**
   * Whether this entry is currently narrowed. Held by the caller: narrowing
   * is a per-selection choice **Run the whole tree instead** flips, not a
   * fact about the Check.
   */
  narrowed?: boolean;
};

export type RunSheetGroup = {
  kind: RunSheetGroupKind;
  /** `Setup`, `Checks`, `Commands` — the vocabulary, never invented here. */
  label: string;
  entries: RunSheetEntry[];
};

export type RunSheetPastRun = {
  id: string;
  name: ReactNode;
  /** `exit 0 (expects 0)`. Unhued — a rehearsal is not a verdict. */
  result: ReactNode;
  time: ReactNode;
  duration: ReactNode;
  onOpen?: () => void;
};

export type RunSheetResult = {
  name: ReactNode;
  exitCode: number;
  expected: number;
  duration: ReactNode;
};

/**
 * An address a server offers, as a button rather than a link — pressing one
 * hands the URL to a callback, never an `<a href>` default, because no
 * surface in Bridge navigates.
 */
export type RunSheetServerLink = {
  url: string;
  /** The button's label. Absent draws the URL itself. */
  name?: ReactNode;
};

/**
 * What a Command declaring `serve` is doing, for the currently selected
 * entry. Three phases, and only `serving` carries links — pressing one before
 * `ready` passes would open an address nothing is answering on yet.
 *
 * **Nothing here is hued.** A server's activity is not a Job state, and
 * `tokens/status.css` declares no value for it — so `exited`, which reads
 * closest to a failure, stays as neutral as `starting` and `serving`. Words
 * carry the reading: *serving*, *stopped on its own*.
 */
export type RunSheetServerStatus =
  | { phase: "starting" }
  | {
      phase: "serving";
      /** `localhost:41207` — where it is, distinct from what it names. */
      address: ReactNode;
      uptime: ReactNode;
      links: RunSheetServerLink[];
      /** A Drone started this server, through Fleet's MCP tool. */
      startedByDrone?: boolean;
    }
  | {
      phase: "exited";
      /** A server that exits on its own has failed, whatever the code. */
      exitCode: number;
    };

export type RunSheetProps = {
  open: boolean;
  onClose?: () => void;
  /** The window is at `--window-floor`. */
  floor?: boolean;

  /** The Job, in the subtitle. */
  jobName?: ReactNode;
  /** When `armada.yml` was last edited before the Job froze it. */
  manifestEditedAt?: ReactNode;

  groups: RunSheetGroup[];
  /** Which entry the control and output below answer for. */
  selectedId?: string | null;
  onSelect?: (id: string) => void;
  /** Flips narrowing for the named entry — **Run the whole tree instead**. */
  onToggleNarrow?: (id: string) => void;
  /** Reports the selected entry and whether it ran narrowed. */
  onRun?: (run: { id: string; narrowed: boolean }) => void;

  /**
   * A Drone is working in this Job's worktree. Draws a notice, and — this is
   * the rule rather than a suggestion — suppresses **Undo this run**
   * regardless of whether `changed.onUndo` is supplied, since Fleet commits a
   * Job's work only at delivery and a plain discard would take it too.
   */
  droneWorking?: boolean;
  /** The worktree's own `armada.yml` differs from the one the Job froze. */
  manifestDiffers?: { onUseWorktreeVersion?: () => void };

  /** The active or most recently finished run's output. Absent where nothing has run yet. */
  output?: ConsoleOutputProps;
  /** Present while a run is in flight. Elapsed is the caller's own figure — this draws no arithmetic. */
  running?: { elapsed: ReactNode; onStop?: () => void };
  /** The last run's result line. Unhued: a run from here carries no verdict. */
  result?: RunSheetResult;

  /** Earlier runs from this sheet. */
  runs?: RunSheetPastRun[];

  /**
   * The selected entry's server state, where its Command declares `serve`.
   * Present through all three phases — the resolved command and **Run**
   * still show alongside it once `exited`, since a server that stopped on
   * its own is started the same way any Command is.
   */
  server?: RunSheetServerStatus;
  /** Reports which server was stopped. */
  onStopServer?: (id: string) => void;
  /** Reports a link's URL. Never navigates — the caller hands it to the OS. */
  onOpenLink?: (url: string) => void;

  /** What the last run wrote, where it wrote. */
  changed?: {
    files: ChangedFile[];
    onOpenDiff?: () => void;
    onUndo?: () => void;
  };
};

export function RunSheet({
  open,
  onClose,
  floor = false,
  jobName,
  manifestEditedAt,
  groups,
  selectedId = null,
  onSelect,
  onToggleNarrow,
  onRun,
  droneWorking = false,
  manifestDiffers,
  output,
  running,
  result,
  runs = [],
  changed,
  server,
  onStopServer,
  onOpenLink,
}: RunSheetProps) {
  const selected =
    groups.flatMap((group) => group.entries).find((entry) => entry.id === selectedId) ?? null;

  const showBands = droneWorking || manifestDiffers !== undefined;
  const bands = !showBands ? undefined : (
    <>
      {droneWorking ? (
        <Alert tone="caution" title="A Drone is working in this tree">
          This run shares the worktree and its build directory. Nothing locks.
        </Alert>
      ) : null}
      {manifestDiffers === undefined ? null : (
        <Alert
          tone="neutral"
          title="The worktree's armada.yml differs from the one this Job froze"
          action={
            manifestDiffers.onUseWorktreeVersion === undefined ? undefined : (
              <Button
                variant="ghost"
                size="sm"
                ground="sunken"
                onClick={manifestDiffers.onUseWorktreeVersion}
              >
                Run the worktree's version
              </Button>
            )
          }
        >
          The list below is the Job's frozen Manifest.
        </Alert>
      )}
    </>
  );

  return (
    <Sheet
      open={open}
      contained
      size="wide"
      floor={floor}
      title="Run"
      subtitle={
        jobName === undefined && manifestEditedAt === undefined ? undefined : (
          <>
            {jobName}
            {jobName !== undefined && manifestEditedAt !== undefined ? " · " : null}
            {manifestEditedAt}
          </>
        )
      }
      bands={bands}
      closeLabel="Close"
      closeBinding="Esc"
      bleed
      onClose={onClose}
    >
      <div className="armada-run-sheet">
        <div className="armada-run-sheet__rail">
          {groups.map((group) => (
            <RunSheetGroupList key={group.kind} group={group} selectedId={selectedId} onSelect={onSelect} />
          ))}
        </div>
        <div className="armada-run-sheet__main">
          {/* A server hides the resolved command and Run while it is starting
              or serving — pressing Run again answers a question that is not
              being asked. Exited is the exception: a server that stopped on
              its own is started the same way any Command is, so Run returns. */}
          {server !== undefined && server.phase !== "exited" ? null : (
            <RunSheetControl entry={selected} onToggleNarrow={onToggleNarrow} onRun={onRun} />
          )}

          {server === undefined || selected === null ? null : (
            <RunSheetServer
              id={selected.id}
              status={server}
              onStop={onStopServer}
              onOpenLink={onOpenLink}
            />
          )}

          {running === undefined ? null : (
            <div className="armada-run-sheet__running">
              <span className="armada-run-sheet__elapsed">{running.elapsed}</span>
              {running.onStop === undefined ? null : (
                <Button variant="secondary" size="sm" onClick={running.onStop}>
                  Stop
                </Button>
              )}
            </div>
          )}

          {output === undefined ? null : <RunSheetOutput output={output} />}

          {result === undefined ? null : (
            <p className="armada-run-sheet__result">
              {result.name}{" "}
              <FactChip>{`exit ${result.exitCode} (expects ${result.expected})`}</FactChip>{" "}
              {result.duration}
            </p>
          )}

          {changed === undefined ? null : (
            <div className="armada-run-sheet__changed">
              <ChangedFiles files={changed.files} emptyNote="This run changed nothing." />
              <div className="armada-run-sheet__changed-acts">
                {changed.onOpenDiff === undefined ? null : (
                  <Button variant="secondary" size="sm" onClick={changed.onOpenDiff}>
                    Open the diff
                  </Button>
                )}
                {/* Undo needs a snapshot taken before the run, and a Drone's work is
                    uncommitted until delivery — so this is never drawn while one is
                    working, whatever the caller passed for onUndo. */}
                {changed.onUndo === undefined || droneWorking ? null : (
                  <Button variant="secondary" size="sm" onClick={changed.onUndo}>
                    Undo this run
                  </Button>
                )}
              </div>
            </div>
          )}

          {runs.length === 0 ? null : (
            <CheckRuns
              label="Earlier runs"
              rows={runs.map(
                (run): CheckRun => ({
                  id: run.id,
                  says: run.name,
                  identifier: run.time,
                  result: (
                    <>
                      {run.result}
                      {" · "}
                      {run.duration}
                    </>
                  ),
                  output: run.onOpen === undefined ? undefined : "log",
                }),
              )}
              onOpen={(id) => runs.find((run) => run.id === id)?.onOpen?.()}
              openSaid="Click to open this run's log"
            />
          )}
        </div>
      </div>
    </Sheet>
  );
}

function RunSheetGroupList({
  group,
  selectedId,
  onSelect,
}: {
  group: RunSheetGroup;
  selectedId: string | null;
  onSelect?: (id: string) => void;
}) {
  return (
    <div className="armada-run-sheet__group">
      <span className="armada-run-sheet__group-label">{group.label}</span>
      <ul className="armada-run-sheet__entries">
        {group.entries.map((entry) => (
          <li className="armada-run-sheet__entry" key={entry.id}>
            <button
              type="button"
              className="armada-run-sheet__entry-select"
              aria-current={entry.id === selectedId ? "true" : undefined}
              onClick={() => onSelect?.(entry.id)}
            >
              <span className="armada-run-sheet__entry-name">{entry.name}</span>
              <span className="armada-run-sheet__entry-run">{entry.run}</span>
            </button>
            {entry.note === undefined ? null : (
              <p className="armada-run-sheet__entry-note">{entry.note}</p>
            )}
          </li>
        ))}
      </ul>
    </div>
  );
}

function RunSheetControl({
  entry,
  onToggleNarrow,
  onRun,
}: {
  entry: RunSheetEntry | null;
  onToggleNarrow?: (id: string) => void;
  onRun?: (run: { id: string; narrowed: boolean }) => void;
}) {
  if (entry === null) {
    return (
      <p className="armada-run-sheet__control-empty">
        Select a Check or Command from the list to run it here.
      </p>
    );
  }

  const narrowed = entry.narrowRun !== undefined && entry.narrowed === true;
  const command = narrowed ? entry.narrowRun! : entry.run;
  const scopeName = `armada-run-sheet-scope-${entry.id}`;

  return (
    <div className="armada-run-sheet__control">
      <code className="armada-run-sheet__resolved">{command}</code>
      <div className="armada-run-sheet__control-acts">
        {entry.narrowRun === undefined ? null : (
          <div className="armada-run-sheet__scope">
            <RadioGroup label="Run scope">
              <Radio
                name={scopeName}
                checked={narrowed}
                onChange={() => {
                  if (!narrowed) onToggleNarrow?.(entry.id);
                }}
              >
                <Tooltip label={<code>{entry.narrowRun}</code>}>Changed</Tooltip>
              </Radio>
              <Radio
                name={scopeName}
                checked={!narrowed}
                onChange={() => {
                  if (narrowed) onToggleNarrow?.(entry.id);
                }}
              >
                <Tooltip label={<code>{entry.run}</code>}>All</Tooltip>
              </Radio>
            </RadioGroup>
          </div>
        )}
        <Button variant="primary" onClick={() => onRun?.({ id: entry.id, narrowed })}>
          Run
        </Button>
      </div>
    </div>
  );
}

/**
 * A server's own status — starting, serving with its links and Stop, or
 * exited on its own. Beside `RunSheetControl` rather than inside it: the
 * control answers *what would run*, this answers *what is running*, and a
 * server is the one entry on this sheet where both can be true together.
 */
function RunSheetServer({
  id,
  status,
  onStop,
  onOpenLink,
}: {
  id: string;
  status: RunSheetServerStatus;
  onStop?: (id: string) => void;
  onOpenLink?: (url: string) => void;
}) {
  if (status.phase === "starting") {
    return <p className="armada-run-sheet__server-status">Starting.</p>;
  }

  if (status.phase === "exited") {
    return (
      <p className="armada-run-sheet__server-status">
        Stopped on its own. <FactChip>{`exit ${status.exitCode}`}</FactChip>
      </p>
    );
  }

  return (
    <div className="armada-run-sheet__server">
      <span className="armada-run-sheet__server-status">serving</span>
      <span className="armada-run-sheet__server-address">{status.address}</span>
      <span className="armada-run-sheet__server-uptime">{status.uptime}</span>
      {status.startedByDrone ? (
        <span className="armada-run-sheet__server-note">A Drone started this server.</span>
      ) : null}
      <span className="armada-run-sheet__server-links">
        {status.links.map((link) => (
          <Button
            key={link.url}
            variant="secondary"
            size="sm"
            onClick={() => onOpenLink?.(link.url)}
          >
            {link.name ?? link.url}
          </Button>
        ))}
      </span>
      {onStop === undefined ? null : (
        <Button variant="secondary" size="sm" onClick={() => onStop(id)}>
          Stop
        </Button>
      )}
    </div>
  );
}

/**
 * The output panel — fills whatever height is left in the main column and
 * scrolls inside itself, so the sheet never grows for it. A thousand lines
 * is the case this exists for: without a ceiling, the panel would simply be
 * a thousand lines tall and the sheet along with it.
 *
 * **Follows the tail while streaming, and stops the moment a reader scrolls
 * up.** `ConsoleOutput`'s own `following` prop is the caller's fact about
 * whether the process is still writing — a different question from whether
 * *this viewport* is pinned to the bottom, which is what this component
 * tracks. A reader forty lines up reading a stack trace must not be dragged
 * back to the tail by the next line arriving.
 */
function RunSheetOutput({ output }: { output: ConsoleOutputProps }) {
  const ref = useRef<HTMLDivElement>(null);
  const [followingTail, setFollowingTail] = useState(true);

  useEffect(() => {
    const el = ref.current;
    if (el === null || !followingTail) return;
    el.scrollTop = el.scrollHeight;
  }, [output.rows, followingTail]);

  const onScroll = useCallback(() => {
    const el = ref.current;
    if (el === null) return;
    // Within a line's reach of the bottom counts as "returned to it" —
    // fractional scroll positions rarely land on an exact equality.
    const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 24;
    setFollowingTail(atBottom);
  }, []);

  return (
    <div className="armada-run-sheet__output" ref={ref} onScroll={onScroll}>
      <ConsoleOutput {...output} />
    </div>
  );
}
