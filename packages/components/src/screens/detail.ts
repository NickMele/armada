import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import type { JobDetailField } from "../compositions/JobDetailHeaderActions/JobDetailHeaderActions";
import type { JobBriefProps } from "../compositions/JobBrief/JobBrief";
import type { JobLogReferenceRow } from "../compositions/JobLogReference/JobLogReference";

/**
 * What the three job detail renders share.
 *
 * Declared once because all three pass the same block to
 * `JobDetailHeaderActions` — three copies of this type is how the three
 * screens start disagreeing about what a Job's header is.
 */
export type JobDetailHeading = {
  /** The status token stem, e.g. `running`. From the generated vocabulary. */
  status: string;
  statusIcon: LucideIcon;
  statusLabel: ReactNode;
  headline: ReactNode;
  jobId?: ReactNode;
  fields: JobDetailField[];
  /** The controls at the header's trailing edge. `Kill`, or a redispatch, and the way into a running Job's settings. */
  actions?: ReactNode;
  /**
   * A fact carrying an `href` was clicked, with the address it carries.
   *
   * **Here rather than beside `onCopied` on the screen**, and the difference is
   * scope: copying is offered by every mono value on job detail, in four
   * regions, so the screen owns it; a link is offered by one fact in this
   * block, and the address and what to do with it are one decision made in one
   * place. Riding on the heading means it reaches the header through the same
   * spread the fields do, and no render in between has to know it exists.
   */
  onFollowed?: (href: string) => void;
  /**
   * A fact carrying an `opensJob` was pressed, with the Job id it names.
   *
   * Rides on the heading for `onFollowed`'s reason, and is the inward half of
   * the same pair: one fact in this block points at another Job, and where
   * that press lands is one decision made in one place.
   */
  onOpenJob?: (jobId: string) => void;
  /**
   * The trail's first segment — the screen this Job was opened from,
   * `Overview`. Absent draws no trail: the badge and the title alone, for a
   * caller that has not said where "back" goes.
   */
  from?: ReactNode;
  /**
   * The trail's first segment, pressed. Absent draws `from` as plain text
   * rather than a control — a caller that names where the person came from
   * without yet wiring a way there is the one shape to look for when this
   * segment does nothing on a click.
   */
  onLeave?: () => void;
};

/**
 * Where the work is — and, above it, what the Job was told.
 *
 * The brief lives here rather than in a region of its own: "what was it told,
 * and what did done mean" is asked in the same breath as "where are its
 * files", and two regions would separate the question from the answer.
 */
export type JobDetailLog = {
  rows: JobLogReferenceRow[];
  /** The brief and its criteria. Absent where the Job carries neither. */
  brief?: JobBriefProps;
  /** The sentence beneath — what the log holds, or what is left in place. */
  note?: ReactNode;
  actions?: ReactNode;
};
