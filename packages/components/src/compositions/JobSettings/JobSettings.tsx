import { useId, type ReactNode } from "react";
import type { Reach, WhenBlocked, WhenRefused } from "@armada/protocol";
import { Button } from "../../primitives/Button/Button";
import { Radio, RadioGroup } from "../../primitives/Radio/Radio";
import { Select } from "../../primitives/Select/Select";
import { Sheet } from "../../primitives/Sheet/Sheet";

/**
 * Every setting a person can change on a running Job, on the trailing layer the
 * log, the patch and Pulse already use.
 *
 * **One panel rather than a line per setting.** The header carried one of them
 * — how the Job meets a command it was not given — as a menu under the facts,
 * and it read as the screen's main button with nothing saying what pressing it
 * did. Three settings as three header controls would be worse, and a setting is
 * read before it is changed: each one here says what it does and when it takes.
 *
 * **What the panel says is fixed; what it holds is the caller's.** The cap and
 * spend figures, the model names and the three choices' words come in, because
 * each has an owner elsewhere — `facts.ts` hedges a spend, `list_models` names
 * the models, and `copy.ts` holds the words the waiting band and the refused
 * rows also say. The raise dialogs are the caller's too: they exist, and this
 * does not write a second one.
 *
 * **Nothing here restarts anything**, and the lead says so before a control is
 * reached, because a person changing a running Job's model needs to know the
 * step in front of them is not thrown away.
 */
export type JobSettingsProps = {
  open: boolean;
  /** The Job, in mono. Absent at the floor, where the width is not there. */
  jobId?: ReactNode;
  /** The window is at `--window-floor`. */
  floor?: boolean;
  /** The cost ceiling. Absent where Fleet sent no spend, and the row is not drawn. */
  costCap?: JobSettingsCeiling;
  /** The turn ceiling, on `costCap`'s terms. */
  turnCap?: JobSettingsCeiling;
  /** What `list_models` offers, in its order. */
  models: readonly string[];
  /** The model chosen for later steps, or `null` for the workflow's own. */
  model: string | null;
  /** The line under the model once a change took. */
  modelSaid?: ReactNode;
  /** The three answers to a command the drone was not given, in the order to offer them. */
  choices: readonly JobSettingsChoice[];
  whenBlocked: WhenBlocked;
  /** The line under the choice once a change took. */
  whenBlockedSaid?: ReactNode;
  /**
   * The three answers to a judge criterion that refuses, in the order to
   * offer them. Absent where Fleet sent no `when_refused` — a Fleet older
   * than 11.3 — and then the section is not drawn at all.
   */
  judgeChoices?: readonly JobSettingsJudgeChoice[];
  whenRefused?: WhenRefused;
  /** The line under the choice once a change took. */
  whenRefusedSaid?: ReactNode;
  onWhenRefused?: (whenRefused: WhenRefused) => void;
  /** What a person allowed for this Job, oldest first. */
  allowed: readonly JobSettingsAllowed[];
  /** The line under the list once an allow was taken back. */
  allowedSaid?: ReactNode;
  /**
   * Every control is off — the reading is not live, or a change is already
   * out — as every control that sends is. The raise buttons go with them.
   */
  disabled?: boolean;
  /** Why they are off, said once under the lead rather than left to be guessed. */
  disabledNote?: ReactNode;
  onModel: (model: string | null) => void;
  onWhenBlocked: (whenBlocked: WhenBlocked) => void;
  onRemove: (run: string) => void;
  onClose?: () => void;
};

/** One ceiling: the cap in force, what has gone against it, and the press that raises it. */
export type JobSettingsCeiling = {
  /** The cap, exactly — `$60.00`, or `1000`. Mono. */
  cap: string;
  /** What has gone against it — `~$29.63`, hedged because it is estimated, or `580`. Mono. */
  used: string;
  /** Opens the raise. The dialog that collects the figure is the caller's. */
  onRaise: () => void;
  /** The line under the row once a raise took. */
  said?: ReactNode;
};

/** One answer to a command the drone was not given, with what it commits to. */
export type JobSettingsChoice = { value: WhenBlocked; label: string; means: string };

/** One answer to a judge criterion that refuses, with what it commits to. */
export type JobSettingsJudgeChoice = { value: WhenRefused; label: string; means: string };

/** One command allowed for this Job, and how far the allow reaches. */
export type JobSettingsAllowed = { run: string; reach: Reach };

/** The first option: no model chosen, so each step runs on the one its workflow gives it. */
const WORKFLOWS_CHOICE = "";

export function JobSettings({
  open,
  jobId,
  floor = false,
  costCap,
  turnCap,
  models,
  model,
  modelSaid,
  choices,
  whenBlocked,
  whenBlockedSaid,
  judgeChoices,
  whenRefused,
  whenRefusedSaid,
  allowed,
  allowedSaid,
  disabled = false,
  disabledNote,
  onModel,
  onWhenBlocked,
  onWhenRefused,
  onRemove,
  onClose,
}: JobSettingsProps) {
  const group = useId();
  // A chosen model Fleet holds and `list_models` no longer offers is still the
  // one in force, so it stays an option rather than the select falling back to
  // a first entry that says something untrue.
  const offered = model === null || models.includes(model) ? models : [model, ...models];
  const runsAll = whenBlocked === "allow_all";

  return (
    <Sheet
      open={open}
      contained
      floor={floor}
      title="Job settings"
      subtitle={
        jobId === undefined || floor ? undefined : (
          <span className="armada-job-settings__mono">{jobId}</span>
        )
      }
      closeLabel="Close"
      closeBinding="Esc"
      onClose={onClose}
    >
      <div className="armada-job-settings">
        <div className="armada-job-settings__opening">
          <p className="armada-job-settings__lead">
            For this job only. Each change applies from the next thing the drone does. Nothing
            restarts, and nothing already done is undone.
          </p>
          {disabled && disabledNote !== undefined ? (
            <p className="armada-job-settings__means">{disabledNote}</p>
          ) : null}
        </div>

        {costCap === undefined && turnCap === undefined ? null : (
          <section className="armada-job-settings__section" aria-labelledby={`${group}-limits`}>
            <h3 className="armada-job-settings__heading" id={`${group}-limits`}>
              Limits
            </h3>
            {costCap === undefined ? null : (
              <Ceiling
                label="Cost cap"
                raiseLabel="Raise the cost cap"
                figures={
                  <>
                    <span className="armada-job-settings__mono">{costCap.cap}</span>
                    {", "}
                    <span className="armada-job-settings__mono">{costCap.used}</span> spent
                  </>
                }
                ceiling={costCap}
                disabled={disabled}
              />
            )}
            {turnCap === undefined ? null : (
              <Ceiling
                label="Turn cap"
                raiseLabel="Raise the turn cap"
                figures={
                  <>
                    <span className="armada-job-settings__mono">{turnCap.cap}</span> turns,{" "}
                    <span className="armada-job-settings__mono">{turnCap.used}</span> used
                  </>
                }
                ceiling={turnCap}
                disabled={disabled}
              />
            )}
          </section>
        )}

        <section className="armada-job-settings__section" aria-labelledby={`${group}-model`}>
          <h3 className="armada-job-settings__heading" id={`${group}-model`}>
            Model
          </h3>
          <div className="armada-job-settings__field">
            <Select
              label="Model for the next step"
              value={model ?? WORKFLOWS_CHOICE}
              disabled={disabled}
              onChange={(event) =>
                onModel(event.target.value === WORKFLOWS_CHOICE ? null : event.target.value)
              }
            >
              <option value={WORKFLOWS_CHOICE}>The workflow's choice</option>
              {offered.map((name) => (
                <option key={name} value={name}>
                  {name}
                </option>
              ))}
            </Select>
            <p className="armada-job-settings__means">
              The step running now keeps its model. Later steps start on this one.
            </p>
            <Said>{modelSaid}</Said>
          </div>
        </section>

        <section className="armada-job-settings__section" aria-labelledby={`${group}-commands`}>
          <h3 className="armada-job-settings__heading" id={`${group}-commands`}>
            Commands
          </h3>
          <div className="armada-job-settings__field">
            <RadioGroup label="When the drone needs a command it wasn't given">
              {choices.map((choice) => (
                <div className="armada-job-settings__option" key={choice.value}>
                  <Radio
                    name={`${group}-when-blocked`}
                    value={choice.value}
                    checked={whenBlocked === choice.value}
                    disabled={disabled}
                    aria-describedby={`${group}-${choice.value}`}
                    onChange={() => onWhenBlocked(choice.value)}
                  >
                    {choice.label}
                  </Radio>
                  {/* Under the label, as a drone's question draws what each
                      answer commits to: read after the name, before the press. */}
                  <p className="armada-job-settings__option-means" id={`${group}-${choice.value}`}>
                    {choice.means}
                  </p>
                </div>
              ))}
            </RadioGroup>
            <Said>{whenBlockedSaid}</Said>
          </div>

          {/* **Dimmed and kept under Run it**, never emptied: switching back
              should find the list where it was, and a list that vanished would
              read as allows that were thrown away. Dimming is the token step,
              not an alpha. */}
          <div className="armada-job-settings__allowed" data-dimmed={runsAll || undefined}>
            <span className="armada-job-settings__label" id={`${group}-allowed`}>
              Allowed for this job
            </span>
            <p className="armada-job-settings__means">
              Commands you allowed while it ran. Always-allowed ones live in armada.yml instead.
            </p>
            {runsAll && allowed.length > 0 ? (
              <p className="armada-job-settings__means">
                Not needed while every command runs. They're kept in case you switch back.
              </p>
            ) : null}
            {allowed.length === 0 ? (
              <p className="armada-job-settings__empty">Nothing allowed for this job yet.</p>
            ) : (
              <ul className="armada-job-settings__commands" aria-labelledby={`${group}-allowed`}>
                {allowed.map((row) => (
                  <li className="armada-job-settings__command" key={row.run}>
                    <div className="armada-job-settings__command-text">
                      <span className="armada-job-settings__mono">{row.run}</span>
                      {/* Removing here does not reach the file. Said on the row,
                          because that is where somebody reads what Remove will do. */}
                      {row.reach === "repository" ? (
                        <span className="armada-job-settings__means">
                          Also always allowed in armada.yml. Removing it here leaves that in
                          place.
                        </span>
                      ) : null}
                    </div>
                    {/* The name carries the command, so a list of Removes is a
                        list of different acts to anything reading by name. */}
                    <Button
                      variant="secondary"
                      size="sm"
                      ground="sunken"
                      disabled={disabled}
                      aria-label={`Remove ${row.run}`}
                      onClick={() => onRemove(row.run)}
                    >
                      Remove
                    </Button>
                  </li>
                ))}
              </ul>
            )}
            <Said>{allowedSaid}</Said>
          </div>
        </section>

        {judgeChoices === undefined || whenRefused === undefined ? null : (
          <section className="armada-job-settings__section" aria-labelledby={`${group}-judge`}>
            <h3 className="armada-job-settings__heading" id={`${group}-judge`}>
              Judge
            </h3>
            <div className="armada-job-settings__field">
              <RadioGroup label="When a judge refuses">
                {judgeChoices.map((choice) => (
                  <div className="armada-job-settings__option" key={choice.value}>
                    <Radio
                      name={`${group}-when-refused`}
                      value={choice.value}
                      checked={whenRefused === choice.value}
                      disabled={disabled}
                      aria-describedby={`${group}-${choice.value}`}
                      onChange={() => onWhenRefused?.(choice.value)}
                    >
                      {choice.label}
                    </Radio>
                    <p className="armada-job-settings__option-means" id={`${group}-${choice.value}`}>
                      {choice.means}
                    </p>
                  </div>
                ))}
              </RadioGroup>
              <Said>{whenRefusedSaid}</Said>
            </div>
          </section>
        )}
      </div>
    </Sheet>
  );
}

/**
 * One ceiling's row. **It can only go up**, and the row says so beside the one
 * control it has: a lower cap on a running Job would stop work Fleet has already
 * admitted, so there is no field here to type one into.
 */
function Ceiling({
  label,
  raiseLabel,
  figures,
  ceiling,
  disabled,
}: {
  label: string;
  raiseLabel: string;
  figures: ReactNode;
  ceiling: JobSettingsCeiling;
  disabled: boolean;
}) {
  return (
    <div className="armada-job-settings__field">
      <div className="armada-job-settings__row">
        <div className="armada-job-settings__row-text">
          <span className="armada-job-settings__label">{label}</span>
          <span className="armada-job-settings__figures">{figures}</span>
          <p className="armada-job-settings__means">It can only go up while the job runs.</p>
        </div>
        {/* `Raise` on the face and the ceiling in the name: the row's label
            already says which, and the dialog it opens names the act whole. */}
        <Button
          variant="secondary"
          size="sm"
          ground="sunken"
          disabled={disabled}
          aria-label={raiseLabel}
          onClick={ceiling.onRaise}
        >
          Raise
        </Button>
      </div>
      <Said>{ceiling.said}</Said>
    </div>
  );
}

/**
 * The line under a row once a change took, and when it applies. A live region,
 * because the change was made with a press and the answer arrives a round trip
 * later — a person who pressed and looked away is told without looking back.
 */
function Said({ children }: { children: ReactNode }) {
  if (children === undefined || children === null) return null;
  return (
    <p className="armada-job-settings__said" role="status">
      {children}
    </p>
  );
}

/**
 * The header's way into the panel, and how much on it differs from how a Job
 * starts.
 *
 * **Secondary, and beside the acts rather than among them.** It ends nothing
 * and it is not the screen's decision, so it takes no accent; it sits left of
 * the act group because that is where the header's controls are, and the
 * count is what lets a person see a Job was changed without opening anything.
 * **The count is quiet**: a changed Job is not a problem, so it is `--fg-muted`
 * text inside the button rather than a badge, which is a Job state and nothing
 * else.
 */
export function JobSettingsButton({
  changed,
  onOpen,
}: {
  /** How many settings differ from a new Job's. Nothing is drawn at zero. */
  changed: number;
  onOpen: () => void;
}) {
  return (
    <Button variant="secondary" onClick={onOpen}>
      Job settings
      {changed > 0 ? (
        <span className="armada-job-settings-button__count">{`${changed} changed`}</span>
      ) : null}
    </Button>
  );
}
