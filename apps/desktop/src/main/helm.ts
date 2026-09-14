// One repository's Helm conversation: the socket that carries it, which
// repository it is pointed at, and the two requests that send into it. Held
// beside `observe.ts` for that file's own reason — a second socket to the
// same peer, never a second peer — and unlike it, this one can send. #939,
// #944.
//
// One at a time: the dock shows one repository's thread, and picking another
// replaces it. A `closed` message means Fleet forgot the thread this socket
// was reading — a Start fresh, on this window or another — so this reopens
// the same repository onto the new, empty one rather than leaving a viewer on
// a connection with nothing left to say.

import WebSocket from "ws";

import type {
  AskHelm,
  HelmContext,
  HelmMessage,
  HelmThread,
  HelmThreadItem,
  Outcome,
  RepositorySummary,
} from "@armada/protocol";
import { helmArrived, NO_HELM_ITEMS } from "@armada/protocol";
import { ask } from "./request";
import { HOST } from "./runtime-file";

type Held = { replying: boolean; skipped: number; missed: number; items: HelmThreadItem[] };

/** One repository's Helm conversation, read and written over one socket and two requests. */
export class HelmSocket {
  private readonly publish: (thread: HelmThread) => void;
  private socket: WebSocket | null = null;
  private manifestId: string | null = null;
  private port: number | null = null;
  private status: "opening" | "open" | "cleared" | "failed" = "opening";
  private held: Held = NO_HELM_ITEMS;
  private detail = "";
  /** An item's own identity, since none carries one — counted once per item actually added. */
  private seq = 0;

  constructor(publish: (thread: HelmThread) => void) {
    this.publish = publish;
  }

  /** Which repository's conversation is watched, or `null` to stop. Reopens only where that changed. */
  open(port: number | null, manifestId: string | null): void {
    if (manifestId !== null && manifestId === this.manifestId && this.socket !== null) return;
    this.close();
    this.manifestId = manifestId;
    this.port = port;
    this.held = NO_HELM_ITEMS;
    this.seq = 0;
    if (manifestId === null) {
      this.publish({ state: "none" });
      return;
    }
    if (port === null) {
      this.status = "failed";
      this.detail = "Fleet is not connected.";
      this.publishNow();
      return;
    }
    this.status = "opening";
    this.publishNow();
    this.connect(port, manifestId);
  }

  close(): void {
    const socket = this.socket;
    this.socket = null;
    if (socket === null) return;
    socket.removeAllListeners();
    // One listener stays, `observe.ts`'s reason: a socket still shaking hands
    // reports its abort as an `error` on the next tick, with nothing
    // listening, which Node throws.
    socket.on("error", () => {});
    socket.close();
  }

  private connect(port: number, manifestId: string): void {
    const path = `/helm/observe?manifest_id=${encodeURIComponent(manifestId)}`;
    const socket = new WebSocket(`ws://${HOST}:${port}${path}`);
    this.socket = socket;
    socket.on("message", (data: WebSocket.RawData) => this.arrived(manifestId, String(data)));
    socket.on("error", (cause: Error) => this.broke(manifestId, cause.message));
    socket.on("close", () => this.broke(manifestId, "the connection closed"));
  }

  private arrived(manifestId: string, text: string): void {
    if (manifestId !== this.manifestId) return;
    let message: HelmMessage;
    try {
      message = JSON.parse(text) as HelmMessage;
    } catch {
      this.broke(manifestId, "Fleet sent a message this Bridge could not read.");
      return;
    }

    const next = helmArrived(this.held, message, this.seq);
    if (next.added) this.seq += 1;
    this.held = next.held;

    if (next.ended !== undefined) {
      // Fleet forgot this thread. Say so for a beat, then reopen the same
      // repository onto the conversation replacing it.
      this.close();
      this.status = "cleared";
      this.publishNow();
      const port = this.port;
      if (port !== null) {
        this.held = NO_HELM_ITEMS;
        this.seq = 0;
        this.status = "opening";
        this.connect(port, manifestId);
      }
      return;
    }

    this.status = "open";
    this.publishNow();
  }

  /** The socket went without a sentence. What had arrived stays arrived. */
  private broke(manifestId: string, detail: string): void {
    if (this.socket === null || manifestId !== this.manifestId) return;
    this.socket.removeAllListeners();
    this.socket = null;
    this.status = "failed";
    this.detail = detail;
    this.publishNow();
  }

  private publishNow(): void {
    const manifestId = this.manifestId;
    if (manifestId === null) {
      this.publish({ state: "none" });
      return;
    }
    if (this.status === "opening") {
      this.publish({ state: "opening", manifestId });
      return;
    }
    if (this.status === "cleared") {
      this.publish({ state: "cleared", manifestId });
      return;
    }
    if (this.status === "failed") {
      this.publish({ state: "failed", manifestId, ...this.held, detail: this.detail });
      return;
    }
    this.publish({ state: "open", manifestId, ...this.held });
  }

  /** `POST /helm/ask`. Answers at once; the reply is always the socket's. */
  async askHelm(manifestId: string, text: string, context?: HelmContext): Promise<Outcome> {
    if (this.port === null) return { ok: false, why: "not_connected" };
    const body: AskHelm = context === undefined ? { text } : { text, context };
    const answer = await ask(
      this.port,
      "POST",
      `/helm/ask?manifest_id=${encodeURIComponent(manifestId)}`,
      body,
    );
    return answer.ok === true ? { ok: true } : answer.outcome;
  }

  /** `POST /helm/start_fresh`. Refused while a reply is being written. */
  async startFresh(manifestId: string): Promise<Outcome> {
    if (this.port === null) return { ok: false, why: "not_connected" };
    const answer = await ask(
      this.port,
      "POST",
      `/helm/start_fresh?manifest_id=${encodeURIComponent(manifestId)}`,
    );
    return answer.ok === true ? { ok: true } : answer.outcome;
  }
}

/**
 * Which repository Helm answers for, and the socket onto it.
 *
 * The most recent explicit act wins — a pick to a specific repository, or a
 * point ("Discuss with Helm", the dock's own switch) — and failing either,
 * Helm stays on the last repository it actually heard from this run, or the
 * first one Fleet lists.
 */
export class HelmConnection {
  private readonly publish: (change: { helm: HelmThread }) => void;
  private readonly port: () => number | null;
  private readonly socket: HelmSocket;
  private repositories: readonly RepositorySummary[] = [];
  /**
   * The most recent explicit act — a pick to a specific repository, or a
   * point ("Discuss with Helm", the dock's own switch) — whichever came
   * last. `none` until either has ever happened.
   *
   * **Moving to All is not an act with a target of its own.** A pick to a
   * repository sets this; a pick to All carries none to set, so it leaves
   * whichever act was most recent standing — which is what lets a person
   * pick a repository, look at All, and find Helm still on the one they
   * picked, and what lets Discuss survive the rail moving under it.
   */
  private explicit: { kind: "picked"; root: string } | { kind: "pointed"; manifestId: string } | { kind: "none" } = {
    kind: "none",
  };
  /** The last repository a message was actually sent to. In memory for this run of Bridge —
   * surviving a quit needs a `crates/config` field or a new local store, neither of which this reaches. */
  private lastTalked: string | null = null;

  constructor(wiring: { publish: (change: { helm: HelmThread }) => void; port: () => number | null }) {
    this.publish = wiring.publish;
    this.port = wiring.port;
    this.socket = new HelmSocket((helm) => this.publish({ helm }));
  }

  close(): void {
    this.socket.close();
  }

  /** The repositories Fleet serves changed, or were read for the first time. */
  onRepositoriesChanged(repositories: readonly RepositorySummary[]): void {
    this.repositories = repositories;
    this.retarget();
  }

  /** The rail's own pick moved. `null` is All repositories, which names nothing to point at. */
  onPicked(root: string | null): void {
    if (root !== null) this.explicit = { kind: "picked", root };
    this.retarget();
  }

  /** "Discuss with Helm" on a card, or the dock's own switch. The rail's pick does not move. */
  point(manifestId: string): void {
    this.explicit = { kind: "pointed", manifestId };
    this.retarget();
  }

  /** The connection came back, or a window asked for the state fresh. Reopens where it is not already up. */
  reconnected(port: number): void {
    this.socket.open(port, this.targetManifestId());
  }

  async askHelm(text: string, context?: HelmContext): Promise<Outcome> {
    const manifestId = this.targetManifestId();
    if (manifestId === null) return { ok: false, why: "not_connected" };
    const outcome = await this.socket.askHelm(manifestId, text, context);
    if (outcome.ok) this.lastTalked = manifestId;
    return outcome;
  }

  async startFresh(): Promise<Outcome> {
    const manifestId = this.targetManifestId();
    if (manifestId === null) return { ok: false, why: "not_connected" };
    return await this.socket.startFresh(manifestId);
  }

  /** The repository Helm answers for right now, for the dock to name. `null` where nothing is servable yet. */
  target(): string | null {
    return this.targetManifestId();
  }

  private retarget(): void {
    this.socket.open(this.port(), this.targetManifestId());
  }

  private targetManifestId(): string | null {
    const explicit = this.explicit;
    if (explicit.kind === "picked") {
      // Looked up live, not snapshotted: a repository picked before its
      // Manifest finished reading is followed to it the moment it arrives.
      return this.repositories.find((one) => one.root === explicit.root)?.manifest?.id ?? null;
    }
    if (explicit.kind === "pointed") return explicit.manifestId;
    if (this.lastTalked !== null) return this.lastTalked;
    return this.repositories.find((one) => one.manifest !== undefined)?.manifest?.id ?? null;
  }
}
