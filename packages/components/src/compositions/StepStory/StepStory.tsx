import type { ReactNode } from "react";
import { useState } from "react";
import { Button } from "../../primitives/Button/Button";

/**
 * The step's story — Drone instructions, then Activity log, then Produced, in
 * the order it happened.
 *
 * **Opening one chapter collapses the others to their header line.** Not to
 * save room: to keep the story's order intact while one part of it is long. A
 * collapsed chapter still carries its header, so what happened in the step
 * stays readable at a glance even while a reader is deep in one part of it.
 *
 * **One open at a time**, and that is the constraint that makes this different
 * from a stack of accordions — six open at once was unreadable, which is what
 * this replaces. The cost is real and stated rather than hidden: you cannot
 * read the transcript beside the diff, and that pairing is how a Drone
 * narrating one thing and doing another is caught. If that turns out to matter
 * the fix is a split for those two only, not letting everything open at once.
 *
 * **Every chapter is always on screen.** A chapter with nothing open shows its
 * preview — the first few log entries, the files with their counts — so the
 * story reads through without pressing anything. Opening is for the whole of a
 * thing, never for finding out that it exists.
 */
export type StepChapter = {
  id: string;
  /**
   * Its place in the story, drawn in the mark. The order is the order things
   * happened and it never changes, which is what a reader navigates by.
   */
  ordinal: number;
  title: ReactNode;
  /**
   * The header line's trailing half — `live · 47 entries · every line opens`,
   * `3 files · +94 −31`. What the chapter holds, so a collapsed one still
   * answers for itself.
   */
  summary?: ReactNode;
  /** What it shows while nothing in the story is open. */
  preview?: ReactNode;
  /**
   * What it shows while it is the open one. Absent on a chapter that has no
   * more to give than its preview — which makes it un-openable, and the
   * control does not draw.
   */
  content?: ReactNode;
  /** The control that opens it — `Open the log — all 47 entries`. */
  openLabel?: ReactNode;
  /** The control that closes it. `Close` unless a chapter wants its own word. */
  closeLabel?: ReactNode;
};

export type StepStoryProps = {
  chapters: StepChapter[];
  /** Which chapter is open on mount. After that the story holds its own. */
  openId?: string;
  /** Told when a chapter is opened or closed, for a caller that records it. */
  onOpen?: (chapterId: string | null) => void;
};

export function StepStory({ chapters, openId, onOpen }: StepStoryProps) {
  const [open, setOpen] = useState<string | null>(openId ?? null);

  function toggle(chapterId: string): void {
    const next = open === chapterId ? null : chapterId;
    setOpen(next);
    onOpen?.(next);
  }

  return (
    <ol className="armada-story">
      {chapters.map((chapter) => {
        const opens = chapter.content !== undefined;
        const shown = open === chapter.id;
        // A chapter that is not the open one collapses to its header the
        // moment anything is open. Nothing else changes: same chapters, same
        // order, so the reader never loses where they are.
        const collapsed = open !== null && !shown;
        return (
          <li className="armada-story__chapter" key={chapter.id} data-open={shown || undefined}>
            <div className="armada-story__head">
              <span className="armada-story__ordinal" aria-hidden>
                {chapter.ordinal}
              </span>
              <span className="armada-story__title">{chapter.title}</span>
              {chapter.summary === undefined ? null : (
                <span className="armada-story__summary">{chapter.summary}</span>
              )}
              {!opens ? null : (
                <Button
                  variant="ghost"
                  size="sm"
                  aria-expanded={shown}
                  onClick={() => toggle(chapter.id)}
                >
                  {shown ? (chapter.closeLabel ?? "Close") : (chapter.openLabel ?? "Open")}
                </Button>
              )}
            </div>

            {collapsed ? null : (
              <div className="armada-story__body">
                {shown ? chapter.content : chapter.preview}
              </div>
            )}
          </li>
        );
      })}
    </ol>
  );
}
