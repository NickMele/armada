// What each of Bridge's failures says.
//
// **A builder each, not one message.** Fleet unreachable, a renderer that
// threw, and a Job the store refused are three different situations demanding
// three different things, and folding them into one sentence is the defect
// these files exist to repair. They share a shape — `Failure notice` — the way
// six Job states share one row shape, and `notice.ts` is that shape.
//
// **Eleven codes, each with a class and a builder, and they divide by failure
// rather than by part.** Three lists of eleven would put a code in one file and
// the argument for its class in another. What each builder knows is the seam
// instead:
//
// | File | What it is handed | The failures |
// | --- | --- | --- |
// | `reading.ts` | state Fleet published, read every render | the connection, a row the store refused |
// | `commands.ts` | the answer to something somebody pressed | a refusal, a command Fleet did not answer |
// | `thrown.ts` | an exception raised in this process | a render that threw, a throw no boundary saw |
//
// **Bridge mints a code for each of its own faults, and each is declared beside
// the builder that raises it.** Only one of these failures crosses the wire, so
// only one arrives with a code it did not mint. The namespace and the argument
// for it are in `codes.ts` beside `ErrorCode` in `@armada/components`.
//
// **`cargo xtask verify-error-codes` collects these**, since `#345`: it walks
// both languages and fails on a duplicate within either, so a second
// declaration of one code names both sites rather than resting on somebody
// having read one file.

export type { Failure, FailureFacts } from "./notice";
export * from "./reading";
export * from "./commands";
export * from "./thrown";
