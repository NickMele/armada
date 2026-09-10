import { useCallback, useState, type ReactNode } from "react";
import { Button } from "../../primitives/Button/Button";
import { ActivityLog } from "../../compositions/ActivityLog/ActivityLog";
import {
  ActivityLogSheet,
  type ActivityFilter,
} from "../../compositions/ActivityLogSheet/ActivityLogSheet";
import { ConsoleOutput } from "../../compositions/ConsoleOutput/ConsoleOutput";
import { EvidenceSheet, type EvidenceView } from "../../compositions/EvidenceSheet/EvidenceSheet";
import { EvidenceStrip } from "../../compositions/EvidenceStrip/EvidenceStrip";
import { JudgeCitations } from "../../compositions/JudgeCitations/JudgeCitations";
import { JudgeVerdicts } from "../../compositions/JudgeVerdicts/JudgeVerdicts";
import { Absent } from "../absent";
import { CHAPTERS, JOB, WHOLE, chapterAct } from "./fixtures";
import {
  API_OUTPUT,
  ATTEMPT,
  BENCH_MEASURED,
  BENCH_OUTPUT,
  GLYPHS,
  JUDGES,
  MIGRATION_DOC,
  PANEL_CITATIONS,
  PANEL_INPUTS_VIEW,
  PRODUCED_CHIPS,
  PRODUCED_LABEL,
  REFUSED_DECISION,
  SUITE_ASSERTIONS,
  SUITE_OUTPUT,
  PATCH,
  SYMBOLS,
  aRefusedJob,
  evidenceChapters,
  inFile,
  patchOf,
  verdictRows,
} from "./evidence";

/**
 * The evidence surface with its selectors wired — the same refused Job the
 * stories draw, holding the state that makes it pressable.
 *
 * **Every selector on this screen already emits an artifact id.** A check row
 * emits `chk-suite`, a chip emits `bench`, a citation emits `diff-loose`, a
 * run-tree fact emits `chk-suite@1`, a phase-card row emits `chk-api`. So the
 * surface resolves the id against one registry, and no control on the screen
 * knows what a viewer renders — which is the only arrangement that stays true
 * once the wire serves these rather than a fixture file.
 *
 * **An id nothing serves draws a named absence.** It is a real state — a
 * citation into a log that was cleaned up with the worktree is exactly it — and
 * a viewer that opened empty would look like one that failed to load.
 */

/**
 * Which sheet is open. **One slot, two kinds of reading.**
 *
 * The viewer holds artifacts — a kind, a file, finished, something a citation
 * can point into. A step's activity log is none of those: it is a live stream
 * with filters, a held reading and a count of what arrived while you were
 * reading, so it keeps its own sheet and shares the slot. Opening either closes
 * the other, which is the rule the screen already keeps — `Esc` returns to the
 * panel, never to the sheet before.
 */
type Open = { log: true } | { artifact: string } | null;

const isLog = (open: Open): open is { log: true } => open !== null && "log" in open;

/** One rendering of one artifact: what the sheet's header says, and what it draws. */
type Rendering = { kind: string; name: ReactNode; extent?: ReactNode; body: ReactNode };

/** The key an artifact with a single rendering files it under. */
const ONE = "one";

type Artifact = {
  /** The tabs. Fewer than two draws none, which `EvidenceSheet` already decides. */
  views?: EvidenceView[];
  /** Which rendering a selector naming this artifact lands on. */
  opens: string;
  at: Record<string, Rendering>;
};

function attemptLog(nth: number): Artifact {
  return {
    opens: ONE,
    at: {
      [ONE]: {
        kind: "Console output",
        name: `check:test_suite — attempt ${nth} of 3`,
        extent: "1,208 lines · 2.2MB",
        body: (
          <ConsoleOutput
            rows={ATTEMPT}
            region={{
              says: "lines 1,196–1,208 of 1,208",
              path: `.armada/jobs/${JOB}/checks/test_suite.${nth}.log`,
              why: "jumped to the failure",
            }}
          />
        ),
      },
    },
  };
}

/** An artifact with one rendering and no tabs. */
function only(rendering: Rendering): Artifact {
  return { opens: ONE, at: { [ONE]: rendering } };
}

// ----------------------------------------------------------------- registry

/**
 * Every artifact this step produced, by the id its selectors emit.
 *
 * The Check ids are the Manifest's, the attempt ids are the Check's with the
 * attempt on them, and the diff ids are the ones the panel cited — none of them
 * is invented here, which is what keeps the registry a lookup rather than a
 * second vocabulary.
 */
function artifacts(select: (artifactId: string) => void): Record<string, Artifact> {
  return {
    "chk-suite": {
      views: [
        { id: "output", label: "Output" },
        { id: "assertions", label: "Assertions" },
      ],
      opens: "output",
      at: {
        output: {
          kind: "Console output",
          name: "check:test_suite — output",
          extent: "2,180 lines · 4.1MB",
          body: SUITE_OUTPUT,
        },
        assertions: {
          kind: "Test results",
          name: "check:test_suite — assertions",
          extent: "315 assertions",
          body: SUITE_ASSERTIONS,
        },
      },
    },
    "chk-api": {
      views: [
        { id: "output", label: "Output" },
        { id: "measurement", label: "Measurement" },
      ],
      opens: "output",
      at: {
        output: {
          kind: "Console output",
          name: "check:public_api — output",
          extent: "44 lines",
          body: <ConsoleOutput rows={API_OUTPUT} region={{ says: "the whole output" }} />,
        },
        measurement: {
          kind: "Measurement",
          name: "check:public_api — measured",
          extent: "41 symbols",
          body: (
            <ConsoleOutput
              rows={SYMBOLS}
              region={{ says: "the exported surface at HEAD", size: "41 symbols" }}
            />
          ),
        },
      },
    },
    "chk-bench": {
      views: [
        { id: "output", label: "Output" },
        { id: "measurement", label: "Measurement" },
      ],
      opens: "output",
      at: {
        output: {
          kind: "Console output",
          name: "check:bench — output",
          extent: "96 lines",
          body: (
            <ConsoleOutput
              rows={BENCH_OUTPUT}
              region={{ says: "lines 1–96 of 96", why: "jumped to the reported time" }}
            />
          ),
        },
        measurement: {
          kind: "Measurement",
          name: "check:bench — measured",
          extent: "1.42 → 1.19µs",
          body: (
            <ConsoleOutput
              rows={BENCH_MEASURED}
              region={{ says: "what the Judge was handed", why: "the cap is the criterion" }}
            />
          ),
        },
      },
    },
    judge: {
      views: [
        { id: "verdicts", label: "Verdicts" },
        { id: "citations", label: "Citations" },
        { id: "inputs", label: "Inputs" },
      ],
      opens: "verdicts",
      at: {
        verdicts: {
          kind: "Judge verdicts",
          name: "The Judge — what the panel found",
          extent: "4 criteria · 3 judges",
          body: (
            <JudgeVerdicts judges={JUDGES} glyphs={GLYPHS} rows={verdictRows(select)} openId="c2" />
          ),
        },
        citations: {
          kind: "Judge verdicts",
          name: "The Judge — what the panel cited",
          extent: "5 citations",
          // Each row carries its own destination, because a citation names an
          // artifact rather than a place in this one.
          body: (
            <JudgeCitations
              rows={PANEL_CITATIONS.map((row) => ({ ...row, onOpen: select }))}
            />
          ),
        },
        inputs: {
          kind: "Judge verdicts",
          name: "The Judge — what the panel was shown",
          extent: "38.2k of 40k",
          body: PANEL_INPUTS_VIEW,
        },
      },
    },
    diff: only({
      kind: "Code diff",
      name: "the patch",
      extent: "−318 +94 · 5 files",
      body: patchOf(PATCH, "read from the worktree at 15:02:44"),
    }),
    "diff-loose": only({
      kind: "Code diff",
      name: "tests/loose.rs — the deleted lines",
      extent: "−14",
      body: patchOf(inFile("tests/loose.rs"), "the region j3 cited"),
    }),
    "diff-mod": only({
      kind: "Code diff",
      name: "src/parse/mod.rs — the entry point",
      extent: "+94",
      body: patchOf(inFile("src/parse/mod.rs"), "the region j1 and j2 cited"),
    }),
    "diff-lib": only({
      kind: "Code diff",
      name: "src/lib.rs — the surviving re-export",
      extent: "+1",
      body: patchOf(inFile("src/lib.rs"), "the region j2 cited"),
    }),
    migration: only({
      kind: "Document",
      name: "MIGRATION.md",
      extent: "11 lines",
      body: MIGRATION_DOC,
    }),
    "chk-suite@1": attemptLog(1),
    "chk-suite@2": attemptLog(2),
    "chk-suite@3": attemptLog(3),
  };
}

/**
 * A chip that names a rendering rather than an artifact of its own.
 *
 * **The strip's words are the reason.** `assertion set` and `bench 1.42 →
 * 1.19µs` are what a Check's one output looks like drawn two ways, so they
 * resolve to a view of that Check rather than to a registry entry that would
 * hold the same file twice.
 */
const CHIP_VIEWS: Record<string, { of: string; view: string }> = {
  assertions: { of: "chk-suite", view: "assertions" },
  sym: { of: "chk-api", view: "measurement" },
  bench: { of: "chk-bench", view: "measurement" },
};

/** Which chip the strip marks: the alias that is showing, or the artifact itself. */
function chipShowing(id: string, view: string): string | null {
  const alias = Object.keys(CHIP_VIEWS).find(
    (chip) => CHIP_VIEWS[chip]!.of === id && CHIP_VIEWS[chip]!.view === view,
  );
  if (alias !== undefined) return alias;
  return PRODUCED_CHIPS.some((chip) => chip.id === id) ? id : null;
}

// -------------------------------------------------------------- the surface

export function PlayableJob() {
  const [open, setOpen] = useState<Open>(null);
  /** `null` is the artifact's own opening view. Only a tab names one. */
  const [view, setView] = useState<string | null>(null);
  const [filter, setFilter] = useState<ActivityFilter>("all");

  // A selector says which artifact, never which rendering — except a chip that
  // is a rendering, which is the only thing `CHIP_VIEWS` holds. So the registry
  // is read at draw time and nothing here has to know what it contains.
  const select = useCallback((selectorId: string) => {
    const alias = CHIP_VIEWS[selectorId];
    setOpen({ artifact: alias?.of ?? selectorId });
    setView(alias?.view ?? null);
  }, []);

  const id = open === null || isLog(open) ? null : open.artifact;
  const registry = artifacts(select);
  const artifact = id === null ? undefined : registry[id];
  const showingView = view ?? artifact?.opens ?? ONE;
  const showing = artifact?.at[showingView];

  return aRefusedJob({
    fields: [
      { label: "Took", value: "8m 41s", mono: true },
      { label: "Attempt", value: "3 of 3", mono: true },
      { label: "Refused at", value: "15:02:44", mono: true },
    ],
    acts: <Button variant="secondary">Re-run the gate</Button>,
    notice: {
      tone: "stopped",
      title: "All mechanical checks passed, but the panel of judges refused the work.",
      children:
        "The suite is green because one of its cases stopped existing. 2 of the 3 judges refused criterion 02, and the assertion they both cite is in the check output below.",
    },
    onOpenArtifact: select,
    chapters: [
      // The diff is an artifact and lands in the viewer; the log is a stream
      // and lands in its own sheet. Two acts, one slot.
      ...CHAPTERS.map((chapter) =>
        chapter.id === "produced"
          ? { ...chapter, act: chapterAct("Open the diff", "f", () => select("diff")) }
          : chapter.id === "log"
            ? { ...chapter, act: chapterAct("Open the log", "L", () => setOpen({ log: true })) }
            : chapter,
      ),
      ...evidenceChapters({ openId: id, onOpen: select }),
    ],
    after: REFUSED_DECISION,
    sheet: isLog(open) ? (
      <ActivityLogSheet
        open
        step="Verify"
        jobId={JOB}
        total={WHOLE.length}
        // The Job is stopped, so the stream has an end rather than a live mark.
        endedAt="15:02:44"
        filter={filter}
        onFilter={setFilter}
        onClose={() => setOpen(null)}
      >
        <ActivityLog entries={WHOLE.filter((e) => filter === "all" || e.actor === filter)} />
      </ActivityLogSheet>
    ) : id === null ? undefined : (
      <EvidenceSheet
        open
        kind={showing?.kind ?? "Nothing serves this"}
        name={showing?.name ?? id}
        step="Verify"
        jobId={JOB}
        extent={showing?.extent}
        views={artifact?.views}
        view={showingView}
        onView={setView}
        strip={
          <EvidenceStrip
            chips={PRODUCED_CHIPS}
            openId={chipShowing(id, showingView)}
            label={PRODUCED_LABEL}
            onOpen={select}
          />
        }
        onClose={() => setOpen(null)}
      >
        {showing?.body ?? (
          <Absent
            name={id}
            note="No artifact is registered under this id, so the viewer has nothing to draw. The id is what the selector emitted."
          />
        )}
      </EvidenceSheet>
    ),
  });
}
