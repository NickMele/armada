import { contextBridge } from 'electron'

// The whole surface the renderer is allowed to see. It is deliberately almost
// empty: every capability added here is one the renderer can reach, so the
// list is meant to stay short enough to read in one go.
contextBridge.exposeInMainWorld('armada', {
  // **A placeholder, and a known lie.** The protocol version has one source of
  // truth — `protocol-version.toml` at the repo root — which `crates/ipc/build.rs`
  // already reads for the Rust side. The TypeScript side is meant to be generated
  // from the same file, and until that codegen exists this literal is exactly the
  // drift the shared file was created to prevent. Do not update it by hand; wire
  // the codegen instead.
  protocolVersion: (): number => 1,
})
