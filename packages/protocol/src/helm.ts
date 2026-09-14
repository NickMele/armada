// One repository's Helm conversation, as TypeScript sees it. `crates/ipc/src/helm.rs`.
//
// Hand-written like `turn.ts`, for `turn.ts`'s own reason: this is a second
// query whose transport is a socket, and the codegen that would emit both
// sides does not exist yet.
//
// **A `row` here is a Drone's turn's own `Saw`, from `turn.ts`.** A reply is
// read with the vocabulary a Drone's transcript already has — `docs/practices/
// protocol.md`'s "The Helm socket" is explicit about it — so this file adds no
// second row shape for prose that happens to come from a session instead of a
// Drone.

import type { Missed } from "./events";
import type { Saw, Voice } from "./turn";
import type { ProtocolVersion } from "./version";

/** `POST /helm/ask` — what a person says to Helm. Blank is refused before this is sent. */
export type AskHelm = {
  text: string;
  /** Where the person is in Bridge, sent with every ask. `#1075`. */
  context?: HelmContext;
};

/**
 * Where the person is in Bridge, as one ask carries it — a snapshot of the
 * moment it was sent, never a subscription. `crates/ipc/src/helm.rs`.
 */
export type HelmContext = {
  screen: HelmScreen;
  /** The repository the rail has picked. Absent for All repositories. */
  picked?: string;
  /** The Job chipped above Helm's message box — present only while the chip stands. */
  chip?: string;
  /** The row the cursor is on, in the Board or in Overview. Neither always has one. */
  cursor?: string;
};

/** Which screen is showing. `App.tsx` is the one place that decides between them. */
export type HelmScreen = "overview" | "board" | "manifest" | "cleanup" | "job_detail";

/**
 * What `POST /helm/ask` and `POST /helm/start_fresh` answer with. **Not the
 * reply**, which arrives on the socket.
 */
export type HelmConversation = {
  manifest_id: string;
  /** A reply is being written, or a message is waiting for one. */
  replying: boolean;
  /** The next message resumes a stored session. `false` starts a new one. */
  resumes: boolean;
};

/** One message on `GET /helm/observe`'s socket. */
export type HelmMessage =
  | ({ message: "opened" } & HelmOpened)
  | ({ message: "asked" } & HelmAsked)
  /** `Shown` on the Rust side. `turn.ts`'s own row, tagged the same way. */
  | ({ message: "row"; ts: string; step?: string; by?: Voice } & Saw)
  | ({ message: "fresh" } & HelmFresh)
  | ({ message: "unanswered" } & HelmUnanswered)
  | ({ message: "missed" } & Missed)
  | ({ message: "closed" } & HelmClosed);

/** The first message on every connection. */
export type HelmOpened = {
  protocol_version: ProtocolVersion;
  manifest_id: string;
  /** Whether a reply was being written when this opened. */
  replying: boolean;
  /** Older messages the thread left out, because the backfill is bounded. */
  skipped: number;
};

/** What a person said, as Fleet took it. */
export type HelmAsked = {
  ts: string;
  text: string;
};

/** The stored session could not be resumed, so this reply starts a new one. */
export type HelmFresh = {
  ts: string;
  because: Freshness;
};

/** Why a conversation started over without a person asking it to. Left as a plain union, `Voice`'s reason: no registry declares the set. */
export type Freshness = "session_not_found";

/** No reply came, and why. */
export type HelmUnanswered = {
  ts: string;
  why: string;
};

/** Nothing more on this connection, and why. */
export type HelmClosed = {
  because: HelmSilence;
};

export type HelmSilence = "started_fresh";
