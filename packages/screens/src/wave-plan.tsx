// The wave on the Plan destination: every pass of the split, the Judge that
// read the live one, and one Job's inspector. `#1544`.
//
// **An earlier pass is history, not a second plan.** A loop return replaces
// `plan.md` whole (`.armada/workflows/epic.json`), so only the live round is
// drawn as the split and the rest are listed as what was tried.

import { Badge, HoldButton, Sheet } from "@armada/components";
import { JOB_STATUS } from "@armada/components/src/generated/vocabulary";
import { useState } from "react";

import type { StepDetail } from "@armada/protocol";

import { Eyebrow } from "./regions";
import type { WaveJobView, WaveView } from "./draft/wave";
import { WaveRegion, type WaveRegionProps } from "./tab-wave";
import { waveLandedSaid } from "./wave";

/** What dropping a Job from the wave does, said where a person is about to do it. */
const DROP_SAID =
  "The Job ends at killed, which is terminal and carries no verdict. Nothing resumes it, " +
  "anything its Drone wrote stays on its branch, and the rest of the wave carries on.";

export type WavePlanProps = WaveRegionProps & {
  /** The step that recorded the split, where one did. Its Judge is read off it. */
  planStep?: StepDetail;
  /** The window is at `--window-floor`, so the inspector goes flush. */
  floor: boolean;
  /**
   * Drop one Job from the wave, held rather than pressed.
   *
   * **`kill_job` on that Job, by the name this surface gives it.** `killed` is
   * the registry's own "cleared from the Board — an operator act, carrying no
   * verdict", which is exactly what dropping one is. No second act is minted.
   */
  onDropFromWave: (jobId: string) => void;
};

/** What the Judge made of the split, off the step that recorded it. */
function judgedSaid(step: StepDetail | undefined): string | undefined {
  if (step === undefined || step.judged.length === 0) return undefined;
  // `criterion_verdict_judge`: `met` or `not_met`. The refusals are what a
  // person reads the split's Judge for.
  const refused = step.judged.filter((one) => one.verdict === "not_met");
  if (refused.length === 0) {
    return `The Judge read the split and refused nothing — ${String(step.judged.length)} criteria.`;
  }
  return `The Judge refused ${String(refused.length)} of ${String(step.judged.length)} criteria on the split.`;
}

/** Every pass of the split, the live one first said to be live. */
function Rounds({ wave }: { wave: WaveView }) {
  if (wave.rounds.length === 0) return null;
  return (
    <section className="armada-detail-tab__region" aria-label="Every pass of the split">
      <Eyebrow>Every pass of the split</Eyebrow>
      <ul className="armada-wave__rounds">
        {wave.rounds.map((round) => (
          <li key={round.round} data-live={round.live || undefined}>
            <span className="armada-wave__round-at">Pass {round.round}</span>
            <span className="armada-wave__round-says">{round.says}</span>
            <span className="armada-wave__round-live">
              {round.live
                ? `${String(wave.jobs.filter((one) => one.round === round.round).length)} Jobs, the plan being run now`
                : "Replaced by the pass after it"}
            </span>
          </li>
        ))}
      </ul>
    </section>
  );
}

/** One Job of the wave, read whole, with the one act this surface offers. */
function JobInspector({
  job,
  waits,
  stale,
  acting,
  floor,
  onOpenJob,
  onDrop,
  onClose,
}: {
  job: WaveJobView;
  waits: readonly WaveJobView[];
  stale: boolean;
  acting: boolean;
  floor: boolean;
  onOpenJob: (jobId: string) => void;
  onDrop: () => void;
  onClose: () => void;
}) {
  const rendering = JOB_STATUS[job.status];
  const landed = waveLandedSaid(job);
  return (
    <Sheet open title={job.title} subtitle={job.handle} floor={floor} onClose={onClose}>
      <div className="armada-wave__inspector">
        {rendering?.badgeStatus == null || rendering.icon == null ? (
          <p className="armada-wave__needs-said">{job.status}</p>
        ) : (
          <Badge status={rendering.badgeStatus} icon={rendering.icon}>
            {rendering.verb}
          </Badge>
        )}
        <dl className="armada-wave__facts">
          <dt>Waits on</dt>
          <dd>{waits.length === 0 ? "nothing" : waits.map((one) => one.title).join(", ")}</dd>
          <dt>Pull request</dt>
          <dd>{landed ?? "not settled"}</dd>
          <dt>Dispatched by</dt>
          <dd>Pass {job.round}</dd>
        </dl>
        <div className="armada-wave__inspector-acts">
          <button
            type="button"
            className="armada-screen__eyebrow-act"
            onClick={() => onOpenJob(job.job)}
          >
            Open this Job
          </button>
          <HoldButton
            askLabel="Drop from the wave"
            description={DROP_SAID}
            disabled={stale || acting}
            onAsk={onDrop}
            onCommit={onDrop}
          >
            Hold to drop from the wave
          </HoldButton>
        </div>
      </div>
    </Sheet>
  );
}

export function WavePlan({ planStep, floor, onDropFromWave, ...region }: WavePlanProps) {
  // Which Job the inspector is on. **This destination's own state**: a sheet
  // that survived leaving the Plan would open over a board nobody is reading.
  const [open, setOpen] = useState<string | null>(null);

  const wave = region.draft?.wave ?? undefined;
  if (wave === undefined) return <WaveRegion {...region} />;

  const byId = new Map(wave.jobs.map((one) => [one.job, one]));
  const reading = open === null ? undefined : byId.get(open);
  const judged = judgedSaid(planStep);

  return (
    <>
      <WaveRegion {...region} onOpenJob={setOpen} />
      {judged === undefined ? null : (
        <p className="armada-wave__judged" role="note">
          {judged}
        </p>
      )}
      <Rounds wave={wave} />
      {reading === undefined ? null : (
        <JobInspector
          job={reading}
          waits={reading.waits_on.flatMap((id) => {
            const held = byId.get(id);
            return held === undefined ? [] : [held];
          })}
          stale={region.stale}
          acting={region.acting}
          floor={floor}
          onOpenJob={region.onOpenJob}
          onDrop={() => {
            onDropFromWave(reading.job);
            setOpen(null);
          }}
          onClose={() => setOpen(null)}
        />
      )}
    </>
  );
}
