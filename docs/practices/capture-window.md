# The capture window

**What it is:** the security review Bridge's posture requires before any window
loads another repository's running web app, and the rules that window is built
to. **Kind:** practice.

Read before building or changing the window Studio capture opens on a server
Run. Bridge's own window is not this window: `bridge.md`, Security posture,
still governs it, unchanged.

---

## What this window is

A person starts a declared server from a Studio, presses open on its Run node,
and gets a second window showing that server's page with Studio capture over
it. Notes land on the Studio the Run belongs to, fixed at capture as every Note
is.

**The page is untrusted, and that is not a judgement about the person's code.**
A dev server renders whatever the checkout and its dependency tree produce, so a
compromised dependency is the attacker this review is written against. Bridge
authored none of those bytes, which is the whole distinction `bridge.md` draws
around `nodeIntegration: false`.

## What it may load

> **Rule.** The window loads one origin — the origin of a link the Run
> published — fixed when the window opens and never changed afterwards.
> Why: an address that can change is an address that can be wrong, and there is
> no second place this window is meant to reach.

> **Rule.** The renderer names a server id and one of that server's links. Main
> resolves the address off the live holder Fleet published, and loads no string
> a click handler composed.
> Why: `apps/desktop/src/main/servers.ts` already holds that rule for opening a
> link in the system browser, and loading one in a window is strictly more.

> **Rule.** The host is `127.0.0.1`, `[::1]` or `localhost`, and the scheme is
> `http:` or `https:`. Any other address opens no window at all.
> Why: a Manifest may declare a server behind a public host or a tunnel, and
> this window is for a process on this machine. A port comes from the checkout's
> own span, `crates/fleet/src/ports.rs`.

> **Rule.** One window per Run. Opening again raises the one that is open.

> **Rule.** The Studio is the Run's own, decided when the window opens. Capture
> here has nothing to aim.

> **Rule.** `webSecurity` stays on, and Bridge neither adds a header to the
> page's response nor removes one. A page's own CSP is not rewritten to make it
> render.
> Why: rewriting a header would be Bridge claiming to contain a document it
> cannot contain, and turning off same-origin policy inside a foreign page is
> the opposite of this review.

## How it is held there

| What moves | Held by | What happens |
|---|---|---|
| The first load | The address main resolved | Nothing else opens a window |
| Top-level navigation | `will-navigate` | Refused, address said |
| A server redirect | `will-redirect` | Refused, address said |
| A subframe | `will-frame-navigate` | Refused, counted on the bar |
| A popup or new window | `setWindowOpenHandler` | Denied, address said |
| A download | `will-download` | Refused |

> **Rule.** A comparison is on the whole origin — scheme, host and port —
> against the origin the window opened on. Nothing is matched by prefix, by
> suffix or by hostname alone.

> **Rule.** A refused address is drawn in full and never followed. One act
> offers it to the system browser, `http:` and `https:` only, on a person's
> press.
> Why: an app a person is building redirects to an auth provider, and the honest
> answer is the browser they already use rather than a second browser inside
> Armada.

> **Rule.** Subresources are the page's own business and are not pinned to the
> origin.
> Why: a window that blocked the app's own API and its fonts would draw a broken
> app, and a subresource cannot become a document with no rail. What the page
> could send anywhere is bounded by its holding nothing of Armada's.

> **Rule.** Capture is refused from the moment the Run stops serving, and the
> bar says the run ended.
> Why: a loopback port is not an identity. Anything on the machine may bind it
> once the server exits, and a Note captured from a replacement would be a
> record of something other than what it names.

> **Rule.** The window loads nothing further once the Run ends. What is on
> screen stays; reload and every navigation are refused.

> **Rule.** A Note captured here records the Run and the address beside
> everything a Note captured on Bridge records.

## The injected layer

> **Rule.** The layer runs in the page's own world, injected by main after the
> first load and after every accepted navigation.
> Why: a component name is read off the `__reactFiber$` expando the page sets,
> and a contextIsolated world cannot see one —
> `apps/desktop/src/renderer/src/annotate/fiber.ts`.

> **Rule.** It reads the page and writes nothing to it: no node, no attribute,
> no style. It adds document-level listeners while capture is armed and removes
> them when it is not.
> Why: an overlay drawn in the page is an overlay the page can read, restyle and
> forge — and a Note about how something looks, taken through it, would be a
> Note about Bridge's own box.

> **Rule.** It reads the fields a Note keeps and nothing wider: the shape in
> `packages/protocol/src/studio.ts`, over the declared property list in
> `apps/desktop/src/renderer/src/capture/note.ts`.

> **Rule.** It reads no cookie, no `localStorage`, no `sessionStorage` and no
> form value, and issues no request of its own.

> **Rule.** The markup a Note keeps carries no `value` attribute, and no
> contents of an `input`, a `textarea` or anything under a password type.
> Why: `outerHTML` over a form is a person's own typing, kept for the life of a
> Studio that nothing expires.

> **Rule.** What the person says is typed into Bridge's own surface, drawn above
> the page and outside it, and never reaches the page.
> Why: the page would otherwise read every note left on it. The surface a person
> types into is built from `packages/tokens` and shadcn, which a foreign
> document's stylesheet would fight.

> **Rule.** The Studio's id, Fleet's port and every filesystem path stay in
> main. Nothing naming Armada's own records crosses into the page.

> **Rule.** The frame is taken by main, of this window's own contents, cropped
> to bounds clamped to the viewport.
> Why: main takes a Bridge Note's frame for the same reason — the capability is
> a Note with a picture, never a screenshot the page can ask for.

## What may come back

> **Rule.** One shape crosses, `StudioCapture`, and main re-applies every bound
> rather than trusting the page's: the declared style properties, the markup
> length, the text length, the selector length and each field's type.
> Why: the page can answer with anything, fifty megabytes of markup included,
> so the bound belongs on the side that is not the page.

> **Rule.** One armed press yields one capture, on a promise main created. The
> layer cannot send unasked.

> **Rule.** A page that forges a capture can put a Note on that Studio, beside
> the person's own words, and can do nothing else. That is the whole authority
> this channel carries.

## The four protections

| Protection | This window | Bridge's window |
|---|---|---|
| `contextIsolation: true` | Unchanged | Unchanged |
| `nodeIntegration: false` | Unchanged | Unchanged |
| `sandbox: true` | Unchanged | Unchanged |
| `default-src 'self'` | Not applied — no Armada document | Unchanged |

> **Rule.** The CSP is not loosened, relaxed or widened. It is a `meta` in
> `apps/desktop/src/renderer/index.html`, which this window never loads, so the
> page is governed by whatever its own server sends and Bridge's promise about
> Bridge is untouched.

> **Rule.** What is loosened is none of the four: this window loads an address
> where Bridge's loads a file, and its navigation refusal is an origin
> comparison rather than a flat refusal.
> Why: that is the change this review exists to approve, and naming it as a CSP
> change would hide it behind a flag nobody moved.

> **Rule.** Bridge's own window keeps `loadFile`, denies every navigation and
> every window open, and its CSP still refuses `127.0.0.1`.

## What the page can reach back into

> **Rule.** The window is created with no preload. The page has no
> `ipcRenderer`, no exposed bridge, and no channel to main of its own.
> Why: every function on a preload is a capability that window can call, and the
> only surface that cannot be widened by accident is an absent one. See
> `apps/desktop/src/preload/index.ts` for the surface this window does not get.

| Reach | Answer | By |
|---|---|---|
| Armada's IPC | None | No preload on this window |
| Node, `require`, `fs` | None | `nodeIntegration: false`, `sandbox: true` |
| The Studio's records | None | No id, no path, no port in the page |
| A frame Fleet kept | None | `armada-frame:` is handled on the default session |
| Bridge's window | None | Every window open denied, so no opener |
| Bridge's cookies and cache | None | Its own session partition |
| The filesystem | None | Downloads refused, `file:` never loaded |
| The screen | None | Fullscreen, pointer lock and every permission denied |
| A dialog that looks like Armada's | None | `disableDialogs` |

> **Rule.** The window runs in a session partition of its own, persistent, one
> per repository.
> Why: an app under development needs a login, and a window that forgot it every
> time is a window used once. Bridge's own session holds none of it, and two
> repositories share nothing.

> **Rule.** Every permission request is denied without asking, and a certificate
> error is not overridden.

## What a person sees

> **Rule.** The window carries the OS frame and its title, where Bridge's own
> window is frameless and draws its own.
> Why: two windows that are not the same kind of thing do not have the same
> chrome.

> **Rule.** A bar Bridge owns sits above the page, loaded from disk with
> Bridge's preload and Bridge's CSP, naming the Run, the address in full, and
> the Studio a Note will land on.
> Why: a person has to tell at a glance that what is below the bar is not
> Armada, and a bar composited above the page is a bar the page cannot draw
> over.

> **Rule.** There is no address field, no Back, no Forward and no history.
> Reload reloads the pinned origin.

> **Rule.** A refusal is said on the bar, naming the address that was refused.

> **Rule.** The window draws no rail and reaches no Armada surface. A Studio is
> read on Bridge's window.

## Refused outright

| Refused | Rather than |
|---|---|
| Any origin but the Run's | An allowlist, a setting, or a trusted-host list |
| A non-loopback address a Manifest declares | Capture against staging or production |
| Typing or pasting an address | A general browser inside Armada |
| Capture once the Run is not serving | Trusting a port to still be the Run's |
| Rewriting the page's CSP, or `webSecurity: false` | Making an awkward app render |
| Any preload capability on this window | One small exception, then a second |
| A page's `window.open`, downloads and dialogs | Filtering them |
| Following a refused address in this window | A second browser nobody asked for |

> **Rule.** A capability added to this window later is another security review,
> on the same terms as the four protections.
> Why: every guarantee above rests on the page holding nothing and reaching
> nothing, which one addition ends.

## What this does not claim

**The origin is not an identity.** Loopback is not a trust boundary between
processes: whatever binds the port owns the origin, and Armada cannot prove the
page came from the Run. What bounds it is that capture ends with the Run and the
window stops loading — not a check that would read as proof.

**A page hostile to its own data is not contained.** The window holds nothing of
Armada's, and it holds everything the page already had. Nothing here protects
the person's app from its own dependency tree.

**A Note keeps what was on screen.** Markup, styles and a frame taken over a
logged-in app carry that app's data into a Studio nothing expires, on the
person's own machine.

**Fleet's loopback surface is reachable from any browser on this machine.** Its
routes take a body as bytes and decode it themselves — `crates/api/src/studios.rs`
and every route beside it — so no content type is required and a cross-origin
`POST` is a request a browser sends without asking first. A page that finds the
port has that reach in Safari today; this window neither widens it nor narrows
it, and closing it is Fleet's work rather than this window's.
