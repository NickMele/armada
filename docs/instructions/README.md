# Instruction sets

The one surface that is not this repository carries its own instructions, pasted
in by hand — the platform reads no file from here. The canonical text lives in
this directory so it can be diffed, and so it says the same thing about where a
fact lives as the repository does rather than an approximation of it.

| File | Pasted into |
|---|---|
| `claude-desktop.md` | The Armada project's instructions in the Claude desktop app |

**The file is exactly what gets pasted, byte for byte.** No header, no note
about being canonical — anything in the file would go into the destination
along with it. That is what this README is for.

**Edit here first, then paste.** A change made only in the app is a change the
next person cannot find.

## What it must agree with

| Holds | Home |
|---|---|
| Contracts, practices, concepts, journeys, spikes, open questions | The repository |
| Registries code reads — tokens, icons, settings, the domain model | The repository |
| What is being built and in what order | GitHub issues and milestones |
| A component as built, and how it is agreed | Storybook, `packages/components` |

**Nothing sits outside the repository.** A component that should exist and does
not is a GitHub issue.
