# apps/

Bridge, the Electron desktop application. Read `docs/practices/bridge.md` before
writing here.

- A surface builds from `packages/tokens` and shadcn primitives alone, with
  lucide-react for icons. Nothing invented.
- Bridge talks to one peer, Armada API, and never to a Drone. One socket carries
  the Board; a second, opened per Job, carries one Drone's turns and is read-only.
- Every capability added to the preload bridge is one the renderer can reach.

Anything crossing to Rust goes through `docs/practices/protocol.md` first.
