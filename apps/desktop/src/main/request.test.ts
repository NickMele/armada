// What importing the HTTP half does to the process it is imported into.
//
// **The one thing in `request.ts` that is not a function call.** Everything
// else there is a `fetch` with a timeout and is proved by the files that use
// it; this is a change to `node:net` made at import, and nothing else would
// notice if it stopped happening.

import { createServer, type Server, Socket } from "node:net";
import { once } from "node:events";
import type { AddressInfo } from "node:net";

import { afterEach, expect, it } from "vitest";

import "./request";
import { HOST } from "./runtime-file";

/** Everything one case opens, closed in the order it was opened. */
const opened: (() => void)[] = [];

afterEach(() => {
  while (opened.length > 0) opened.pop()?.();
});

/** `setTypeOfService` is Node 24's and is in no `@types/node` this repo has. */
function setting(socket: Socket): (tos: number) => unknown {
  const setter = (socket as unknown as { setTypeOfService?: (tos: number) => unknown })
    .setTypeOfService;
  expect(setter, "this Node has no setTypeOfService to guard").toBeTypeOf("function");
  return (tos) => setter?.call(socket, tos);
}

/** A listener to aim at, so nothing here depends on a port being free. */
async function listening(): Promise<Server> {
  const server = createServer((socket) => socket.destroy());
  server.listen(0, HOST);
  await once(server, "listening");
  opened.push(() => server.close());
  return server;
}

it("does not throw when the OS refuses the type-of-service byte", async () => {
  const server = await listening();
  const socket = new Socket();
  socket.on("error", () => {});
  opened.push(() => socket.destroy());

  // A socket that has a handle and no file descriptor yet: `connect` makes the
  // handle and libuv opens the descriptor when it attempts the connection, so
  // the syscall is refused here for the same reason it is refused on a socket
  // the peer has already reset — the descriptor is not one it can be set on.
  socket.connect((server.address() as AddressInfo).port, HOST);

  expect(() => setting(socket)(1)).not.toThrow();
});

it("still throws on an argument no socket would accept", async () => {
  const server = await listening();
  const socket = new Socket();
  socket.on("error", () => {});
  opened.push(() => socket.destroy());
  socket.connect((server.address() as AddressInfo).port, HOST);

  // The guard is the syscall's errno and nothing else. Node's own range check
  // runs ahead of it and is untouched, which is what says this is not a catch
  // around the call.
  expect(() => setting(socket)(999)).toThrow();
});
