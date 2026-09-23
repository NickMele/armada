// Where a workflow came from, in Fleet's own words for `WorkflowSource` — #425.
// It lived in `@armada/screens`'s `facts.ts` while the hand form was the only
// picker; the Settings block is the picker now, and a vocabulary has one owner.

/** Fleet's three sources, as a person reads them. */
export const WORKFLOW_SOURCE: Readonly<Record<string, string>> = {
  armada: "carried by Armada",
  kit: "from Kit",
  repository: "from the repository",
};

/** `, from Kit`, or nothing where Fleet did not say. An unknown word renders as itself. */
export function sourceOf(source: string | undefined): string {
  if (source === undefined || source === "") return "";
  return `, ${WORKFLOW_SOURCE[source] ?? source}`;
}
