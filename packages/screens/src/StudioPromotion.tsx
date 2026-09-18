// Promotion on the whiteboard: cluster, outline, defer, write up, edit, end a Contradiction,
// dispatch — #1291. Its own file because `Studios.tsx` is the surface and this is one panel on it.
//
// **Every act is a dialog that collects.** Each rung writes a node a person then reads, so each
// asks for the words before it sends them rather than making one from a title Armada invented.
//
// **Nothing here decides.** `studio-promotion.ts` says which acts the selection offers and Fleet
// holds every rule; what is drawn is what a person may reach and what came back when they pressed.
//
// **The acts and their dialog are two renderings of one state**, because they belong in two places:
// the rungs are entries in the aside's one control since #1399 — a list, not a row that grows with
// the kinds — and the dialog cannot be in the board at all, since React Flow paints its nodes over
// anything inside its subtree. Delete node was already outside for the same reason.

import { useState, type ReactNode } from "react";
import { Dialog, Input, Select, Textarea, type StudioPickedAct } from "@armada/components";
import type { EpicTake, Outcome, Studio, StudioNode, StudioPromotion } from "@armada/protocol";

import { nodeNamed } from "./studio";
import { actsOn, aWriteUp, dispatchedAs, placedBeside, selectedNodes } from "./studio-promotion";

/** One rung as the aside's control offers it. */
type Rung = StudioPickedAct & { id: Filling };

/** Which rung a person is filling in, or `null`. */
type Filling =
  | "cluster"
  | "outline"
  | "defer"
  | "write_up"
  | "edit"
  | "line"
  | "resolved"
  | "settled"
  | "dispatch"
  | "read_in";

export type StudioPromotionProps = {
  studio: Studio;
  /** The nodes picked on the whiteboard, in the order it reports them. */
  selected: readonly string[];
  /** One rung, as `main` sends it. Answers with the Studio folded in, or why not. */
  onPromote: (promotion: StudioPromotion) => Promise<Outcome>;
  /** What to say when Fleet refused, or `null` to clear it. */
  onAnswered: (outcome: Outcome) => void;
};

/**
 * The rungs the selection offers, what to do when one is chosen, and the dialog
 * whichever one opened.
 */
export type StudioPromotionParts = {
  acts: readonly StudioPickedAct[];
  onAct: (id: string) => void;
  dialog: ReactNode;
};

/** The acts the selection offers, each one press from its dialog. */
export function useStudioPromotion(props: StudioPromotionProps): StudioPromotionParts {
  const { studio, selected } = props;
  const [filling, setFilling] = useState<Filling | null>(null);
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");
  const [blocks, setBlocks] = useState("");
  // Which of an Epic's issues to take — #1405. It opens on what the Epic
  // already took, so pressing Read in again shows the answer it is changing.
  const [take, setTake] = useState<EpicTake>("open");
  const [sending, setSending] = useState(false);
  const acts = actsOn(studio, selected);
  const picked = selectedNodes(studio, selected);
  const one = picked.length === 1 ? picked[0] : undefined;

  /** Open a rung, with whatever it starts from already in its fields. */
  function fill(rung: Filling): void {
    const written = rung === "write_up" && one !== undefined ? aWriteUp(studio, one) : null;
    const draft = rung === "edit" && one?.kind === "issue_draft" ? one : null;
    // The line opens on what is already there, so editing it is a change to
    // what a person wrote rather than typing it again — #1378.
    const link = rung === "line" && one !== undefined && "said" in one ? one : null;
    setTitle(written?.title ?? draft?.title ?? "");
    setBody(written?.body ?? draft?.body ?? link?.said ?? "");
    setBlocks("");
    if (rung === "read_in") setTake(one?.kind === "epic" ? (one.read_in?.took ?? "open") : "open");
    setFilling(rung);
  }

  function send(promotion: StudioPromotion): void {
    setSending(true);
    void props.onPromote(promotion).then((outcome) => {
      setSending(false);
      setFilling(null);
      props.onAnswered(outcome);
      // **What was picked stays picked.** Selection is the whiteboard's, so clearing a copy of it
      // here would be undone by its next report — and a person reads the new node beside the one
      // they made it from.
    });
  }

  function confirm(): void {
    const position = placedBeside(studio, selected);
    const node_id = one?.id ?? "";
    switch (filling) {
      case "cluster":
        return send({
          act: "group",
          kind: "cluster",
          title,
          from: [...selected],
          position,
        });
      case "outline":
        return send({
          act: "group",
          kind: "outline",
          body,
          from: [...selected],
          position,
        });
      case "defer":
        return send({
          act: "defer",
          what: body,
          raised_on: node_id,
          ...(blocks === "" ? {} : { blocks }),
          position,
        });
      case "write_up":
        return send({ act: "write_up", node_id, title, body, position });
      case "edit":
        return send({ act: "edit_draft", node_id, title, body });
      case "line":
        return send({ act: "edit_link", node_id, said: body });
      case "settled":
        return send({ act: "settle", node_id, outcome: "not_a_problem" });
      case "resolved":
        return send({
          act: "settle",
          node_id,
          outcome: "resolved_here",
          answer: body,
        });
      case "dispatch":
        return send({ act: "dispatch", node_id, position });
      case "read_in":
        // **The answer rides on an Epic alone.** Every other kind has one
        // thing to read and nothing to ask about — #1405.
        return send({ act: "read_in", node_id, position, ...(one?.kind === "epic" ? { take } : {}) });
      case null:
        return;
    }
  }

  // In the order a person climbs them: what several nodes become, then what one
  // node becomes, then how a Contradiction ends.
  const rung = (id: Filling, label: string, offers: boolean): Rung[] => (offers ? [{ id, label }] : []);
  const offered: Rung[] = [
    ...rung("cluster", "Cluster Notes", acts.cluster),
    ...rung("outline", "Outline", acts.outline),
    ...rung("write_up", "Write up", acts.writeUp),
    ...rung("defer", "Defer", acts.defer),
    ...rung("edit", "Edit draft", acts.edit),
    ...rung("line", "Edit line", acts.editLink),
    ...rung("dispatch", "Dispatch", acts.dispatch),
    ...rung("read_in", "Read in", acts.readIn),
    ...rung("settled", "Not a problem", acts.settle),
    ...rung("resolved", "Resolved here", acts.settle),
  ];

  return {
    acts: offered,
    onAct: (id) => {
      const chosen = offered.find((act) => act.id === id);
      if (chosen !== undefined) fill(chosen.id);
    },
    dialog: (
      <Dialog
        open={filling !== null}
        tone="neutral"
        title={filling === null ? "" : asked(filling, one)}
        confirmLabel={filling === null ? "" : CONFIRMS[filling]}
        confirmDisabled={sending || !said(filling, title, body)}
        onCancel={() => setFilling(null)}
        onConfirm={confirm}
      >
        <Filling
          filling={filling}
          studio={studio}
          one={one}
          picked={picked}
          title={title}
          body={body}
          blocks={blocks}
          take={take}
          onTitle={setTitle}
          onBody={setBody}
          onBlocks={setBlocks}
          onTake={setTake}
        />
      </Dialog>
    ),
  };
}

/**
 * What the dialog is called. **Dispatch names what it was pressed on**, because
 * a draft sends words a person wrote and the other three send something already
 * on a forge, and those are different things to be about to do — #1379, #1394.
 */
function asked(filling: Filling, one: StudioNode | undefined): string {
  if (filling === "dispatch" && one !== undefined && one.kind in ON_A_FORGE) {
    return `Dispatch this ${ON_A_FORGE[one.kind]}`;
  }
  if (filling === "read_in" && one !== undefined) return `Read this ${aKind(one)} in`;
  if (filling === "line" && one !== undefined) return "Say why you kept this";
  return ASKED[filling];
}

/**
 * What each of the three is called where the dialog names it. **Read off the
 * kind and never off the address**, which is a thing this side may not read.
 */
const ON_A_FORGE: Readonly<Record<string, string>> = {
  issue: "issue",
  pull_request: "pull request",
  epic: "epic",
};

/** What a node with an address is called in a sentence. */
function aKind(node: StudioNode): string {
  return ON_A_FORGE[node.kind] ?? "Link";
}

/** What each rung's dialog is called, in sentence case, naming what happens. */
const ASKED: Readonly<Record<Filling, string>> = {
  cluster: "Accept these Notes as one Cluster",
  outline: "Read these nodes as one Outline",
  defer: "Put this off",
  write_up: "Write this up as an Issue draft",
  edit: "Edit this Issue draft",
  line: "Say why you kept this",
  settled: "End this as Not a problem",
  resolved: "Resolve this here",
  dispatch: "Dispatch this Issue draft",
  read_in: "Read this in",
};

/** The action, keeping its name through the flow. */
const CONFIRMS: Readonly<Record<Filling, string>> = {
  cluster: "Cluster",
  outline: "Outline",
  defer: "Defer",
  write_up: "Write up",
  edit: "Save draft",
  line: "Save line",
  settled: "Not a problem",
  resolved: "Resolve",
  dispatch: "Dispatch",
  read_in: "Read in",
};

/** Whether the rung has what it needs. Fleet refuses a blank field; this does not offer one. */
function said(filling: Filling | null, title: string, body: string): boolean {
  switch (filling) {
    case "cluster":
      return title.trim() !== "";
    case "outline":
    case "defer":
    case "resolved":
      return body.trim() !== "";
    case "write_up":
    case "edit":
      return title.trim() !== "" && body.trim() !== "";
    default:
      return true;
  }
}

type FillingProps = {
  filling: Filling | null;
  studio: Studio;
  one: StudioNode | undefined;
  picked: readonly StudioNode[];
  title: string;
  body: string;
  blocks: string;
  take: EpicTake;
  onTitle: (title: string) => void;
  onBody: (body: string) => void;
  onBlocks: (blocks: string) => void;
  onTake: (take: EpicTake) => void;
};

/** What each rung asks for, and what it says will happen. */
function Filling(props: FillingProps) {
  const { filling, studio, one, picked, title, body, blocks, take } = props;
  const named = (node: StudioNode) => nodeNamed(studio, node.id, []);
  switch (filling) {
    case null:
      return null;
    case "cluster":
    case "outline":
      return (
        <>
          <p>
            {picked.map(named).join("; ")}. Each keeps a Produced edge to the new node, in the order they were
            picked.
          </p>
          {filling === "cluster" ? (
            <Input label="Title" value={title} onChange={(event) => props.onTitle(event.target.value)} />
          ) : (
            <Textarea
              label="Reading"
              rows={4}
              value={body}
              onChange={(event) => props.onBody(event.target.value)}
            />
          )}
        </>
      );
    case "defer":
      return (
        <>
          <p>Raised on {one === undefined ? "the node" : named(one)}, and put off until somebody answers it.</p>
          <Textarea
            label="What is being put off"
            rows={3}
            value={body}
            onChange={(event) => props.onBody(event.target.value)}
          />
          <Select label="Blocks" value={blocks} onChange={(event) => props.onBlocks(event.target.value)}>
            <option value="">Nothing on this Studio</option>
            {studio.nodes
              .filter((node) => node.id !== one?.id)
              .map((node) => (
                <option key={node.id} value={node.id}>
                  {named(node)}
                </option>
              ))}
          </Select>
        </>
      );
    case "write_up":
    case "edit":
      return (
        <>
          <p>
            {filling === "write_up"
              ? "A draft on this Studio, and nothing filed anywhere. Filing it is yours to do."
              : "What is dispatched is what you leave here."}
          </p>
          <Input label="Title" value={title} onChange={(event) => props.onTitle(event.target.value)} />
          <Textarea label="Body" rows={8} value={body} onChange={(event) => props.onBody(event.target.value)} />
        </>
      );
    case "line":
      return (
        <>
          <p>
            The card reads this line, with the address under it. The address does not change, and clearing the
            line leaves the node as its address alone.
          </p>
          <Textarea label="Your line" rows={3} value={body} onChange={(event) => props.onBody(event.target.value)} />
        </>
      );
    case "read_in":
      // **An epic is the one kind with something to ask.** It holds a set, and
      // which of that set to take is the person's answer — #1405.
      return one?.kind === "epic" ? (
        <>
          <p>
            Armada fetches this epic and fills it in as one Issue per issue, with no scout at all. The node stays
            where it is and keeps its address. Reading it in again with the other answer widens or narrows what is
            here, and leaves standing anything you have since worked on.
          </p>
          <Select label="Take" value={take} onChange={(event) => props.onTake(event.target.value as EpicTake)}>
            <option value="open">Only what is open</option>
            <option value="everything">Every issue</option>
          </Select>
        </>
      ) : (
        <p>
          Armada fetches {one === undefined ? "this address" : named(one)} and a scout reads what comes back, off
          this Studio. The node stays where it is and keeps its address; an address Armada does not read comes back
          saying so.
        </p>
      );
    case "settled":
      return (
        <p>Both statements hold, in different contexts. The node keeps saying so, and nothing else is made.</p>
      );
    case "resolved":
      return (
        <>
          <p>The answer is kept on the node, so this can be reread.</p>
          <Textarea label="Answer" rows={4} value={body} onChange={(event) => props.onBody(event.target.value)} />
        </>
      );
    case "dispatch":
      return (
        <>
          <p>
            {one !== undefined && one.kind in ON_A_FORGE
              ? // Nothing is filed: what the node names is already on the forge, and what goes to
                // the proposer is its address. **Which workflow is the proposer's** — an issue, a
                // pull request and an epic are three different asks, and the sentence says so
                // rather than naming a workflow nobody chose — #1379, #1394.
                `This address goes to the Job proposer, and it reads the ${ON_A_FORGE[one.kind]}. Nothing is filed: it already exists. Which workflow the work runs under is the proposer's answer.`
              : "This text goes to the Job proposer, whole."}{" "}
            Every Job it becomes waits at the dispatch gate, and appears here as a Job node.
          </p>
          <Textarea
            label="What is sent"
            rows={one !== undefined && one.kind in ON_A_FORGE ? 2 : 8}
            readOnly
            value={one === undefined ? "" : (dispatchedAs(one) ?? "")}
          />
        </>
      );
  }
}
