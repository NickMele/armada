// Kit's MCP servers, read for the Manifest surface and changed there. #1275.
//
// `repository-allows.ts`'s shape: no run and no file, so `port()` and the pick
// are what a route under the picked Manifest needs. Every act answers with the
// whole list, so there is one read type and one fold.

import type {
  AddKitServer,
  KitInventory,
  ForgetKitServer,
  KitServers,
  ManifestReach,
  ReachesDrones,
  SetKitServerReach,
  SetManifestServerReach,
} from "@armada/protocol";
import type { KitInventoryRead, KitServersRead } from "@armada/screens/src/manifest-kit";

import type { Picked } from "./picked";
import { ask, NOT_SET_UP } from "./request";

/** `get_kit_servers` and the four acts under it. */
export class KitCommands {
  private readonly port: () => number | null;
  private readonly picked: Picked;

  constructor(port: () => number | null, picked: Picked) {
    this.port = port;
    this.picked = picked;
  }

  /**
   * The setup a person already has, read to be shown — #1491. **Machine-wide**,
   * so no pick scopes it and the route takes no Manifest.
   */
  async inventory(): Promise<KitInventoryRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const answer = await ask(port, "GET", "/kit/inventory");
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, setup: answer.body as KitInventory };
  }

  /** Every server in Kit, with what a Drone dispatched here resolves. */
  async list(): Promise<KitServersRead> {
    return this.read("GET", "/kit/servers");
  }

  /** Put one in Kit. It reaches no Drone until a person says so. */
  async add(adding: AddKitServer): Promise<KitServersRead> {
    return this.read("POST", "/kit/servers/add", adding);
  }

  /** Take one out, and every Manifest's word about it with it. */
  async forget(name: string): Promise<KitServersRead> {
    const body: ForgetKitServer = { name };
    return this.read("POST", "/kit/servers/forget", body);
  }

  /** Kit's own tier, across every Manifest that has not said otherwise. */
  async setKitReach(name: string, drones: ReachesDrones): Promise<KitServersRead> {
    const body: SetKitServerReach = { name, drones };
    return this.read("POST", "/kit/servers/reach", body);
  }

  /**
   * This Manifest's own word, or `null` to take it back.
   *
   * **`null` is sent rather than left out**, which is what Fleet refuses a body
   * without: a dropped key would throw a person's word away and answer 200.
   */
  async setManifestReach(name: string, reach: ManifestReach | null): Promise<KitServersRead> {
    const body: SetManifestServerReach = { name, reach };
    return this.read("POST", "/kit/servers/manifest_reach", body);
  }

  private async read(method: "GET" | "POST", route: string, body?: unknown): Promise<KitServersRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const path = this.picked.manifest(route);
    if (path === null) return { ok: false, outcome: NOT_SET_UP };
    const answer = await ask(port, method, path, body);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, kit: answer.body as KitServers };
  }
}
