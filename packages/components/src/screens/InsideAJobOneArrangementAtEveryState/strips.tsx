import type { PhaseStripProps } from "../../compositions/PhaseStrip/PhaseStrip";

/**
 * Where a step stands, one strip per state the screen draws.
 *
 * **Their own file, beside `evidence.tsx` and `playable.tsx`.** A strip is
 * fifteen lines of data and a story is a claim about one moment, so six of them
 * typed into the stories said almost nothing about what made each state
 * different — and put both that file and `fixtures.tsx` past the length the
 * gate refuses. A fixture file per subject is the pattern this directory
 * already keeps.
 */

export const PHASES_WORKING: PhaseStripProps = {
            note: "The Drone is working. Nothing has been submitted, so no gate has been asked anything yet.",
            stages: [
              { id: "instructed", label: "Instructed", state: "cleared" },
              { id: "working", label: "Working", state: "current" },
              { id: "submitted", label: "Submitted", state: "ahead" },
              {
                id: "checks",
                label: "build, test",
                kind: "checks",
                state: "ahead",
                stands: "not run",
                rows: [
                  { label: "cargo build --workspace --locked", mono: true, result: "not run" },
                  { label: "cargo nextest run --workspace", mono: true, result: "not run" },
                ],
              },
              {
                id: "judge",
                label: "Judge · 2 criteria",
                kind: "judge",
                state: "ahead",
                stands: "not reached",
                rows: [
                  { label: "Selectors import without the store", result: "not reached" },
                  { label: "No behaviour change in the reducer", result: "not reached" },
                ],
              },
              { id: "you", label: "You", kind: "human", state: "ahead" },
            ],
          };

/**
 * **`noteOf`'s own sentence for a step at a human gate.** It read "The suite
 * passed and the Judge met both criteria. Nothing is wrong; the workflow asks
 * for a person here." — a sentence the product does not have, restating the two
 * tiers the strip above it already draws and opening on a denial.
 */
export const PHASES_AT_THE_GATE: PhaseStripProps = {
            note: "Everything mechanical has cleared. The workflow asks for a person here.",
            stages: [
              { id: "instructed", label: "Instructed", state: "cleared" },
              { id: "working", label: "Working", state: "cleared" },
              { id: "submitted", label: "Submitted", state: "cleared" },
              {
                id: "checks",
                label: "build, test",
                kind: "checks",
                state: "cleared",
                stands: "2 of 2 passed",
                rows: [
                  { label: "cargo build --workspace --locked", mono: true, result: "exit 0 · 47s", named: "passed" },
                  { label: "cargo nextest run --workspace", mono: true, result: "exit 0 · 1m 22s", named: "passed" },
                ],
              },
              {
                id: "judge",
                label: "Judge · 2 of 2 met",
                kind: "judge",
                state: "cleared",
                stands: "2 of 2 met",
                rows: [
                  { label: "Selectors import without the store", result: "met", named: "met" },
                  { label: "No behaviour change in the reducer", result: "met", named: "met" },
                ],
              },
              { id: "you", label: "You", kind: "human", state: "waiting", stands: "waiting · 2m 04s" },
            ],
          };

/** `noteOf`'s sentence for a step the gate has run on and handed back. */
export const PHASES_CHECK_FAILED: PhaseStripProps = {
            note: "The gate has run and the Drone has the step back. The tiers behind it are still ahead, not cancelled.",
            stages: [
              { id: "instructed", label: "Instructed", state: "cleared" },
              { id: "working", label: "Working", state: "current" },
              { id: "submitted", label: "Submitted", state: "cleared" },
              {
                id: "checks",
                label: "test failed · fixing",
                kind: "checks",
                state: "failed",
                stands: "exit 101 · attempt 2 of 3",
                rows: [
                  { label: "cargo build --workspace --locked", mono: true, result: "exit 0 · 47s", named: "passed" },
                  { label: "cargo nextest run --workspace", mono: true, result: "exit 101 · 3 failures", named: "failed" },
                ],
              },
              { id: "judge", label: "Judge · 2 criteria", kind: "judge", state: "ahead", stands: "not reached" },
              { id: "you", label: "You", kind: "human", state: "ahead" },
            ],
          };

export const PHASES_RETRIES_SPENT: PhaseStripProps = {
            note: "A Check that fails ends the step before the Judge reads anything, so there is no verdict here. The Judge tier was never reached.",
            stages: [
              { id: "instructed", label: "Instructed", state: "cleared" },
              { id: "working", label: "Working", state: "cleared" },
              { id: "submitted", label: "Submitted", state: "cleared" },
              {
                id: "checks",
                label: "test failed · retries spent",
                kind: "checks",
                state: "failed",
                stands: "exit 101 · 3 of 3 attempts",
                rows: [
                  { label: "cargo build --workspace --locked", mono: true, result: "exit 0 · 47s", named: "passed" },
                  { label: "cargo nextest run --workspace", mono: true, result: "exit 101 · same failure ×3", named: "failed" },
                ],
              },
              { id: "judge", label: "Judge · 2 criteria", kind: "judge", state: "ahead", stands: "not reached" },
              { id: "you", label: "You", kind: "human", state: "ahead" },
            ],
          };

export const PHASES_BLOCKED: PhaseStripProps = {
            note: "Nothing was submitted, so no gate has been asked anything. The drone stopped reaching for what it needed.",
            stages: [
              { id: "instructed", label: "Instructed", state: "cleared" },
              { id: "working", label: "Working", state: "current" },
              { id: "submitted", label: "Submitted", state: "ahead" },
              { id: "checks", label: "Checks", kind: "checks", state: "ahead", stands: "not reached" },
              { id: "judge", label: "Judge · 2 criteria", kind: "judge", state: "ahead", stands: "not reached" },
              { id: "you", label: "You", kind: "human", state: "waiting", stands: "waiting · 3m 02s" },
            ],
          };

export const PHASES_ENDED: PhaseStripProps = {
            note: "Nothing advances this Job. Redispatch mints a replacement; it does not reopen this one.",
            stages: [
              { id: "instructed", label: "Instructed", state: "cleared" },
              { id: "working", label: "Working", state: "cleared" },
              { id: "submitted", label: "Submitted", state: "cleared" },
              {
                id: "checks",
                label: "test failed",
                kind: "checks",
                state: "failed",
                stands: "exit 101",
                rows: [
                  { label: "cargo build --workspace --locked", mono: true, result: "exit 0 · 47s", named: "passed" },
                  { label: "cargo nextest run --workspace", mono: true, result: "exit 101", named: "failed" },
                ],
              },
              { id: "judge", label: "Judge · 2 criteria", kind: "judge", state: "ahead", stands: "not reached" },
            ],
          };
