// A repository's Studios and the one open, held while the Studios surface shows — #1287.
//
// `holding.ts`'s shape twice over: a read opened by a surface and dropped when it closes, with an
// answer that lands after the close thrown away. **Kept current by the stream, not by polling**:
// `studio.changed` carries a Studio whole after every write, and an act's own answer is the same
// Studio, so both are folded the one way — the open Studio replaced, the list's row replaced.
//
// **One per main, not per window**, `holding.ts`'s own limit: two windows on two repositories'
// Studios would each read the other's list after a switch. Overview went per window for exactly
// this, and the day two windows hold Studios open at once, this follows it.

import type {
  CaptureStudioNote,
  FrameRead,
  Outcome,
  StagedFrame,
  Studio,
  StudioCapture,
  StudioDeleted,
  StudioList,
  StudioNodeByHand,
  StudioPosition,
  StudioPromotion,
} from "@armada/protocol";
import { foldStudio } from "@armada/screens/src/studio-reads";
import type { StudioAnswer, StudioRead, StudiosRead } from "@armada/screens/src/studio-reads";
import { ask, studioFrameOf } from "./request";

type Publish = (change: { studios?: StudiosRead; studio?: StudioRead }) => void;

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

const member = (studioId: string, act = "") => `/studios/${encodeURIComponent(studioId)}${act}`;

/** How far under the lowest node a capture lands, in canvas units. A node card's height and a gap. */
const NOTE_APART = 260;

/** Where each rung of promotion is served. `crates/ipc/operations.toml` is the authority. */
const PROMOTION_ROUTE: Readonly<Record<StudioPromotion["act"], string>> = {
  group: "/group_nodes",
  defer: "/defer",
  write_up: "/write_up",
  edit_draft: "/edit_draft",
  edit_link: "/edit_link",
  settle: "/settle",
  dispatch: "/dispatch_draft",
  read_in: "/read_in",
};

export class StudioReads {
  private readonly publish: Publish;
  private readonly port: () => number | null;
  /** The repository whose list is held, or `null` where no surface holds one. */
  private listing: string | null = null;
  /** The Studio held open, or `null`. */
  private opened: string | null = null;
  private list: StudiosRead = { state: "none" };

  constructor(publish: Publish, port: () => number | null) {
    this.publish = publish;
    this.port = port;
  }

  /** Hold one repository's list, or `null` to drop it. */
  async watchList(manifestId: string | null): Promise<void> {
    this.listing = manifestId;
    if (manifestId === null) return this.showList({ state: "none" });
    await this.readList();
  }

  /** Hold one Studio, or `null` to drop it. */
  async watchStudio(studioId: string | null): Promise<void> {
    this.opened = studioId;
    if (studioId === null) return this.publish({ studio: { state: "none" } });
    await this.readStudio();
  }

  /** Read both again, where held — a resync, a reconnection, or Refresh. */
  async again(): Promise<void> {
    await Promise.all([this.readList(), this.readStudio()]);
  }

  /** `studio.changed`, or an act's own answer: the Studio whole, as Fleet wrote it. */
  changed(studio: Studio): void {
    if (this.opened === studio.id) this.publish({ studio: { state: "read", studio } });
    if (this.list.state === "read" && this.list.manifestId === studio.manifest_id) {
      this.showList({ ...this.list, list: { studios: foldStudio(this.list.list.studios, studio) } });
    }
  }

  /** `studio.deleted`: gone from the list, and the open Studio says so rather than failing. */
  deleted(gone: StudioDeleted): void {
    if (this.opened === gone.id) this.publish({ studio: { state: "gone", studioId: gone.id } });
    if (this.list.state === "read" && this.list.manifestId === gone.manifest_id) {
      const studios = this.list.list.studios.filter((one) => one.id !== gone.id);
      this.showList({ ...this.list, list: { studios } });
    }
  }

  /** Start an untitled Studio in one repository. */
  async create(manifestId: string): Promise<StudioAnswer> {
    const answer = await this.act(`/studios/create?manifest_id=${encodeURIComponent(manifestId)}`, {});
    if (answer.ok) this.changed(answer.studio);
    return answer;
  }

  /**
   * Put a Note where a person pointed — #1290. **The frame is staged and named,
   * never sent**: Fleet copies it into the Studio's own keeping, so a frame
   * Bridge could not take leaves a Note that carries none rather than failing.
   *
   * **It lands under what is already there.** A Studio is laid out by hand, so
   * the placement only has to be somewhere a person can find it — and two Notes
   * at one point read as one Note.
   */
  async captureNote(
    studioId: string,
    said: string,
    capture: StudioCapture,
    frame: StagedFrame | null,
  ): Promise<Outcome> {
    const body: CaptureStudioNote = {
      said,
      capture,
      position: { x: 0, y: await this.under(studioId) },
      ...(frame === null ? {} : { frame }),
    };
    return this.acted(await this.act(member(studioId, "/capture_note"), body));
  }

  /**
   * The picture one Note kept — #1352.
   *
   * **Answered to the caller, never published.** A frame is hundreds of
   * kilobytes and a Studio is kept until a person deletes it, so holding one in
   * the state every window reads would keep every Note's picture alive for as
   * long as the Studio is open. The bytes stop in main on their way to a
   * `blob:` the renderer makes itself: the CSP's `img-src 'self' blob:` is what
   * draws them, and no scheme was added to it.
   */
  async frameOf(studioId: string, nodeId: string): Promise<FrameRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    return await studioFrameOf(port, studioId, nodeId);
  }

  /** A row below the lowest node on the Studio, or the origin on an empty one. */
  private async under(studioId: string): Promise<number> {
    const port = this.port();
    if (port === null) return 0;
    const answer = await ask(port, "GET", member(studioId));
    if (!answer.ok) return 0;
    const nodes = (answer.body as Studio).nodes;
    return nodes.length === 0 ? 0 : Math.max(...nodes.map((node) => node.position.y)) + NOTE_APART;
  }

  /** Name a Studio, or name it again. A person's act as much as Helm's — #1364. */
  async rename(studioId: string, name: string): Promise<Outcome> {
    return this.acted(await this.act(member(studioId, "/rename"), { name }));
  }

  /**
   * Put a Note, a Link or a Sketch on a Studio where the person is looking —
   * #1364. **The kind is the narrow one**, so nothing the renderer can ask for
   * is a kind Fleet would refuse as `fleet.studio_node_not_a_persons`.
   */
  async addNode(studioId: string, node: StudioNodeByHand, position: StudioPosition): Promise<Outcome> {
    return this.acted(await this.act(member(studioId, "/add_node"), { ...node, position }));
  }

  async moveNode(studioId: string, nodeId: string, position: StudioPosition): Promise<Outcome> {
    return this.acted(await this.act(member(studioId, "/move_node"), { node_id: nodeId, position }));
  }

  async removeNode(studioId: string, nodeId: string): Promise<Outcome> {
    return this.acted(await this.act(member(studioId, "/remove_node"), { node_id: nodeId }));
  }

  async decideEdge(studioId: string, edgeId: string, accepted: boolean): Promise<Outcome> {
    return this.acted(await this.act(member(studioId, "/decide_edge"), { edge_id: edgeId, accepted }));
  }

  /**
   * One rung of promotion — #1291. **`act` picks the route and is not sent**: each of Fleet's
   * operations takes its own body, and the tag is how one capability on the bridge reaches them
   * all. Every one answers with the Studio whole, so they all fold the one way.
   */
  async promote(studioId: string, promotion: StudioPromotion): Promise<Outcome> {
    const { act, ...body } = promotion;
    return this.acted(await this.act(member(studioId, PROMOTION_ROUTE[act]), body));
  }

  close(): void {
    this.listing = null;
    this.opened = null;
    this.list = { state: "none" };
  }

  private acted(answer: StudioAnswer): Outcome {
    if (!answer.ok) return answer.outcome;
    this.changed(answer.studio);
    return { ok: true };
  }

  private async act(path: string, body: unknown): Promise<StudioAnswer> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "POST", path, body);
    return answer.ok ? { ok: true, studio: answer.body as Studio } : { ok: false, outcome: answer.outcome };
  }

  private async readList(): Promise<void> {
    const manifestId = this.listing;
    if (manifestId === null) return;
    const port = this.port();
    if (port === null) return this.showList({ state: "failed", manifestId, outcome: NOT_CONNECTED });
    if (this.list.state !== "read" || this.list.manifestId !== manifestId) {
      this.showList({ state: "reading", manifestId });
    }
    const answer = await ask(port, "GET", `/studios?manifest_id=${encodeURIComponent(manifestId)}`);
    // The surface closed or moved to another repository while this was out.
    if (this.listing !== manifestId) return;
    this.showList(
      answer.ok
        ? { state: "read", manifestId, list: answer.body as StudioList }
        : { state: "failed", manifestId, outcome: answer.outcome },
    );
  }

  private async readStudio(): Promise<void> {
    const studioId = this.opened;
    if (studioId === null) return;
    const port = this.port();
    if (port === null) return this.publish({ studio: { state: "failed", studioId, outcome: NOT_CONNECTED } });
    const answer = await ask(port, "GET", member(studioId));
    if (this.opened !== studioId) return;
    this.publish({
      studio: answer.ok
        ? { state: "read", studio: answer.body as Studio }
        : { state: "failed", studioId, outcome: answer.outcome },
    });
  }

  private showList(list: StudiosRead): void {
    this.list = list;
    this.publish({ studios: list });
  }
}
