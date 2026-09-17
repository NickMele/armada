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

import type { Outcome, Studio, StudioDeleted, StudioList, StudioPosition } from "@armada/protocol";
import { foldStudio } from "@armada/screens/src/studio-reads";
import type { StudioAnswer, StudioRead, StudiosRead } from "@armada/screens/src/studio-reads";
import { ask } from "./request";

type Publish = (change: { studios?: StudiosRead; studio?: StudioRead }) => void;

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

const member = (studioId: string, act = "") => `/studios/${encodeURIComponent(studioId)}${act}`;

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

  async moveNode(studioId: string, nodeId: string, position: StudioPosition): Promise<Outcome> {
    return this.acted(await this.act(member(studioId, "/move_node"), { node_id: nodeId, position }));
  }

  async removeNode(studioId: string, nodeId: string): Promise<Outcome> {
    return this.acted(await this.act(member(studioId, "/remove_node"), { node_id: nodeId }));
  }

  async decideEdge(studioId: string, edgeId: string, accepted: boolean): Promise<Outcome> {
    return this.acted(await this.act(member(studioId, "/decide_edge"), { edge_id: edgeId, accepted }));
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
