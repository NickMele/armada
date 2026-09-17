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
// the acts float over the whiteboard in its own aside, and the dialog cannot be in there at all —
// React Flow paints its nodes over anything inside its subtree, so a dialog drawn there is read
// through the Notes it was opened from. Delete node was already outside for the same reason.

import { useState, type ReactNode } from "react";
import { Button, Dialog, Input, Select, Textarea } from "@armada/components";
import type { Outcome, Studio, StudioLinkForge, StudioNode, StudioPromotion } from "@armada/protocol";

import { nodeNamed } from "./studio";
import { actsOn, aWriteUp, dispatchedAs, placedBeside, selectedNodes } from "./studio-promotion";

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

/** The acts the selection offers, and the dialog whichever one opened. */
export type StudioPromotionParts = { acts: ReactNode; dialog: ReactNode };

/** The acts the selection offers, each one press from its dialog. */
export function useStudioPromotion(props: StudioPromotionProps): StudioPromotionParts {
  const { studio, selected } = props;
  const [filling, setFilling] = useState<Filling | null>(null);
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");
  const [blocks, setBlocks] = useState("");
  const [sending, setSending] = useState(false);
  const acts = actsOn(studio, selected);
  const picked = selectedNodes(studio, selected);
  const one = picked.length === 1 ? picked[0] : undefined;

  /** Open a rung, with whatever it starts from already in its fields. */
  function fill(rung: Filling): void {
    const written = rung === "write_up" && one !== undefined ? aWriteUp(studio, one) : null;
    const draft = rung === "edit" && one?.kind === "issue_draft" ? one : null;
    // A Link's line opens on what is already there, so editing it is a change
    // to what a person wrote rather than typing it again — #1378.
    const link = rung === "line" && one?.kind === "link" ? one : null;
    setTitle(written?.title ?? draft?.title ?? "");
    setBody(written?.body ?? draft?.body ?? link?.said ?? "");
    setBlocks("");
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
        return send({ act: "read_in", node_id, position });
      case null:
        return;
    }
  }

  const drawn = (
    // Named as a group: a whiteboard has a Dispatch of its own and so does the shell, and what
    // tells them apart for a reader is which panel each is in.
    <div className="armada-studio__acts" role="group" aria-label="Acts on what is selected">
      {acts.cluster ? <Act label="Cluster Notes" onPress={() => fill("cluster")} /> : null}
      {acts.outline ? <Act label="Outline" onPress={() => fill("outline")} /> : null}
      {acts.writeUp ? <Act label="Write up" onPress={() => fill("write_up")} /> : null}
      {acts.defer ? <Act label="Defer" onPress={() => fill("defer")} /> : null}
      {acts.edit ? <Act label="Edit draft" onPress={() => fill("edit")} /> : null}
      {acts.editLink ? <Act label="Edit line" onPress={() => fill("line")} /> : null}
      {acts.dispatch ? <Act label="Dispatch" onPress={() => fill("dispatch")} /> : null}
      {acts.readIn ? <Act label="Read in" onPress={() => fill("read_in")} /> : null}
      {acts.settle ? <Act label="Not a problem" onPress={() => fill("settled")} /> : null}
      {acts.settle ? <Act label="Resolved here" onPress={() => fill("resolved")} /> : null}
    </div>
  );

  return {
    acts: drawn,
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
          onTitle={setTitle}
          onBody={setBody}
          onBlocks={setBlocks}
        />
      </Dialog>
    ),
  };
}

function Act({ label, onPress }: { label: string; onPress: () => void }) {
  return (
    <Button size="sm" onClick={onPress}>
      {label}
    </Button>
  );
}

/**
 * What a Link's address names, in the words the dialog uses. **Read off
 * `forge`, which is Fleet's reading of the address** — nothing here reads one.
 */
const FORGE_NAMES: Readonly<Record<StudioLinkForge, string>> = {
  issue: "issue",
  pull_request: "pull request",
  milestone: "milestone",
};

/**
 * What the dialog is called. **Dispatch names what it was pressed on**, because
 * a draft sends words a person wrote and a Link sends something already on the
 * forge, and those are different things to be about to do — #1379.
 */
function asked(filling: Filling, one: StudioNode | undefined): string {
  if (filling === "dispatch" && one?.kind === "link" && one.forge !== undefined) {
    return `Dispatch the ${FORGE_NAMES[one.forge]} this Link names`;
  }
  return ASKED[filling];
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
  read_in: "Read this Link in",
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
  onTitle: (title: string) => void;
  onBody: (body: string) => void;
  onBlocks: (blocks: string) => void;
};

/** What each rung asks for, and what it says will happen. */
function Filling(props: FillingProps) {
  const { filling, studio, one, picked, title, body, blocks } = props;
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
            line leaves the Link as its address alone.
          </p>
          <Textarea label="Your line" rows={3} value={body} onChange={(event) => props.onBody(event.target.value)} />
        </>
      );
    case "read_in":
      return (
        <p>
          Armada fetches {one === undefined ? "this address" : named(one)} and a scout reads what comes back, off
          this Studio. The Link stays where it is and keeps its address. A milestone fills in as one Link per
          issue, with no scout at all; an address Armada does not read comes back saying so.
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
            {one?.kind === "link" && one.forge !== undefined
              ? // Nothing is filed here: what the Link names is already on the forge, and what
                // goes to the proposer is its address. **Which workflow is the proposer's** —
                // an issue, a pull request and a milestone are three different asks, and the
                // sentence says so rather than naming a workflow nobody chose — #1379.
                `This address goes to the Job proposer, and it reads the ${FORGE_NAMES[one.forge]}. Nothing is filed: it already exists. Which workflow the work runs under is the proposer's answer.`
              : "This text goes to the Job proposer, whole."}{" "}
            Every Job it becomes waits at the dispatch gate, and appears here as a Job node.
          </p>
          <Textarea
            label="What is sent"
            rows={one?.kind === "link" ? 2 : 8}
            readOnly
            value={one === undefined ? "" : (dispatchedAs(one) ?? "")}
          />
        </>
      );
  }
}
