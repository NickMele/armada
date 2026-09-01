// Words the domain is counted in.
//
// **Here and not with the list that first needed it.** How a Job is pluralised
// is the same question wherever a count is rendered — a status bar, an empty
// state, a filter — and the lexicon is what the design system already owns. A
// helper living in the surface that happened to need it first is a helper the
// next surface writes again.

/** A count of Jobs, in words. `1 job`, and `0 jobs`. */
export function plural(total: number): string {
  return `${total} ${total === 1 ? "job" : "jobs"}`;
}
