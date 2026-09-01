---
name: Something is broken
about: The code does something other than what it says it does
labels: bug
---

<!--
The rules are in docs/practices/writing-an-issue.md. Read it once; this file
is only the shape.

The one rule that matters: the paragraph below has no heading and comes first.
It says what a person cannot do. Everything else is for whoever implements it.
-->

Two or three sentences, no heading, before anything else. Name the person and
what they cannot do — not the mechanism, not where it was found, not the file.
If a real person hit this, say when and what it cost them.

## What happened

The symptom, then what the code actually does. Exact error text and the command
that shows it. This is where the file paths, the code block and the mechanism
go — second, not first.

## In

What has to change, and **what already exists that it builds on**. Name the file
and the line. An issue that makes the reader rediscover what you already found
wastes the work.

## Watch for

The wrong fix. Every bug has one and it is usually the first thing that comes to
mind. Say why it is wrong, not that it is.

## Definition of done

One sentence, checkable, in the present tense as a person experiences it.

## How this was found

Optional, and last. Provenance, the run that surfaced it, the prior attempt that
was abandoned. Load-bearing for whoever picks this up, and never the first thing
read.
