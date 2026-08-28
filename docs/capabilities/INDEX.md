# Capabilities

One file per capability, paired to one GitHub issue by its frontmatter. The
issue tracks — its steps are sub-issues, so progress is computed rather than
reported. The file holds the prose, the rationale and the links to the concepts
it depends on, because an issue body buries all of that the moment the issue
closes.

The frontmatter is the binding: `capability:` must match the filename, `issue:`
must name the issue that tracks it, and `milestone:` is the milestone it sits
under. `cargo xtask verify-foundations` checks the shape and this index offline;
`cargo xtask verify-roadmap` checks that the issue exists.

A capability acquires a file when it acquires prose worth keeping. One that is
only its steps does not need one.

- [`drone-per-step.md`](drone-per-step.md) — a Drone belongs to a workflow step
  rather than to a Job, ending when its step ends so that what crosses the
  boundary is the record rather than the session.
