// Dispatch a job: describe the work, or fill the form in by hand.
//
// **One surface with two ways through it, and they are not equal.** Describing
// the work is the path — the Job proposer reads the request and answers the
// title, the workflow and, where the work is several jobs, the order between
// them. `docs/concepts/job-proposer.md` calls hand entry the override, and the
// composer this swaps to is what hand entry became.
//
// # The proposal is this screen's state, not the app's
//
// Nothing outside this surface reads it, it dies when the surface closes, and
// the app holding it made the guard below depend on the app re-rendering in
// time. What does cross back is the half the app draws: a refusal with no
// drawing here is an `Outcome`, and `answeredAs` is what decides which of the
// two an answer is.
//
// # The double-press guard, and why it is two things
//
// There is no in-flight guard on the preload call, so two presses are two model
// calls and two drafted plans — two of everything at the gate, and somebody
// deleting one by hand. **The form is what stops it.**
//
// The control being off while a call is out is the first half and the one a
// person sees. The second half is the ref: a press that arrives anyway sends
// nothing, because one request is outstanding until its promise settles. A
// disabled attribute is a rendering, and a rendering is not a guarantee — a key
// repeat and a synthetic click both reach the handler with the button drawn
// live.

import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";

import { Button, DispatchRequest, DispatchSettings, WhatElseIsRunning } from "@armada/components";
import type {
  DispatchSettingsValue,
  PeerJob,
  Proposal,
  ProposalWatch,
  Refs,
} from "@armada/components";
import type { StagedAttachment, WorkflowSummary } from "@armada/protocol";

import { howManySet } from "./draft/dispatch";
import type { DispatchSettingsView } from "./draft/dispatch";
import type { LandingRule } from "./draft/landing";
import type { PeerOverlapAnswer, PeerView } from "./draft/peers";
import type { Answered } from "./proposal";
import { PROPOSAL_IS_SLOW } from "./proposal";

/** What the control that goes back to describing is called. */
const BACK = "Describe the work instead";

export type DispatchJobProps = {
  /**
   * Send the request, and answer with what came back.
   *
   * **A promise rather than a callback**, for the reason `onReadCall` is one:
   * the answer belongs to the press that asked for it and to nothing else, so
   * publishing it as app state would make one person's gesture part of what
   * every surface re-renders on.
   *
   * Called at most once per answer, and never with a blank request.
   */
  onPropose: (request: string, attachments: readonly StagedAttachment[]) => Promise<Answered>;
  /**
   * Put a picked or pasted file somewhere the Job can name, and answer with
   * the path. The same call `Composer`'s `onStage` makes; this screen owns
   * the staged list until `onPropose` carries it on.
   */
  onStage: (bytes: ArrayBuffer, filename: string, mimeType: string) => Promise<{ path: string }>;
  /** Narrow the checkout against typed text, for the `@` mention popup. */
  onSearchFiles: (query: string) => Promise<readonly string[]>;
  /** Open one of the jobs that came back, where its own gate is drawn. */
  onOpen: (jobId: string) => void;
  /**
   * Release the head of the proposal. **What starts the work, from the screen
   * that proposed it** — the workflow, the name and the split are all on it, so
   * approving is a press here rather than a trip to detail.
   */
  onApprove: (jobId: string) => void;
  /** Jobs with an approval in flight, from the app. What stops a second press. */
  approving: readonly string[];
  /**
   * What Fleet says a proposed Job is at now, or `undefined` where the board
   * holds no row for it.
   *
   * **The proposal is this screen's and the status is Fleet's**, and approving
   * one changes the status without changing the proposal. A row drawn from the
   * answer alone would say `needs approval` for as long as the surface stayed
   * open, under a job already queued.
   */
  statusOf?: (jobId: string) => string | undefined;
  /** Hand entry, built by the caller. The composer, and the override. */
  byHand: ReactNode;
  /**
   * What Fleet says the call in flight is doing, or `null`.
   *
   * **The app's, where the proposal is this screen's.** The two are not the
   * same fact and cannot come from the same place: the proposal is what the
   * press asked for and belongs to the press, and this arrives on the event
   * stream between the asking and the answer. A screen holding its own copy
   * would have nothing to fill it from.
   */
  watching: ProposalWatch | null;
  /**
   * Stop the call. **Kills it rather than stopping the wait** — a wait
   * abandoned leaves the proposer running inside Fleet and spending.
   */
  onStop: () => void;
  /**
   * The way out of the composer, drawn on the head of the card that asks for
   * the request. The caller draws it once and hands the same control to every
   * state it opens; absent draws none.
   */
  close?: ReactNode;
  /** The repository this dispatch is for. A fact, answered before this opens. */
  repository?: string;
  /**
   * Where the work starts and where it lands, as the Manifest declares them.
   * Both `null` is a Manifest naming no base, which draws empty fields rather
   * than a branch name nobody chose.
   */
  landing: LandingRule;
  /**
   * What else is writing where this work would. **`null` is nobody having
   * looked**, which is every request at dispatch today — the read hangs off a
   * Job and there is no Job yet. Drawing it as "nobody is there" would say
   * something nothing has checked.
   */
  peers: PeerOverlapAnswer;
  /** The workflows Fleet holds, for the settings block's override. */
  workflows: readonly WorkflowSummary[];
  /** The models a tier may name. Empty until the connection answers. */
  models: readonly string[];
  /** How many Drones this machine runs across every Job. `null` before Fleet said. */
  machineCap: number | null;
  /** What the settings block opens on. Absent is nothing set, which is the ordinary case. */
  settings?: DispatchSettingsView;
  /**
   * Whether the request field or its attachments hold anything closing would
   * throw away. Reported as it changes, and `false` on the way out, so the
   * caller's way out can ask first. #1366.
   */
  onTyped?: (typed: boolean) => void;
  /** Nothing may be dispatched while the connection is not live. */
  disabled: boolean;
  /** Why the controls are off, where they are. */
  disabledNote?: ReactNode;
  /** What the surface is told after a clipboard write, so it can raise a toast. */
  onCopied?: (what: string) => void;
};

export function DispatchJob({
  onPropose,
  onStage,
  onSearchFiles,
  onOpen,
  onApprove,
  approving,
  statusOf,
  byHand,
  close,
  repository,
  landing,
  peers,
  workflows,
  models,
  machineCap,
  settings: opensOn,
  onTyped,
  watching,
  onStop,
  disabled,
  disabledNote,
  onCopied,
}: DispatchJobProps) {
  const [request, setRequest] = useState("");
  const [attachments, setAttachments] = useState<StagedAttachment[]>([]);
  // The two refs, seeded from the Manifest and a person's to change. Held as
  // strings rather than as the rule's `string | null`: a field's empty value is
  // "", and `null` here would be a second spelling of the same emptiness.
  const [refs, setRefs] = useState<Refs>({
    from: landing.from_ref ?? "",
    target: landing.target ?? "",
  });
  const [links, setLinks] = useState<string[]>([]);
  const [settings, setSettings] = useState<DispatchSettingsValue>(asChosen(opensOn));
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [proposal, setProposal] = useState<Proposal>({ at: "unasked" });
  // Which of the two ways through this surface is open. Describing is the
  // path, so it is what the surface opens on.
  const [hand, setHand] = useState(false);
  // One request outstanding at a time. A ref rather than state because nothing
  // renders from it: it is the guard, not a reading, and a re-render between
  // the press and the answer would be a second chance to fire.
  const outstanding = useRef(false);

  // What a close would lose here: the words in the field, anything staged or
  // linked against them, and any setting moved off what it opened on. A
  // proposal that came back is not in it — those jobs exist on the board
  // already and closing this surface does not touch them.
  const typed =
    request.trim() !== "" ||
    attachments.length > 0 ||
    links.length > 0 ||
    howManySet(asDrafted(settings)) > 0;
  useEffect(() => {
    onTyped?.(typed);
    return () => onTyped?.(false);
  }, [typed, onTyped]);

  async function dispatch(): Promise<void> {
    // Links go out with the request, one to a line. The proposer's field is
    // prose and a ticket address in it is what it already reads; a chip is so
    // the person can see and take back what they pasted.
    const asked = [request.trim(), ...links].join("\n");
    if (outstanding.current || disabled || request.trim() === "") return;
    outstanding.current = true;
    setProposal({ at: "reading" });
    try {
      const read = await onPropose(asked, attachments);
      setProposal(read.proposal);
      // Fleet's echo, put back in the field. A refused request comes back
      // unchanged, and this is what makes that true rather than only stated.
      if (read.request !== null) setRequest(read.request);
    } catch (thrown) {
      // A surface left on `reading` is worse than the throw: nothing on it says
      // the call is dead and the control never comes back. The throw carries on
      // to the app's own handler for a rejection nothing caught.
      setProposal({ at: "unasked" });
      throw thrown;
    } finally {
      outstanding.current = false;
    }
  }

  function reset(): void {
    setRequest("");
    setAttachments([]);
    setLinks([]);
    setProposal({ at: "unasked" });
  }

  // The rows, against the board's own reading of each Job where there is one.
  // Every other status on this surface is what came back with the proposal;
  // this one moves under it, because approving is now a press on the row.
  const shown: Proposal =
    proposal.at === "proposed" && statusOf !== undefined
      ? {
          ...proposal,
          jobs: proposal.jobs.map((job) => {
            const at = statusOf(job.id);
            return at === undefined || at === job.status ? job : { ...job, status: at };
          }),
        }
      : proposal;

  if (hand) {
    return (
      <div className="armada-screen__pane">
        {/* The way back, above the form. Hand entry is the exception, so
            leaving it is one press and never a dead end. */}
        <div>
          <Button variant="secondary" size="sm" onClick={() => setHand(false)}>
            {BACK}
          </Button>
        </div>
        {byHand}
      </div>
    );
  }

  return (
    <div className="armada-dispatch-pane">
      <div className="armada-dispatch-pane__columns">
      <DispatchRequest
        request={request}
        onRequest={setRequest}
        {...(repository === undefined ? {} : { repository })}
        refs={refs}
        onRefs={setRefs}
        links={links}
        onAddLink={(address) =>
          setLinks((current) => (current.includes(address) ? current : [...current, address]))
        }
        onRemoveLink={(address) => setLinks((current) => current.filter((one) => one !== address))}
        settings={
          <DispatchSettings
            open={settingsOpen}
            onOpenChange={setSettingsOpen}
            settings={settings}
            onSettings={setSettings}
            workflows={workflows}
            models={models}
            machineCap={machineCap}
            disabled={disabled}
          />
        }
        attachments={attachments}
        onStage={onStage}
        onSearchFiles={onSearchFiles}
        onAttach={(attachment) => setAttachments((current) => [...current, attachment])}
        onRemoveAttachment={(path) =>
          setAttachments((current) => current.filter((attachment) => attachment.path !== path))
        }
        onDispatch={() => void dispatch()}
        onEnterByHand={() => setHand(true)}
        close={close}
        onReset={reset}
        onOpen={onOpen}
        onApprove={onApprove}
        approving={approving}
        // The two facts are joined here and nowhere else: the proposal is this
        // screen's and the watch is the app's, and the component takes one value.
        // A screen still `reading` with nothing published yet draws the wait
        // without a reading, which is the sentence that was always there.
        proposal={
          proposal.at === "reading" && watching !== null
            ? { at: "reading", watch: watching }
            : shown
        }
        onStop={onStop}
        slowAfterMs={PROPOSAL_IS_SLOW}
        disabled={disabled}
        disabledNote={disabledNote}
        onCopied={onCopied}
      />
        {/* Beside the request, never over it. An overlap is a fact about
            what is running and it decides nothing here, so it takes a column
            of its own rather than a warning on the control that dispatches. */}
        <WhatElseIsRunning
          peers={peers === null ? null : peers.peers.map(rowOf)}
          {...(peers === null ? {} : { pathsAsked: peers.paths_asked })}
          onOpen={onOpen}
        />
      </div>
    </div>
  );
}

/**
 * The draft's settings as the control holds them, and back again.
 *
 * **Two spellings of four values, and the seam is here on purpose.** The draft
 * is wire-shaped because that is where it is going (`draft/dispatch.ts` names
 * the module); a component's props are the app's own. Mapping in one function
 * each is what keeps a rename on either side a compile error rather than a
 * field that silently stops arriving.
 */
function asChosen(view: DispatchSettingsView = {}): DispatchSettingsValue {
  const chosen: DispatchSettingsValue = {};
  if (view.workflow_id !== undefined) chosen.workflowId = view.workflow_id;
  if (view.tiers !== undefined) chosen.tiers = { ...view.tiers };
  if (view.drone_cap !== undefined) chosen.droneCap = view.drone_cap;
  if (view.lands !== undefined) chosen.lands = view.lands;
  return chosen;
}

function asDrafted(chosen: DispatchSettingsValue): DispatchSettingsView {
  const view: DispatchSettingsView = {};
  if (chosen.workflowId !== undefined) view.workflow_id = chosen.workflowId;
  if (chosen.tiers !== undefined) view.tiers = { ...chosen.tiers };
  if (chosen.droneCap !== undefined) view.drone_cap = chosen.droneCap;
  if (chosen.lands !== undefined) view.lands = chosen.lands;
  return view;
}

/** One peer, as the panel draws it. */
function rowOf(peer: PeerView): PeerJob {
  return {
    id: peer.job,
    title: peer.title,
    status: peer.status,
    sharedPaths: peer.shared_paths,
  };
}
