//! Kit and Manifest resolution, and the merge strategies between them.
//!
//! Owns scan, propose, select and verify — the part of v1 that ported most
//! cleanly — plus Check and Command definitions, and the Kit and Manifest health
//! probes, which read and validate their own files.
//!
//! A Manifest is an `armada.yml` at a workspace root, version-controlled with
//! the project it configures. Path ownership is nearest-ancestor: the nearest
//! `armada.yml` up the tree owns a path, and the root owns whatever no Workspace
//! claims.
