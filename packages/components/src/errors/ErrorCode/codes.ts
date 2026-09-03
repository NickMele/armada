/**
 * The namespace Bridge mints its own error codes in.
 *
 * # Why Bridge mints at all
 *
 * **The code is one of the two channels separating an error from a status**,
 * which are the same red — the other being a solid fill where a status is a
 * 12% tint. So `always` is load-bearing, and only one of Bridge's five
 * failures crosses the wire that guarantees one. A renderer that threw and a
 * Fleet that never answered never reached it; `UnreadableJob` reaches it and
 * carries a sentence with no code.
 *
 * **The precedent is `run_id`, one field over**: each process mints its own so
 * that an error raised inside Bridge carries a real id rather than nothing. A
 * code minted by the process that raised the failure keeps `always` true
 * instead of carving the first exception into the rule that does the
 * separating.
 *
 * The rejected alternative was the region name in the chip's place, which was
 * built and is what this replaces. It keeps the fill and loses the meaning: a
 * region names what stopped drawing, not what went wrong.
 *
 * # Why a namespace, and not the collected manifest
 *
 * `cargo xtask verify-error-codes` is specified as walking the **Rust**
 * workspace, so a code declared in TypeScript has nowhere to be collected
 * from. Joining it means teaching the collection a second language — and that
 * check is not written yet in any form, so the change is not a directory added
 * to a walk but a decision about what a cross-language collector is. **That is
 * a bigger change than the component this arrived through.**
 *
 * The prefix is what keeps the two sets disjoint without a shared collector,
 * and nothing has to agree with anything for it to hold: no crate raises a
 * `bridge.` code, and Bridge never parses a code it received.
 *
 * **What it costs.** Nothing on this side fails on a duplicate. The bound is
 * that every Bridge code is declared in one file — `failures.ts` in
 * `@armada/shell`, beside the builder that raises it. If the Rust collection
 * ever grows a TypeScript walk, this prefix is the set to fold in.
 */

/**
 * A code Bridge minted for one of its own faults.
 *
 * **The prefix names the process that minted it**, which is the whole claim
 * the code makes: a reader meeting `bridge.render.boundary` in an issue body
 * knows without a lookup that no Fleet raised it and no manifest holds it.
 * Dotted lowercase segments below that, matching the shape the error contract
 * uses for a Rust code.
 *
 * The template literal is the whole enforcement and it is enough — a
 * declaration that forgets the prefix does not compile, so the namespace
 * cannot leak by omission. It is a type and not a constant for the same
 * reason: a `startsWith` check nothing calls proves less than a compile error.
 *
 * A code off the wire stays a plain `string`. It is opaque to Bridge — looked
 * up, never parsed — and narrowing it here would be Bridge claiming to know
 * the shape of something it is contractually not allowed to read.
 */
export type BridgeCode = `bridge.${string}`;
