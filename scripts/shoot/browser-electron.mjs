/* One implementation of the seam in `browser.mjs`, and the only file in the
 * tool that knows what a browser is. Runs as an Electron main process, opens
 * one hidden window, and does exactly what the request file asks: read the
 * marks out of a page, or turn boxes on a page into PNGs.
 *
 * Electron because it is already a workspace dependency — Bridge ships it.
 * Playwright or Puppeteer would put a second Chromium on every machine and in
 * CI to render pages the one already here renders. Read `browser.mjs` for why
 * that choice is confined to this file.
 *
 * It talks to the page through the DevTools protocol rather than
 * `webContents.capturePage`, because `Page.captureScreenshot` takes a clip in
 * page coordinates and `captureBeyondViewport` paints the part of a tall page
 * that is not on screen. `capturePage` can only return what the viewport is
 * showing, and the gallery is one very tall page.
 *
 * It is never run by hand. `browser.mjs` writes the request, spawns this, and
 * reads the reply. */
import { app, BrowserWindow } from "electron";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname } from "node:path";

/* Software rendering. A GPU process buys nothing for a window nobody looks at
 * and is one more thing that can fail on a machine with no display attached. */
app.disableHardwareAcceleration();

const requestPath = process.argv[process.argv.length - 1];
const request = JSON.parse(readFileSync(requestPath, "utf8"));

/* Two device pixels per CSS pixel. Text at 12px is most of what these shots
 * carry and at scale 1 it is legible to a person and marginal to an agent
 * reading the PNG back. Doubling it costs disk, which is `.shots/` and
 * ignored. */
const SCALE = 2;

/** What the page knows about itself: every mark, and where its box is. */
const READ_MARKS = `(() => {
  const box = (el) => {
    const r = el.getBoundingClientRect();
    return {
      x: r.left + window.scrollX,
      y: r.top + window.scrollY,
      width: r.width,
      height: r.height,
    };
  };
  /* The gallery marks a <figure> and the caption above the stage is the
     gallery's own furniture, not the component. Capture the stage where there
     is one, and the marked element itself otherwise — a drawing has no
     caption. */
  const marked = [...document.querySelectorAll("[data-shot]")].map((el) => ({
    mark: el.getAttribute("data-shot"),
    ...box(el.querySelector(":scope > .stage") ?? el),
  }));
  /* A frame is a drawing's unit of composition: an element with an id that no
     other id contains. Listing them is how an unmarked drawing is refused by
     name rather than by count. */
  const frames = [...document.querySelectorAll("[id]")]
    .filter((el) => !el.parentElement?.closest("[id]"))
    .map((el) => {
      const heading = el.querySelector("h1, h2, h3, h4, figcaption, legend");
      /* The rule the design settled on: inside a frame, the div whose own
         style paints the app's base background is the screen. */
      const painted = [...el.querySelectorAll("div[style]")].find((d) =>
        /background\\s*:\\s*var\\(--bg-base\\)/.test(d.getAttribute("style")),
      );
      /* No such div, so fall back to the frame's biggest child by area. A
         frame is a heading, sometimes a note, and the thing it names, and the
         thing it names is the largest of those every time in practice —
         "last child" picked the trailing annotation paragraph on three of
         seven frames in the drawing this was first run against.

         Offered as a fallback and labelled as one, because it is a guess and
         the painted div is not. */
      const fallback = [...el.children]
        .map((c) => [c, c.getBoundingClientRect()])
        .sort((a, b) => b[1].width * b[1].height - a[1].width * a[1].height)[0]?.[0];
      const candidate = painted ?? fallback;
      return {
        id: el.id,
        marked: !!el.querySelector("[data-shot]") || el.hasAttribute("data-shot"),
        heading: heading?.textContent?.trim().slice(0, 120) ?? "",
        candidate: candidate
          ? {
              guessed: !painted,
              /* Enough of the opening tag to find the line by eye. */
              opening: candidate.outerHTML.slice(0, candidate.outerHTML.indexOf(">") + 1),
              ...box(candidate),
            }
          : null,
      };
    });
  return {
    marks: marked,
    frames,
    page: {
      width: Math.max(document.documentElement.scrollWidth, window.innerWidth),
      height: Math.max(document.documentElement.scrollHeight, window.innerHeight),
    },
  };
})()`;

/* Fonts loaded, images decoded, two frames painted. Images matter for the pair
 * sheets, which are two <img> in a grid — capturing before they decode gives a
 * pair of empty boxes, and an empty box is exactly the defect this tool exists
 * to catch, so producing one silently would poison the result.
 *
 * A hidden window throttles requestAnimationFrame, so the race gives up after
 * half a second rather than hanging on a callback that is never scheduled. */
const SETTLE = `(async () => {
  try { await document.fonts.ready } catch {}
  await Promise.all(
    [...document.images].map((i) => i.decode().catch(() => {})),
  );
  await Promise.race([
    new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))),
    new Promise((r) => setTimeout(r, 500)),
  ]);
  return true;
})()`;

async function run() {
  app.dock?.hide?.();

  const win = new BrowserWindow({
    show: false,
    width: request.width ?? 1440,
    height: request.height ?? 1200,
    webPreferences: {
      /* The same posture as Bridge's window. Nothing here needs Node in the
         page, and a drawing is a file this tool did not author. */
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
      backgroundThrottling: false,
      /* A drawing and the gallery are both one file on disk that reaches
         nothing. Neither may fetch. */
      webSecurity: true,
    },
  });

  const failures = [];
  win.webContents.on("console-message", (e) => {
    if (e.level === "error") failures.push(e.message);
  });
  win.webContents.on("did-fail-load", (_e, code, desc, url) =>
    failures.push(`${desc} (${code}) loading ${url}`),
  );

  await win.loadFile(request.page);
  await win.webContents.executeJavaScript(SETTLE);

  const read = await win.webContents.executeJavaScript(READ_MARKS);

  const written = [];
  if (request.capture) {
    const dbg = win.webContents.debugger;
    dbg.attach("1.3");
    await dbg.sendCommand("Page.enable");

    for (const { mark, x, y, width, height } of read.marks) {
      if (width < 1 || height < 1) continue;
      const { data } = await dbg.sendCommand("Page.captureScreenshot", {
        format: "png",
        captureBeyondViewport: true,
        fromSurface: true,
        clip: { x, y, width, height, scale: SCALE },
      });
      const out = `${request.into}/${mark}.png`;
      mkdirSync(dirname(out), { recursive: true });
      writeFileSync(out, Buffer.from(data, "base64"));
      written.push({ mark, file: out, width, height });
    }
    dbg.detach();
  }

  writeFileSync(
    request.reply,
    JSON.stringify({ ...read, written, failures }, null, 2),
    "utf8",
  );
  win.destroy();
}

app.whenReady().then(() =>
  run().then(
    () => app.exit(0),
    (e) => {
      console.error(String(e?.stack ?? e));
      app.exit(1);
    },
  ),
);
