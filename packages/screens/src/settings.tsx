// Every setting a person can change on a running Job, and the header's way in.
//
// `JobSettings` in the component library draws the panel. This is what it is
// fed from the wire, what each control sends, and when a change is said to
// have taken — the three things the component cannot know.
//
// **The raise dialogs are the header's own**, `RaiseCap.tsx` and
// `RaiseTurnCap.tsx`, mounted here with their buttons off. The panel's rows are
// another way to open them and not a second way to raise.

import { JOB_LIFECYCLE, JobSettings, JobSettingsButton } from "@armada/components";
import type {
  JobDetail as JobWhole,
  JobSummary,
  ModelChoices,
  WhenBlocked,
  WhenRefused,
} from "@armada/protocol";
import { useState, type ReactNode } from "react";

import {
  WHEN_BLOCKED,
  WHEN_BLOCKED_LABEL,
  WHEN_BLOCKED_MEANS,
  WHEN_REFUSED,
  WHEN_REFUSED_LABEL,
  WHEN_REFUSED_MEANS,
} from "./copy";
import { money } from "./facts";
import { cap, RaiseCapControl } from "./RaiseCap";
import { RaiseTurnCapControl } from "./RaiseTurnCap";

/** How a new Job meets a command it was not given. The count reads against it. */
const STARTS_AT: WhenBlocked = "ask_me";

/** How a new Job meets a judge criterion that refuses. The count reads against it. */
const REFUSED_STARTS_AT: WhenRefused = "per_criterion";

/**
 * Whether this Job has settings to change.
 *
 * **Not where Fleet sent no `when_blocked`**, which is a Fleet older than 10.7:
 * a panel there would draw a default Fleet never said it holds. **Not on a Job
 * that is over**, read off the lifecycle as `Acts` reads it — no Drone will
 * reach for anything again, and a setting nothing reads changes nothing.
 */
export function offersSettings(job: JobSummary, whole: JobWhole | null): boolean {
  return whole?.when_blocked !== undefined && JOB_LIFECYCLE[job.status]?.terminal === false;
}

/**
 * How many settings differ from how a Job starts: a choice other than *Ask me
 * first*, a chosen model, and **one for each command allowed** — each is
 * a separate thing a person did to this Job, and two allows read as one change
 * would undercount what somebody has to go and read.
 *
 * The caps are not counted. Nothing served says what a Job started with, so a
 * raised cap and a manifest's own larger one read the same.
 */
export function changedOf(whole: JobWhole | null): number {
  if (whole === null) return 0;
  const blocked = whole.when_blocked !== undefined && whole.when_blocked !== STARTS_AT ? 1 : 0;
  const refused =
    whole.when_refused !== undefined && whole.when_refused !== REFUSED_STARTS_AT ? 1 : 0;
  const model = whole.model_override === undefined ? 0 : 1;
  return blocked + refused + model + (whole.allowed_commands?.length ?? 0);
}

/** The header's way in, where the Job has settings. Nothing where it has none. */
export function settingsButtonOf(
  job: JobSummary,
  whole: JobWhole | null,
  onOpen: () => void,
): ReactNode {
  if (!offersSettings(job, whole)) return null;
  return <JobSettingsButton changed={changedOf(whole)} onOpen={onOpen} />;
}

/** What the panel sends. Each is live, and none restarts anything. */
export type SettingsCalls = {
  onSetWhenBlocked: (jobId: string, whenBlocked: WhenBlocked) => void;
  onSetWhenRefused: (jobId: string, whenRefused: WhenRefused) => void;
  onSetModel: (jobId: string, model: string | null) => void;
  onRemoveAllowedCommand: (jobId: string, run: string) => void;
  onRaiseCap: (jobId: string, costCapMicros: number) => void;
  onRaiseTurnCap: (jobId: string, turnCap: number) => void;
};

export type SettingsSheetProps = SettingsCalls & {
  job: JobSummary;
  whole: JobWhole | null;
  /** What `list_models` offers. `null` until it has been read. */
  models: ModelChoices | null;
  stale: boolean;
  acting: boolean;
  floor: boolean;
  onClose: () => void;
};

/** A row a change was sent from. */
type Row = "cost" | "turns" | "model" | "blocked" | "refused" | "allowed";

/** What a row says once its change took, and how to tell that it did. */
type Told = { row: Row; says: ReactNode; took: (whole: JobWhole) => boolean };

/**
 * The panel, fed.
 *
 * **A change is said to have taken when the detail shows it**, not when it was
 * pressed. Fleet answers with the Job's summary and main re-reads the detail,
 * so the new value arrives a round trip later — and a line that said *Changed*
 * on the press would say it over a refusal too. What is held is what was sent
 * and the test for it; the line shows once the reading passes.
 *
 * **`Esc` belongs to a raise dialog while one is up.** The sheet catches the key
 * first and stops it, so without this a person closing the dialog would lose
 * the whole panel with it.
 */
export function SettingsSheet({
  job,
  whole,
  models,
  stale,
  acting,
  floor,
  onClose,
  onSetWhenBlocked,
  onSetWhenRefused,
  onSetModel,
  onRemoveAllowedCommand,
  onRaiseCap,
  onRaiseTurnCap,
}: SettingsSheetProps) {
  const [raising, setRaising] = useState<"cost" | "turns" | null>(null);
  const [told, setTold] = useState<Told[]>([]);

  const current = whole?.when_blocked;
  const currentRefused = whole?.when_refused;
  if (whole === null || current === undefined || !offersSettings(job, whole)) return null;

  const tell = (row: Row, says: ReactNode, took: (next: JobWhole) => boolean) =>
    setTold((was) => [...was.filter((one) => one.row !== row), { row, says, took }]);
  const saidOf = (row: Row): ReactNode => {
    const one = told.find((each) => each.row === row);
    return one !== undefined && one.took(whole) ? one.says : undefined;
  };

  const off = stale || acting;
  const spend = whole.spend;
  return (
    <>
      <JobSettings
        open
        floor={floor}
        jobId={job.handle}
        costCap={
          spend === undefined
            ? undefined
            : {
                cap: cap(spend.cost_cap_micros),
                used: money(spend.cost_micros),
                onRaise: () => setRaising("cost"),
                said: saidOf("cost"),
              }
        }
        turnCap={
          spend === undefined
            ? undefined
            : {
                cap: String(spend.turn_cap),
                used: String(spend.turns),
                onRaise: () => setRaising("turns"),
                said: saidOf("turns"),
              }
        }
        models={models?.models ?? []}
        model={whole.model_override ?? null}
        modelSaid={saidOf("model")}
        choices={WHEN_BLOCKED.map((value) => ({
          value,
          label: WHEN_BLOCKED_LABEL[value],
          means: WHEN_BLOCKED_MEANS[value],
        }))}
        whenBlocked={current}
        whenBlockedSaid={saidOf("blocked")}
        judgeChoices={
          currentRefused === undefined
            ? undefined
            : WHEN_REFUSED.map((value) => ({
                value,
                label: WHEN_REFUSED_LABEL[value],
                means: WHEN_REFUSED_MEANS[value],
              }))
        }
        whenRefused={currentRefused}
        whenRefusedSaid={saidOf("refused")}
        allowed={whole.allowed_commands ?? []}
        allowedSaid={saidOf("allowed")}
        disabled={off}
        disabledNote={stale ? NOT_LIVE : acting ? SENDING : undefined}
        onWhenBlocked={(chose) => {
          tell("blocked", BLOCKED_TOOK, (next) => next.when_blocked === chose);
          onSetWhenBlocked(job.id, chose);
        }}
        onWhenRefused={(chose) => {
          tell("refused", REFUSED_TOOK, (next) => next.when_refused === chose);
          onSetWhenRefused(job.id, chose);
        }}
        onModel={(chose) => {
          tell("model", modelTook(chose), (next) => (next.model_override ?? null) === chose);
          onSetModel(job.id, chose);
        }}
        onRemove={(run) => {
          tell("allowed", REMOVED, (next) => !(next.allowed_commands ?? []).some((row) => row.run === run));
          onRemoveAllowedCommand(job.id, run);
        }}
        onClose={raising === null ? onClose : () => setRaising(null)}
      />
      {spend === undefined ? null : (
        <>
          <RaiseCapControl
            trigger={false}
            jobId={job.id}
            spend={spend}
            disabled={off}
            open={raising === "cost" && !stale}
            onOpen={(up) => setRaising(up ? "cost" : null)}
            onRaise={(jobId, micros) => {
              tell("cost", raisedTo(cap(micros)), (next) => next.spend?.cost_cap_micros === micros);
              onRaiseCap(jobId, micros);
            }}
          />
          <RaiseTurnCapControl
            trigger={false}
            jobId={job.id}
            spend={spend}
            disabled={off}
            open={raising === "turns" && !stale}
            onOpen={(up) => setRaising(up ? "turns" : null)}
            onRaise={(jobId, turns) => {
              tell("turns", raisedTo(`${turns} turns`), (next) => next.spend?.turn_cap === turns);
              onRaiseTurnCap(jobId, turns);
            }}
          />
        </>
      )}
    </>
  );
}

/** Why every control is off, where the reading is not live. */
const NOT_LIVE = "This job is not live, so nothing can be changed.";

/** Why they are off, where something sent to this Job has not come back. */
const SENDING = "Something sent to this job is still on its way to Fleet.";

/** A new choice for a command the drone was not given, once it took. */
const BLOCKED_TOOK = "Changed. Applies the next time it reaches for a command.";

/** A new choice for how a judge refusal is met, once it took. */
const REFUSED_TOOK = "Changed. Applies the next time a criterion refuses.";

/** An allow taken back, once it was. The choice above is what answers it now. */
const REMOVED = "Removed. The next time it reaches for that command, the setting above decides.";

/** A model chosen or handed back, once it took. The name is `list_models`', so mono. */
function modelTook(model: string | null): ReactNode {
  if (model === null) return "Changed. The next step starts on the model its workflow gives it.";
  return (
    <>
      Changed. The next step starts on <span className="mono">{model}</span>.
    </>
  );
}

/** A ceiling raised, once it was. The figure is the one the dialog sent. */
function raisedTo(figure: string): ReactNode {
  return (
    <>
      Raised to <span className="mono">{figure}</span>. Applies now.
    </>
  );
}
