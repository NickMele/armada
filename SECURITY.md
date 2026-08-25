# Security

Armada runs coding agents against your repositories, brokers credentials to
them, and executes commands a repository declares. The interesting parts of its
security posture are therefore not the usual web ones.

## Reporting something

Open a [private security advisory](https://github.com/NickMele/armada/security/advisories/new).
Please do not open a public issue for anything exploitable.

There is no release and no user other than the author, so there is no embargo
process to speak of. A report will be read and answered.

## What the design already assumes

These are constraints the system is built to hold. If you find one that does
not hold, that is a report worth making.

| | |
|---|---|
| **An agent gets only the tools it is handed** | No inheritance from the machine, the shell, or anybody's credentials |
| **An agent cannot push** | The version-control seam handed to a Drone has no push method. The call does not exist rather than being refused |
| **An agent works in its own worktree** | Never the main checkout, always its own branch |
| **A credential never reaches a Drone's environment or logs** | Secrets are brokered through Fleet. The type holding one has no `Debug`, no `Display` and no `Serialize`, so it cannot be printed by accident |
| **An agent cannot mark its own work complete** | Evidence goes through a tool; a mechanical check decides |
| **A repository supplies rules Armada verifies it with** | This is the deliberate soft spot. `docs/contracts/system-architecture.md` carries the threat model for it |

## What is not hardened yet

Everything. There is no release, the acceptance test fails on purpose, and most
of the enforcement above exists as a type or a gate rule rather than as running
code. Do not point this at a repository you care about.
