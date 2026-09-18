import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { Button } from "../../primitives/Button/Button";
import { Dialog } from "../../primitives/Dialog/Dialog";
import { FactChip, type FactChipRun } from "../FactChip/FactChip";
import { CheckRuns, type CheckRun } from "../CheckRuns/CheckRuns";
import { ConsoleOutput, type ConsoleOutputProps } from "../ConsoleOutput/ConsoleOutput";
import { ChangedFiles, type ChangedFile } from "../ChangedFiles/ChangedFiles";
import { RunDiffSheet, type RunDiffSheetProps } from "../RunDiffSheet/RunDiffSheet";

/**
 * The Manifest surface's runner — Journey 9's *Running one*, in the main
 * checkout.
 *
 * # Why this is not `RunSheet` widened
 *
 * **A sheet is a thing you dismiss back to what you were doing; this surface
 * is where you were going.** That is the whole of the difference in shape, and
 * it is the reason this is a sibling of `RunSheet` rather than a mode on it:
 * the sheet is built on `Sheet`, closes on `Esc`, and sits over a Job. This
 * fills the panel.
 *
 * The difference in *content* is larger, and it comes from the wire rather
 * than from taste. `CheckoutRunSheet` is not `RunSheet` with the id left out —
 * it drops everything only a Job has:
 *
 * | Not here | Because |
 * |---|---|
 * | The narrow control | There is no branch and no base, so nothing measures a diff to narrow to. `StartCheckoutRun` carries a name and nothing else. |
 * | *Run the worktree's version* | Nothing is frozen. This is the file Fleet holds. |
 * | The Drone notice | No Drone works in the main checkout. |
 * | *Where* | A run goes in the tree as it is on disk, and a throwaway copy would be a worktree by another name. |
 *
 * And two things are here that the sheet has no need of:
 *
 * **A destructive Command confirms once, naming the command.** The flag says
 * nothing in a worktree about to be thrown away and everything in the tree a
 * person is working in. The confirmation is Bridge's — Fleet does not gate it
 * — and it is a confirmation rather than an approval, because the person
 * pressing is the one who asked.
 *
 * **Undo confirms, naming what it would discard.** The snapshot restores the
 * tree as it was just before the run, and this tree holds the owner's own
 * uncommitted work — which no Job's worktree ever does.
 *
 * # Job colours, and still no verdict
 *
 * No run from this surface is a verdict: nothing writes Evidence, no Check
 * gets a stored pass or fail, and nothing reaches the rail or the Board — a
 * remembered verdict here would be the path around verification that v1
 * proved becomes the default path. **Its result still reads in Job colours**,
 * as on the sheet: a `FactChip` hued through `--run-*`. How it looks, not
 * what it counts for.
 *
 * **Data in, callbacks out.** No protocol type and no fetch: a caller resolves
 * the Manifest, starts the run and streams the output.
 */

export type RunPageGroupKind = "setup" | "checks" | "commands";

export type RunPageEntry = {
  id: string;
  /** The Check or Command's own name in the Manifest. Sans, sentence case. */
  name: ReactNode;
  /** Its `run` line — or a server's `serve` line — exactly as declared. Mono. */
  run: string;
  /** A plain sentence, never a bare tag — `Runs fmt first.` */
  note?: ReactNode;
  /**
   * The Manifest declares this Command destructive.
   *
   * **The one field on this page the sheet's own row has no use for.** It is a
   * judgement a person wrote into the file rather than something Scan can
   * observe, and this is the surface where it carries information: the run
   * happens in the tree you are working in.
   */
  destructive?: boolean;
  /**
   * The line this entry declares names something the checkout no longer has —
   * drift's `gone`, joined to this row by name. A word in caution, never red:
   * a drifted file is behind rather than broken.
   */
  drifted?: boolean;
  /**
   * How this entry last ran in this checkout, and when. **In Job colours and
   * still not a verdict**, `Earlier runs`' own rule: how it looks, not what it
   * counts for. Absent where nothing kept says it has run.
   */
  last?: {
    /** The code, short — `exit 2`. `whole` carries what it expected. */
    result: ReactNode;
    whole?: string;
    outcome?: FactChipRun;
    /** The wall clock, formatted by the caller. */
    at: ReactNode;
  };
};

export type RunPageGroup = {
  kind: RunPageGroupKind;
  /** `Setup`, `Checks`, `Commands` — the vocabulary, never invented here. */
  label: string;
  /** A sentence about the group as a whole — Setup's names the declared seed and how warm it is. */
  says?: ReactNode;
  entries: RunPageEntry[];
};

/**
 * One rule a person always-allowed for this repository. **Fleet's own table,
 * since protocol 13.5 — never `armada.yml`.** #836 moved it there because a
 * command declared destructive stays an absolute boundary, and a boundary a
 * gate could be argued past by reading its own commit is not one.
 */
export type RunPageAlwaysAllowedRow = { run: string };

export type RunPagePastRun = {
  id: string;
  name: ReactNode;
  /** `exit 0 (expects 0)`, in a chip hued by `outcome`. */
  result: ReactNode;
  /**
   * How it ended, for its hue. `passed` is the code it expected; `failed` is
   * any other ending; `stopped` is a person pressing Stop, which takes killed's
   * hue because a run somebody ended is not a failure.
   */
  outcome?: FactChipRun;
  time: ReactNode;
  duration: ReactNode;
  onOpen?: () => void;
};

/** The last run's own line, in Job colours, and it says how it ended in words. */
export type RunPageResult = {
  name: ReactNode;
  /** Absent where there was no code — a signal, a spawn that failed. */
  exitCode?: number;
  expected: number;
  /** How it ended, in a sentence, for the case with no code. */
  ended: ReactNode;
  duration: ReactNode;
  /** As `RunPagePastRun`'s. */
  outcome?: FactChipRun;
};

/** An address a server offers, handed to a callback and never an `<a href>`. */
export type RunPageServerLink = { url: string; name?: ReactNode };

/** `RunSheet`'s three phases, unchanged: a server is a server either side. */
export type RunPageServerStatus =
  | { phase: "starting" }
  | {
      phase: "serving";
      address: ReactNode;
      uptime: ReactNode;
      links: RunPageServerLink[];
      startedByDrone?: boolean;
    }
  | {
      phase: "exited";
      /** Absent where it ended on a signal, which is every stop somebody pressed. */
      exitCode?: number;
      /** Somebody stopped it. `false` or absent: it stopped on its own. */
      stopped?: boolean;
    };

export type RunPageProps = {
  /** When `armada.yml` was last changed by a commit, as a sentence. */
  manifestEditedAt?: ReactNode;

  groups: RunPageGroup[];
  /** Which entry the control and the panel below answer for. */
  selectedId?: string | null;
  onSelect?: (id: string) => void;

  /**
   * Every rule a person always-allowed for this repository, oldest first,
   * beside the Commands group they came from a Job's own row of. `undefined`
   * is not yet read, and draws "Reading." rather than an empty list — which
   * would say a person removed every rule there ever was.
   */
  alwaysAllowed?: RunPageAlwaysAllowedRow[];
  /**
   * Take one back. Every job against this repository stops being granted it
   * from the next spawn on. Absent draws no Remove — the Job settings panel
   * shows this same list read-only, with no capability to send this.
   */
  onRemoveAlwaysAllowed?: (run: string) => void;
  /**
   * Run the named entry. **Already confirmed** where the entry is destructive
   * — this component puts that dialog up itself and calls this after it.
   */
  onRun?: (id: string) => void;

  /** The run being read, live or opened from the list. */
  output?: ConsoleOutputProps;
  /**
   * Put the panel away.
   *
   * **It closes the panel, not the log.** Every run writes its output under
   * `./.armada`, one directory per run, so this is a view onto a file — which
   * is what makes dismissing it cost nothing, and what makes a toast the wrong
   * shape for it.
   */
  onDismiss?: () => void;
  /** Present while a run is in flight. The elapsed figure is the caller's own. */
  running?: { elapsed: ReactNode; onStop?: () => void };
  /** The last run's result line. */
  result?: RunPageResult;

  /** Earlier runs in this checkout, newest first. */
  runs?: RunPagePastRun[];

  /** The selected entry's server state, where its Command declares `serve`. */
  server?: RunPageServerStatus;
  onStopServer?: (id: string) => void;
  /** Reports a link's URL. Never navigates — the caller hands it to the OS. */
  onOpenLink?: (url: string) => void;

  /**
   * What the last run wrote, reading how, and putting it back.
   *
   * **The diff is the run's, not the checkout's.** The main checkout has no
   * base, so `get_checkout_run_diff` reads against the snapshot Fleet took just
   * before the run — never `HEAD`, which would show a person's own uncommitted
   * work as the run's. That is what makes *Open the diff* answerable here.
   */
  changed?: {
    files: ChangedFile[];
    /** Open this run's patch. A read: it changes nothing. */
    onOpenDiff?: () => void;
    /** Absent where Fleet cannot honour one, and on a run already undone. */
    onUndo?: () => void;
    /**
     * The run was undone, as a sentence. **The files and the diff stay**: Undo
     * keeps the snapshot, so what the run did can still be read.
     */
    undone?: ReactNode;
    /** Why what the run changed could not be read. Drawn instead of the list. */
    unreadable?: ReactNode;
  };
  /**
   * The run's patch, on a trailing sheet over this page. Present while it is
   * open; the page is its containing block.
   */
  diff?: Omit<RunDiffSheetProps, "open">;
};

export function RunPage({
  manifestEditedAt,
  groups,
  selectedId = null,
  onSelect,
  onRun,
  alwaysAllowed,
  onRemoveAlwaysAllowed,
  output,
  onDismiss,
  running,
  result,
  runs = [],
  server,
  onStopServer,
  onOpenLink,
  changed,
  diff,
}: RunPageProps) {
  const selected =
    groups.flatMap((group) => group.entries).find((entry) => entry.id === selectedId) ?? null;
  /** The destructive Command waiting on its one confirmation. */
  const [confirmingRun, setConfirmingRun] = useState<RunPageEntry | null>(null);
  /** Whether Undo is waiting on its own. */
  const [confirmingUndo, setConfirmingUndo] = useState(false);

  // A dialog that outlives what it names is a dialog that confirms the wrong
  // thing. Both are cleared when the selection moves or the changed files are
  // replaced by the next run's.
  //
  // **Keyed on the files, not on `changed`.** A caller builds `changed` afresh
  // on every render, and the app renders every second to move its clock — so
  // keyed on the object, the Undo dialog closed itself within a second of
  // opening. The list is the run's own and only moves when the runs are read.
  useEffect(() => setConfirmingRun(null), [selectedId]);
  useEffect(() => setConfirmingUndo(false), [changed?.files]);

  function run(entry: RunPageEntry): void {
    if (entry.destructive === true) {
      setConfirmingRun(entry);
      return;
    }
    onRun?.(entry.id);
  }

  const panel = output !== undefined || result !== undefined || changed !== undefined;

  return (
    <div className="armada-run-page">
      <div className="armada-run-page__rail">
        {manifestEditedAt === undefined ? null : (
          <p className="armada-run-page__edited">{manifestEditedAt}</p>
        )}
        {groups.map((group) => (
          <RunPageGroupList
            key={group.kind}
            group={group}
            selectedId={selectedId}
            onSelect={onSelect}
          />
        ))}
        {alwaysAllowed === undefined ? null : (
          <RunPageAlwaysAllowedList rows={alwaysAllowed} onRemove={onRemoveAlwaysAllowed} />
        )}
      </div>

      <div className="armada-run-page__main">
        {/* A server hides the resolved command and Run while it is starting or
            serving — pressing Run again answers a question nobody is asking.
            Exited is the exception: a server that stopped on its own is
            started the same way any Command is. */}
        {server !== undefined && server.phase !== "exited" ? null : (
          <RunPageControl entry={selected} onRun={run} />
        )}

        {server === undefined || selected === null ? null : (
          <RunPageServer
            id={selected.id}
            status={server}
            onStop={onStopServer}
            onOpenLink={onOpenLink}
          />
        )}

        {running === undefined ? null : (
          <div className="armada-run-page__running">
            <span className="armada-run-page__elapsed">{running.elapsed}</span>
            {running.onStop === undefined ? null : (
              <Button variant="secondary" size="sm" onClick={running.onStop}>
                Stop
              </Button>
            )}
          </div>
        )}

        {!panel ? (
          <p className="armada-run-page__nothing">
            Nothing has run here yet. Output stays on this page until you dismiss it.
          </p>
        ) : (
          <div
            className={
              output === undefined
                ? "armada-run-page__panel"
                : "armada-run-page__panel armada-run-page__panel--reading"
            }
          >
            <div className="armada-run-page__panel-head">
              <span className="armada-run-page__panel-title">Output</span>
              {onDismiss === undefined ? null : (
                <Button variant="ghost" size="sm" onClick={onDismiss}>
                  Dismiss
                </Button>
              )}
            </div>

            {output === undefined ? null : <RunPageOutput output={output} />}

            {result === undefined ? null : (
              <p className="armada-run-page__result">
                {result.name}{" "}
                <FactChip run={result.outcome}>
                  {result.exitCode === undefined
                    ? result.ended
                    : `exit ${result.exitCode} (expects ${result.expected})`}
                </FactChip>{" "}
                {result.duration}
              </p>
            )}

            {changed === undefined ? null : (
              <div className="armada-run-page__changed">
                {changed.unreadable === undefined ? (
                  <ChangedFiles files={changed.files} emptyNote="This run changed nothing." />
                ) : (
                  <p className="armada-run-page__changed-note">{changed.unreadable}</p>
                )}
                {/* Unhued, like everything but a result: undone is a fact about the
                    checkout since, not a result. */}
                {changed.undone === undefined ? null : (
                  <p className="armada-run-page__changed-note">{changed.undone}</p>
                )}
                {changed.onOpenDiff === undefined && changed.onUndo === undefined ? null : (
                  <div className="armada-run-page__changed-acts">
                    {changed.onOpenDiff === undefined ? null : (
                      <Button variant="secondary" size="sm" onClick={changed.onOpenDiff}>
                        Open the diff
                      </Button>
                    )}
                    {changed.onUndo === undefined ? null : (
                      <Button
                        variant="secondary"
                        size="sm"
                        onClick={() => setConfirmingUndo(true)}
                      >
                        Undo this run
                      </Button>
                    )}
                  </div>
                )}
              </div>
            )}
          </div>
        )}

        {runs.length === 0 ? null : (
          <CheckRuns
            label="Earlier runs"
            rows={runs.map(
              (past): CheckRun => ({
                id: past.id,
                says: past.name,
                identifier: past.time,
                // The chip, not CheckRuns' own `named`: that hue is a gate's
                // result, and a rehearsal must not read as one.
                result: (
                  <FactChip run={past.outcome}>
                    {past.result}
                    {" · "}
                    {past.duration}
                  </FactChip>
                ),
                output: past.onOpen === undefined ? undefined : "log",
              }),
            )}
            onOpen={(id) => runs.find((past) => past.id === id)?.onOpen?.()}
            openSaid="Click to open this run's log"
          />
        )}
      </div>

      {diff === undefined ? null : <RunDiffSheet open {...diff} />}

      {/* Once, naming the command. It states what happens and where, which is
          the rule every confirmation in Bridge keeps — never "are you sure". */}
      <Dialog
        open={confirmingRun !== null}
        title="Run a Command that writes?"
        confirmLabel="Run it"
        onCancel={() => setConfirmingRun(null)}
        onConfirm={() => {
          const entry = confirmingRun;
          setConfirmingRun(null);
          if (entry !== null) onRun?.(entry.id);
        }}
      >
        <p>
          The Manifest declares <strong>{confirmingRun?.name}</strong> destructive, so it is
          expected to change files.
        </p>
        <p className="mono">{confirmingRun?.run}</p>
        <p>
          It runs in this checkout as it is on disk — the tree you are working in, with
          whatever you have not committed. Nothing is copied first. After it, this page lists
          what it changed and offers to put it back.
        </p>
      </Dialog>

      {/* Undo names what it would discard, file by file. The list is short by
          construction — it is one run's own footprint — and every path on it is
          about to be overwritten from a snapshot. */}
      <Dialog
        open={confirmingUndo}
        title="Put these files back as they were?"
        confirmLabel="Undo the run"
        width="wide"
        onCancel={() => setConfirmingUndo(false)}
        onConfirm={() => {
          setConfirmingUndo(false);
          changed?.onUndo?.();
        }}
      >
        <p>
          The snapshot was taken in this checkout immediately before the run. Restoring it
          discards every change made since — the run's, and anything you or anything else
          wrote into these paths after it.
        </p>
        <ul>
          {(changed?.files ?? []).map((file) => (
            <li key={file.path} className="mono">
              {file.path}
            </li>
          ))}
        </ul>
      </Dialog>
    </div>
  );
}

function RunPageGroupList({
  group,
  selectedId,
  onSelect,
}: {
  group: RunPageGroup;
  selectedId: string | null;
  onSelect?: (id: string) => void;
}) {
  return (
    <div className="armada-run-page__group">
      <span className="armada-run-page__group-label">
        {group.label}
        {/* How many the file declares. The rail's own count, machine-derived
            and mono, so a group reads as a group rather than a rule with rows
            under it. Never on an empty one: the sentence below says it. */}
        {group.entries.length === 0 ? null : (
          <span className="armada-run-page__group-count">{group.entries.length}</span>
        )}
      </span>
      {group.says === undefined ? null : <p className="armada-run-page__group-says">{group.says}</p>}
      {group.entries.length === 0 ? (
        <p className="armada-run-page__group-empty">This Manifest declares none.</p>
      ) : (
        <ul className="armada-run-page__entries">
          {group.entries.map((entry) => (
            <li className="armada-run-page__entry" key={entry.id}>
              <button
                type="button"
                className="armada-run-page__entry-select"
                aria-current={entry.id === selectedId ? "true" : undefined}
                onClick={() => onSelect?.(entry.id)}
              >
                <span className="armada-run-page__entry-head">
                  <span className="armada-run-page__entry-name">{entry.name}</span>
                  {entry.drifted === true ? (
                    <span className="armada-run-page__entry-gone">gone</span>
                  ) : null}
                </span>
                <span className="armada-run-page__entry-run">{entry.run}</span>
                <RunPageEntryFacts entry={entry} />
              </button>
              {entry.note === undefined ? null : (
                <p className="armada-run-page__entry-note">{entry.note}</p>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

/**
 * What a row carries beyond its name and its line: how it last ran, and
 * whether running it writes.
 *
 * **What is already in memory, and nothing more.** The runs are the list
 * *Earlier runs* is drawn from and drift is the read the surface takes on
 * opening; whether the gate runs a Check a Drone skips, and the paths that
 * decide whether it applies at all, are not on the wire —
 * https://github.com/NickMele/armada/issues/1437.
 */
function RunPageEntryFacts({ entry }: { entry: RunPageEntry }) {
  if (entry.last === undefined && entry.destructive !== true) return null;
  return (
    <span className="armada-run-page__entry-facts">
      {entry.last === undefined ? null : (
        <>
          <FactChip run={entry.last.outcome} title={entry.last.whole}>
            {entry.last.result}
          </FactChip>
          <span className="armada-run-page__entry-at">{entry.last.at}</span>
        </>
      )}
      {/* The word the confirmation uses, so the row and the dialog agree. */}
      {entry.destructive === true ? (
        <span className="armada-run-page__entry-writes">writes</span>
      ) : null}
    </span>
  );
}

/**
 * Every rule a person always-allowed for this repository. **Beside the
 * Commands group it grew a Job's own row of a level up.** Fleet's own table
 * since 13.5, so this list is never `armada.yml` — no drone reads it and no
 * commit sits behind it.
 */
function RunPageAlwaysAllowedList({
  rows,
  onRemove,
}: {
  rows: RunPageAlwaysAllowedRow[];
  onRemove?: (run: string) => void;
}) {
  return (
    <div className="armada-run-page__group">
      <span className="armada-run-page__group-label">Always allowed</span>
      {rows.length === 0 ? (
        <p className="armada-run-page__group-empty">Nothing always allowed yet.</p>
      ) : (
        <ul className="armada-run-page__entries">
          {rows.map((row) => (
            <li className="armada-run-page__entry" key={row.run}>
              <span className="armada-run-page__allowed-run">{row.run}</span>
              {onRemove === undefined ? null : (
                <div className="armada-run-page__allowed-remove">
                  <Button
                    variant="secondary"
                    size="sm"
                    ground="sunken"
                    aria-label={`Remove ${row.run}`}
                    onClick={() => onRemove(row.run)}
                  >
                    Remove
                  </Button>
                </div>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

/**
 * What would run, and the one control that runs it.
 *
 * **No scope control, and its absence is the design.** There is no branch and
 * no base in the main checkout, so nothing can resolve a narrowed command —
 * and a *Changed* segment that always meant the whole tree would be a control
 * that lies.
 */
function RunPageControl({
  entry,
  onRun,
}: {
  entry: RunPageEntry | null;
  onRun: (entry: RunPageEntry) => void;
}) {
  if (entry === null) {
    return (
      <p className="armada-run-page__control-empty">
        Pick a Check or a Command from the list to run it in this checkout.
      </p>
    );
  }

  return (
    <div className="armada-run-page__control">
      <code className="armada-run-page__resolved">{entry.run}</code>
      <div className="armada-run-page__control-acts">
        {entry.destructive === true ? (
          <span className="armada-run-page__writes">
            Declared destructive — it confirms before it runs.
          </span>
        ) : null}
        <Button variant="primary" onClick={() => onRun(entry)}>
          Run
        </Button>
      </div>
    </div>
  );
}

/**
 * A server's own status. `RunSheet`'s three phases and its reasoning: the
 * control answers *what would run* and this answers *what is running*, and a
 * server is the one entry where both can be true at once.
 *
 * **A link hands its address to the system browser.** No surface in Bridge
 * navigates, which is why these are buttons reporting a URL rather than
 * anchors.
 */
function RunPageServer({
  id,
  status,
  onStop,
  onOpenLink,
}: {
  id: string;
  status: RunPageServerStatus;
  onStop?: (id: string) => void;
  onOpenLink?: (url: string) => void;
}) {
  if (status.phase === "starting") {
    return <p className="armada-run-page__server-status">Starting.</p>;
  }

  if (status.phase === "exited") {
    return (
      <p className="armada-run-page__server-status">
        {status.stopped === true ? "Stopped." : "Stopped on its own."}
        {status.exitCode === undefined ? null : (
          <>
            {" "}
            <FactChip run={status.stopped === true ? "stopped" : "failed"}>
              {`exit ${status.exitCode}`}
            </FactChip>
          </>
        )}
      </p>
    );
  }

  return (
    <div className="armada-run-page__server">
      <span className="armada-run-page__server-status">serving</span>
      <span className="armada-run-page__server-address">{status.address}</span>
      <span className="armada-run-page__server-uptime">{status.uptime}</span>
      {status.startedByDrone === true ? (
        <span className="armada-run-page__server-note">A Drone started this server.</span>
      ) : null}
      <span className="armada-run-page__server-links">
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
 * The output panel — `RunSheet`'s, and the same two rules, because the failure
 * it avoids is the same one on a page as in a sheet.
 *
 * **It scrolls inside itself**, so a thousand lines do not make the page a
 * thousand lines tall. **It follows the tail while streaming and stops the
 * moment a reader scrolls up** — a reader forty lines up reading a stack trace
 * must not be dragged back by the next line arriving.
 */
function RunPageOutput({ output }: { output: ConsoleOutputProps }) {
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
    <div className="armada-run-page__output" ref={ref} onScroll={onScroll}>
      <ConsoleOutput {...output} />
    </div>
  );
}
