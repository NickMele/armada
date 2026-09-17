import { useState, type ReactNode } from "react";

import { ChevronDown, ChevronRight } from "lucide-react";

import { StepBar, type TaskBarSegment } from "../StepBar/StepBar";
import { TaskMark, type TaskMarkState } from "../TaskMark/TaskMark";

/**
 * A step's work as the Drone told it, grouped under the plan task it served.
 *
 * **The Drone's sentences are the rows**, drawn as written, and each folds the
 * calls after it behind one line. A section is one plan task, or the work
 * outside any; a section with no heading draws its sentences bare, which is a
 * step with no plan. The rows under a sentence are the caller's log.
 */
export type NarrationBeat = {
  /** Stable across re-renders. The first row's own id. */
  id: string;
  /** When it was said, already formatted. */
  at?: string;
  /** The Drone's sentence, verbatim. Absent on calls made before it said anything. */
  said?: string;
  /** What the fold line says — `3 calls · Edit, Read`. */
  meta?: string;
  /** Drawn open. The newest sentence, and any holding a failure. */
  open?: boolean;
  /** The rows behind the sentence. Absent is a sentence with nothing after it. */
  body?: ReactNode;
};

/**
 * One file a section changed, under its heading. #1187.
 *
 * **What its numbers are is the caller's to say**, in the section's
 * `filesSay`: under a task they are the sizes of the Drone's own edits, and
 * under the row no task owns they are the diff's lines. The two are drawn
 * alike and must never be read alike.
 */
export type NarrationFile = {
  /** Stable, and the whole path a hover shows. */
  path: string;
  /** What the row reads — `daemon/fittings.rs`. */
  name: string;
  added?: number;
  deleted?: number;
};

export type NarrationSection = {
  id: string;
  /** The task it is. Absent draws the sentences bare, with no heading. */
  heading?: {
    /** `T3`, where it is a task. */
    task?: string;
    mark?: TaskMarkState;
    title: string;
  };
  /** What the heading says beside the title — `31 calls · 5m`. */
  meta?: string;
  /** Drawn open. A heading over no sentences never opens. */
  open?: boolean;
  beats: NarrationBeat[];
  /** A line above the sentences, for the ones left out — `12 earlier, in the log`. */
  earlier?: string;
  /** The files it changed, under the heading and drawn whether or not it is open. */
  files?: NarrationFile[];
  /** What the files' numbers are — `sizes of its edits`. Required wherever `files` is. */
  filesSay?: string;
  /** Set apart from the tasks above it: the files no task's edits account for. */
  apart?: boolean;
};

export type WorkNarrationProps = {
  sections: NarrationSection[];
  /** What a step with no work says. Never a blank. */
  emptyNote: string;
};

export function WorkNarration({ sections, emptyNote }: WorkNarrationProps) {
  if (sections.every((section) => section.beats.length === 0 && section.heading === undefined)) {
    return (
      <p className="armada-narration__empty" role="note">
        {emptyNote}
      </p>
    );
  }
  return (
    <div className="armada-narration">
      {sections.map((section) => (
        <Section key={section.id} section={section} />
      ))}
    </div>
  );
}

function Section({ section }: { section: NarrationSection }) {
  // A press wins; until one, the caller decides, so the open task moves on
  // with the work rather than staying where it was on mount.
  const [chosen, setOpen] = useState<boolean | undefined>(undefined);
  const open = chosen ?? section.open === true;
  const beats = (
    <>
      {section.earlier === undefined ? null : (
        <p className="armada-narration__earlier">{section.earlier}</p>
      )}
      {section.beats.map((beat) => (
        <Beat key={beat.id} beat={beat} />
      ))}
    </>
  );
  const heading = section.heading;
  if (heading === undefined) return <div className="armada-narration__beats">{beats}</div>;
  const title = (
    <>
      {heading.mark === undefined ? null : <TaskMark state={heading.mark} />}
      {heading.task === undefined ? null : (
        <span className="armada-narration__task mono">{heading.task}</span>
      )}
      <span className="armada-narration__title">{heading.title}</span>
      {section.meta === undefined ? null : (
        <span className="armada-narration__meta">{section.meta}</span>
      )}
    </>
  );
  const files =
    section.files === undefined || section.files.length === 0 ? null : (
      <Files files={section.files} say={section.filesSay} />
    );
  // A task nobody has worked on this step is a line with nothing to open.
  if (section.beats.length === 0) {
    return (
      <div className="armada-narration__section" data-apart={section.apart || undefined}>
        <p className="armada-narration__head">
          <span className="armada-narration__fold" aria-hidden />
          {title}
        </p>
        {files}
      </div>
    );
  }
  return (
    <div className="armada-narration__section" data-apart={section.apart || undefined}>
      <button
        type="button"
        className="armada-narration__head"
        aria-expanded={open}
        onClick={() => setOpen(!open)}
      >
        <Chevron open={open} />
        {title}
      </button>
      {files}
      {/* Hidden rather than unmounted, so a row opened inside survives a fold. */}
      <div className="armada-narration__beats" hidden={!open}>
        {beats}
      </div>
    </div>
  );
}

function Beat({ beat }: { beat: NarrationBeat }) {
  const [chosen, setOpen] = useState<boolean | undefined>(undefined);
  const open = chosen ?? beat.open === true;
  return (
    <div className="armada-narration__beat">
      {beat.said === undefined ? null : (
        <p className="armada-narration__said">
          {beat.at === undefined ? null : (
            <span className="armada-narration__at mono">{beat.at}</span>
          )}
          <span className="armada-narration__words">{beat.said}</span>
        </p>
      )}
      {beat.body === undefined ? null : (
        <>
          <button
            type="button"
            className="armada-narration__calls"
            aria-expanded={open}
            onClick={() => setOpen(!open)}
          >
            <Chevron open={open} />
            <span className="armada-narration__meta">{beat.meta}</span>
          </button>
          <div className="armada-narration__body" hidden={!open}>
            {beat.body}
          </div>
        </>
      )}
    </div>
  );
}

/**
 * A section's files on one wrapping line, each with `+6 −1`, and what those
 * numbers are said after them. **A zero is not drawn**, for `ChangedFiles`'s
 * reason: `−0` reads as a value that failed to arrive.
 */
function Files({ files, say }: { files: NarrationFile[]; say?: string }) {
  return (
    <div className="armada-narration__files">
      <ul aria-label={say === undefined ? "Files" : `Files, ${say}`}>
        {files.map((file) => (
          <li key={file.path} title={file.path}>
            <span className="mono">{file.name}</span>
            {file.added === undefined || file.added === 0 ? null : (
              <span className="armada-narration__added mono">{`+${file.added}`}</span>
            )}
            {file.deleted === undefined || file.deleted === 0 ? null : (
              <span className="armada-narration__deleted mono">{`\u2212${file.deleted}`}</span>
            )}
          </li>
        ))}
      </ul>
      {say === undefined ? null : <span className="armada-narration__files-say">{say}</span>}
    </div>
  );
}

function Chevron({ open }: { open: boolean }) {
  return open ? (
    <ChevronDown className="armada-narration__fold" size={13} strokeWidth={2} aria-hidden />
  ) : (
    <ChevronRight className="armada-narration__fold" size={13} strokeWidth={2} aria-hidden />
  );
}

export type NarrationPlanBarProps = {
  /** One per task not dropped, in plan order. */
  tasks: readonly TaskBarSegment[];
  done: number;
};

/** `2 of 5` beside the Working header's act: the Plan well's bar and figure. */
export function NarrationPlanBar({ tasks, done }: NarrationPlanBarProps) {
  const figure = `${done} of ${tasks.length}`;
  return (
    <span className="armada-narration__plan">
      <StepBar tasks={tasks} label={`${figure} tasks`} />
      <span aria-hidden>{figure}</span>
    </span>
  );
}
