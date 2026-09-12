/**
 * The namespace Bridge mints its own error codes in.
 *
 * Why Bridge mints at all: the code is one of the two channels separating
 * an error from a status, the same red otherwise — the other being a
 * solid fill where a status is a 12% tint. Only one of Bridge's five
 * failures crosses the wire that guarantees one; a renderer that threw
 * and a Fleet that never answered never reached it, and `UnreadableJob`
 * reaches it with a sentence and no code.
 */

/**
 * The precedent is `run_id`, one field over: each process mints its own
 * so an error raised inside Bridge carries a real id rather than
 * nothing. A code minted by the process that raised the failure keeps
 * `always` true instead of carving an exception into the rule that does
 * the separating. The rejected alternative was the region name in the
 * chip's place, which kept the fill and lost the meaning: a region names
 * what stopped drawing, not what went wrong.
 */

/**
 * Why a namespace, and not the collected manifest: `cargo xtask
 * verify-error-codes` was specified as walking the Rust workspace alone,
 * which is why this namespace was taken rather than joined. `#345` since
 * taught the collection both languages, and the prefix survives that on
 * its own merits — it lets each half be checked against itself while
 * deciding nothing about the other.
 *
 * The prefix keeps the two sets disjoint without a shared collector: no
 * crate raises a `bridge.` code, and Bridge never parses a code it
 * received. A duplicate now fails on this side too — the collection
 * reads the `const X: BridgeCode = "…"` form wherever declared, so the
 * bound is no longer that every Bridge code lives in one file and
 * somebody read it.
 */

/**
 * A code Bridge minted for one of its own faults. The prefix names the
 * process that minted it, which is the whole claim the code makes: a
 * reader meeting `bridge.render.boundary` in an issue body knows without
 * a lookup that no Fleet raised it and no manifest holds it. Dotted
 * lowercase segments below that, matching the shape the error contract
 * uses for a Rust code.
 *
 * The template literal is the whole enforcement: a declaration that
 * forgets the prefix does not compile, so the namespace cannot leak by
 * omission. It is a type and not a constant for the same reason — a
 * `startsWith` check nothing calls proves less than a compile error.
 */

/**
 * A code off the wire stays a plain `string`. It is opaque to Bridge —
 * looked up, never parsed — and narrowing it here would be Bridge
 * claiming to know the shape of something it is contractually not
 * allowed to read.
 */
export type BridgeCode = `bridge.${string}`;
