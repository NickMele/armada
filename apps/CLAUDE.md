# apps/

Bridge, the Electron desktop application. Read `docs/practices/bridge.md` before
writing here.

- A surface builds from `packages/tokens` and shadcn primitives alone, with
  lucide-react for icons. Nothing invented.
- Bridge holds one connection, to Armada API. It never talks to a Drone.
- Every capability added to the preload bridge is one the renderer can reach.

Anything crossing to Rust goes through `docs/practices/protocol.md` first.
