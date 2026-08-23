# crates/

The Rust workspace. Read `docs/practices/rust.md` before writing here — it holds
the crate boundaries, the type-system-first pattern, and the reasons.

- Every file under `crates/*/src/` must be in `foundations-manifest.txt`, sorted,
  added in the same change as the file.
- No `serde_json::from_*` outside `store` and `ipc`.
- No vendor literal outside `adapters`.
- 500 lines asks, 900 lines refuses.
- `cargo nextest run --workspace`, not `cargo test`.

All of these are enforced by a hook before the write and by the gate after it.
