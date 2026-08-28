import type { ReactNode } from "react";
import { useState } from "react";
import { Tabs } from "../../primitives/Tabs/Tabs";

/**
 * Job record — everything about a Job that is not what it was or what it
 * produced, folded into one place and one interaction away.
 *
 * **Tucked away is not gone.** A finished Job is read once, to decide whether
 * to take the work, so the two questions that decide it hold the top of the
 * screen and the rest of the record sits under a tab strip. Every fact stays
 * reachable and nobody has to leave for a terminal to get one: this is
 * progressive disclosure, not omission.
 *
 * **A tab strip and not an accordion.** `tabs` is a sanctioned primitive and a
 * disclosure list is not, and the two read differently anyway: sections opened
 * at once would restore the wall of equal-weight regions this component exists
 * to replace, and a rail that grows and shrinks as sections open is the layout
 * that broke on resize in v1.
 *
 * **Only the open section is rendered.** That is what makes the fold cost
 * nothing — a transcript, a rail and a file list mounted behind a closed tab
 * are three lists rendering for nobody. It is also what lets a section own a
 * subscription: a section that is not drawn has not opened anything.
 *
 * **The strip is the whole navigation and it does not reorder.** Sections keep
 * the order they were given whatever arrives, because a strip that reshuffles
 * as data lands is the column flip-flop the failure log named, one level up.
 */
export type JobRecordSection = {
  id: string;
  /** Sentence case, and it names the answer rather than the component. */
  label: string;
  /** Rendered only while this section is the open one. */
  panel: ReactNode;
};

export type JobRecordProps = {
  sections: JobRecordSection[];
  /** Controlled. Omit for the uncontrolled form. */
  value?: string;
  defaultValue?: string;
  onChange?: (id: string) => void;
  /** What the record says when it holds no section at all. */
  emptyNote?: ReactNode;
};

export function JobRecord({
  sections,
  value,
  defaultValue,
  onChange,
  emptyNote = "Nothing about this job is recorded yet.",
}: JobRecordProps) {
  const [internal, setInternal] = useState(defaultValue ?? sections[0]?.id);
  // The open section, held to what exists: a controlled value naming a section
  // that is not in the strip would draw a selected tab with no panel under it.
  const asked = value ?? internal;
  const open = sections.find((section) => section.id === asked) ?? sections[0];

  if (sections.length === 0) {
    return (
      <div className="armada-record">
        <p className="armada-record__note">{emptyNote}</p>
      </div>
    );
  }

  return (
    <div className="armada-record">
      <Tabs
        items={sections.map((section) => ({ id: section.id, label: section.label }))}
        value={open?.id}
        onChange={(id) => {
          if (value === undefined) setInternal(id);
          onChange?.(id);
        }}
      />
      <div className="armada-record__panel" role="tabpanel">
        {open?.panel}
      </div>
    </div>
  );
}
