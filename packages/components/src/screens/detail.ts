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
  /** The controls at the header's trailing edge. `Kill`, or a redispatch. */
  actions?: ReactNode;
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
