import { jsx, jsxs, Fragment } from "react/jsx-runtime";
import { useState, createContext, useCallback, useContext, useRef, useEffect, Children, cloneElement, useId, Fragment as Fragment$1, useMemo, createElement } from "react";
import { ChevronDown, UserCheck, Cpu, GitBranch, CircleDot, X, Check, Power, ChevronRight, Flag, RotateCw, Eye, CircleX, CircleCheck, ShieldCheck, ExternalLink, File, TriangleAlert, OctagonAlert, Folder, GitCommitHorizontal, GitPullRequest, FileCheck, Clock, CircleMinus, ShieldX, ShieldMinus, Lock, MessageSquare, ClipboardList, Activity, Bell, ScrollText, Stethoscope, FileCog, Copy, ShieldOff, Stamp, Terminal, Link, Ban, Archive, RefreshCw, FileQuestionMark, Split, Unplug, ArrowUpToLine, Send, CornerUpRight, Settings } from "lucide-react";
import { renderToStaticMarkup } from "react-dom/server";
function Button({
  variant = "secondary",
  size = "default",
  ground = "card",
  iconOnly = false,
  type = "button",
  children,
  ...rest
}) {
  return /* @__PURE__ */ jsx(
    "button",
    {
      ...rest,
      type,
      className: "armada-button",
      "data-variant": variant,
      "data-size": size,
      "data-ground": ground,
      "data-icon-only": iconOnly || void 0,
      children
    }
  );
}
function SplitButton({
  children,
  items,
  variant = "secondary",
  ground = "card",
  defaultOpen = false,
  disabled = false,
  onAction,
  menuLabel = "More actions"
}) {
  const [open2, setOpen] = useState(defaultOpen);
  return /* @__PURE__ */ jsxs("div", { className: "armada-split-button", children: [
    /* @__PURE__ */ jsxs("div", { className: "armada-split-button__control", "data-variant": variant, "data-ground": ground, children: [
      /* @__PURE__ */ jsx(
        "button",
        {
          type: "button",
          className: "armada-split-button__action",
          disabled,
          onClick: onAction,
          children
        }
      ),
      /* @__PURE__ */ jsx(
        "button",
        {
          type: "button",
          className: "armada-split-button__caret",
          "aria-haspopup": "menu",
          "aria-expanded": open2,
          "aria-label": menuLabel,
          disabled,
          onClick: () => setOpen((was) => !was),
          children: /* @__PURE__ */ jsx(ChevronDown, { size: 16, strokeWidth: 2, "aria-hidden": "true" })
        }
      )
    ] }),
    open2 && /* @__PURE__ */ jsx("div", { className: "armada-split-button__menu", role: "menu", "aria-label": menuLabel, children: items.map((item) => /* @__PURE__ */ jsxs(
      "button",
      {
        type: "button",
        role: "menuitem",
        className: "armada-split-button__item",
        "data-danger": item.danger || void 0,
        onClick: () => {
          setOpen(false);
          item.onSelect?.();
        },
        children: [
          /* @__PURE__ */ jsx("span", { children: item.label }),
          item.shortcut !== void 0 && /* @__PURE__ */ jsx("span", { className: "armada-split-button__shortcut", children: item.shortcut })
        ]
      },
      item.label
    )) })
  ] });
}
const BADGE_ICON = 12;
const BADGE_STROKE = 2;
function Badge({ status, icon: Icon, children, pulsing = false }) {
  const animates = pulsing && status === "running";
  return /* @__PURE__ */ jsxs(
    "span",
    {
      className: "armada-badge",
      "data-status": status,
      "data-pulsing": animates || void 0,
      style: {
        color: `var(--status-${status})`,
        background: `var(--status-${status}-bg)`
      },
      children: [
        Icon ? /* @__PURE__ */ jsx(Icon, { size: BADGE_ICON, strokeWidth: BADGE_STROKE, "aria-hidden": true }) : null,
        children
      ]
    }
  );
}
function Kbd({ className, ...rest }) {
  return /* @__PURE__ */ jsx("kbd", { className: className ? `armada-kbd ${className}` : "armada-kbd", ...rest });
}
function KbdChord({ className, ...rest }) {
  return /* @__PURE__ */ jsx("span", { className: className ? `armada-kbd-chord ${className}` : "armada-kbd-chord", ...rest });
}
const RovingOption = createContext(null);
const FIELD_ICON = 12;
const FIELD_STROKE = 2;
function Copyable({
  value,
  copyValue,
  onCopied,
  className
}) {
  const handleClick = useCallback(
    (event) => {
      if (copyValue === void 0) return;
      event.stopPropagation();
      void navigator.clipboard.writeText(copyValue).then(
        () => onCopied?.(copyValue),
        // A failed clipboard write is otherwise indistinguishable from a dead
        // element, so the surface is told either way and says which happened.
        () => onCopied?.(copyValue)
      );
    },
    [copyValue, onCopied]
  );
  return /* @__PURE__ */ jsx("span", { className, "data-copies": copyValue !== void 0 || void 0, onClick: handleClick, children: value });
}
function JobRowStacked({
  status,
  statusIcon,
  statusLabel,
  headline,
  jobId,
  fields,
  tracks,
  action,
  actionKey,
  pulsing = false,
  focused,
  selected,
  dimmed,
  onOpen,
  onCopied
}) {
  const handleKeyDown = useCallback(
    (event) => {
      if (onOpen === void 0) return;
      if (event.key !== "Enter" && event.key !== " ") return;
      event.preventDefault();
      onOpen();
    },
    [onOpen]
  );
  const opens = onOpen !== void 0;
  const roving = useContext(RovingOption);
  const onCursor = roving === null || roving.index === roving.active;
  const tabIndex = !opens ? void 0 : onCursor ? 0 : -1;
  const pulses = pulsing && onCursor;
  return /* @__PURE__ */ jsxs(
    "div",
    {
      className: "armada-job-row",
      role: opens ? "option" : "listitem",
      "aria-selected": opens ? selected ?? false : void 0,
      tabIndex,
      "data-job-id": jobId,
      "data-focused": focused || void 0,
      "data-selected": selected || void 0,
      "data-dimmed": dimmed || void 0,
      onClick: onOpen,
      onKeyDown: opens ? handleKeyDown : void 0,
      children: [
        /* @__PURE__ */ jsx("div", { className: "armada-job-row__badge", children: /* @__PURE__ */ jsx(Badge, { status, icon: statusIcon, pulsing: pulses, children: statusLabel }) }),
        /* @__PURE__ */ jsxs("div", { className: "armada-job-row__body", children: [
          /* @__PURE__ */ jsxs("div", { className: "armada-job-row__headline", children: [
            /* @__PURE__ */ jsx("span", { className: "armada-job-row__title", children: headline }),
            jobId ? /* @__PURE__ */ jsx(
              Copyable,
              {
                className: "armada-job-row__id",
                value: jobId,
                copyValue: jobId,
                onCopied
              }
            ) : null
          ] }),
          /* @__PURE__ */ jsx(
            "div",
            {
              className: "armada-job-row__fields",
              style: { "--armada-row-fallback-tracks": tracks ?? trackList(fields.length) },
              children: fields.map((field, i) => /* @__PURE__ */ jsxs(
                "span",
                {
                  className: "armada-job-row__field",
                  "data-mono": field.mono || void 0,
                  "data-emphasis": field.emphasis || void 0,
                  "data-quiet": field.quiet || void 0,
                  children: [
                    field.icon ? /* @__PURE__ */ jsx(field.icon, { size: FIELD_ICON, strokeWidth: FIELD_STROKE, "aria-hidden": true }) : null,
                    field.label ? /* @__PURE__ */ jsx("span", { className: "armada-job-row__field-label", children: field.label }) : null,
                    /* @__PURE__ */ jsx(
                      Copyable,
                      {
                        className: "armada-job-row__field-value",
                        value: field.value,
                        copyValue: field.copyValue,
                        onCopied
                      }
                    )
                  ]
                },
                i
              ))
            }
          )
        ] }),
        action ? (
          // The control stops the row's own open, so clicking Approve does not
          // also navigate.
          /* @__PURE__ */ jsxs("div", { className: "armada-job-row__action", onClick: (e) => e.stopPropagation(), children: [
            action,
            actionKey === void 0 ? null : /* @__PURE__ */ jsx(Kbd, { className: "armada-job-row__key", "aria-hidden": true, children: actionKey })
          ] })
        ) : null
      ]
    }
  );
}
const JOB_ROW_LIST = "armada-job-row-list";
const DRAWN_TRACKS = [
  "var(--armada-track-origin)",
  "var(--armada-track-bar)",
  "var(--armada-track-step)",
  "var(--armada-track-time)",
  "var(--armada-track-spend)",
  "var(--armada-track-provenance)"
];
function trackList(count2) {
  return Array.from({ length: count2 }, (_, i) => DRAWN_TRACKS[i] ?? "minmax(0, auto)").join(" ");
}
function Tooltip({ label: label2, shortcut, children, defaultOpen = false }) {
  const [open2, setOpen] = useState(defaultOpen);
  const timer = useRef(void 0);
  useEffect(() => () => window.clearTimeout(timer.current), []);
  function readDelay() {
    const raw = getComputedStyle(document.documentElement).getPropertyValue("--tooltip-delay");
    return Number.parseInt(raw, 10) || 0;
  }
  function show() {
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => setOpen(true), readDelay());
  }
  function hide() {
    window.clearTimeout(timer.current);
    setOpen(false);
  }
  return /* @__PURE__ */ jsxs(
    "span",
    {
      className: "armada-tooltip",
      onMouseEnter: show,
      onMouseLeave: hide,
      onFocus: show,
      onBlur: hide,
      children: [
        children,
        open2 ? /* @__PURE__ */ jsxs("span", { className: "armada-tooltip__bubble", role: "tooltip", children: [
          /* @__PURE__ */ jsx("span", { className: "armada-tooltip__label", children: label2 }),
          shortcut ? /* @__PURE__ */ jsx("kbd", { className: "armada-tooltip__kbd", children: shortcut }) : null
        ] }) : null
      ]
    }
  );
}
function StepBar({ total, current, activity = "not_started", label: label2 }) {
  const segments = Array.from({ length: total }, (_, i) => {
    const position = i + 1;
    if (position < current) return "past";
    if (position === current) return "current";
    return "remaining";
  });
  const bar = /* @__PURE__ */ jsx(
    "span",
    {
      className: "armada-step-bar",
      role: "img",
      "aria-label": label2 ?? `Step ${current} of ${total}`,
      children: segments.map((state, i) => /* @__PURE__ */ jsx(
        "span",
        {
          className: "armada-step-bar__segment",
          "data-state": state,
          "data-activity": state === "current" ? activity : void 0
        },
        i
      ))
    }
  );
  return label2 ? /* @__PURE__ */ jsx(Tooltip, { label: label2, children: bar }) : bar;
}
const ROVES = /* @__PURE__ */ new Set(["ArrowDown", "ArrowUp", "Home", "End"]);
function ActiveJobsList({
  heading: heading2,
  summary,
  action,
  controls,
  children,
  empty,
  selectable = false,
  label: label2
}) {
  const rows = Array.isArray(children) ? children.filter(Boolean) : children;
  const isEmpty = rows === void 0 || rows === null || Array.isArray(rows) && rows.length === 0;
  const frame = useRef(null);
  const [active, setActive] = useState(0);
  const options = useCallback(
    () => Array.from(frame.current?.querySelectorAll(':scope > [role="option"]') ?? []),
    []
  );
  const rove = useCallback(
    (event) => {
      if (!ROVES.has(event.key)) return;
      const found = options();
      if (found.length === 0) return;
      const from = Math.min(active, found.length - 1);
      const to = event.key === "Home" ? 0 : event.key === "End" ? found.length - 1 : Math.min(Math.max(from + (event.key === "ArrowDown" ? 1 : -1), 0), found.length - 1);
      event.preventDefault();
      setActive(to);
      found[to]?.focus();
    },
    [active, options]
  );
  const followed = useCallback(
    (event) => {
      const option = event.target.closest('[role="option"]');
      if (option === null) return;
      const at = options().indexOf(option);
      if (at >= 0) setActive(at);
    },
    [options]
  );
  const roving = selectable && !isEmpty;
  return /* @__PURE__ */ jsxs("section", { className: "armada-active-jobs", children: [
    heading2 || summary || action ? /* @__PURE__ */ jsxs("header", { className: "armada-active-jobs__header", children: [
      /* @__PURE__ */ jsxs("div", { className: "armada-active-jobs__titles", children: [
        heading2 ? /* @__PURE__ */ jsx("h2", { className: "armada-active-jobs__heading", children: heading2 }) : null,
        summary ? /* @__PURE__ */ jsx("p", { className: "armada-active-jobs__summary", children: summary }) : null
      ] }),
      action ? /* @__PURE__ */ jsx("div", { className: "armada-active-jobs__action", children: action }) : null
    ] }) : null,
    controls ? /* @__PURE__ */ jsx("div", { className: "armada-active-jobs__controls", children: controls }) : null,
    /* @__PURE__ */ jsx(
      "div",
      {
        ref: frame,
        className: `armada-active-jobs__frame ${JOB_ROW_LIST}`,
        role: roving ? "listbox" : "list",
        "aria-label": label2,
        onKeyDown: roving ? rove : void 0,
        onFocus: roving ? followed : void 0,
        children: isEmpty ? empty : roving ? (
          // A provider renders no element, so the options stay direct
          // children of the listbox and `:scope >` still finds them.
          Children.map(rows, (row, index) => /* @__PURE__ */ jsx(RovingOption.Provider, { value: { index, active }, children: row }))
        ) : rows
      }
    )
  ] });
}
const meta$17 = {
  title: "Compositions/Active jobs list",
  component: ActiveJobsList
};
const WORKFLOW = /* @__PURE__ */ jsxs(Fragment, { children: [
  /* @__PURE__ */ jsx("span", { style: { fontFamily: "var(--font-mono)" }, children: "bug" }),
  ", 4 steps"
] });
const menu$1 = [
  { label: "Copy job id", shortcut: "⌘C" },
  { label: "Kill", shortcut: "x", danger: true }
];
const SixStates = {
  args: {
    heading: "Active jobs",
    summary: "6 jobs. 1 awaiting approval.",
    action: /* @__PURE__ */ jsx(Button, { variant: "primary", children: "New job" }),
    children: [
      // "Needs approval" is what `enum-verbs.toml` holds for
      // `job_status.awaiting_approval`, and its note says the wording is
      // deliberate: the badge means a person must act, not that time is
      // passing. The M1 drawing writes "Awaiting approval". A status label is
      // never written by hand, so the registry wins here where the drawing
      // wins on arrangement. Reported.
      /* @__PURE__ */ jsx(
        JobRowStacked,
        {
          status: "awaiting-approval",
          statusIcon: UserCheck,
          statusLabel: "Needs approval",
          headline: "Coalesce concurrent token refreshes",
          jobId: "job_7c31",
          fields: [
            { value: WORKFLOW },
            { value: /* @__PURE__ */ jsx(StepBar, { total: 4, current: 0, label: "Not started, 4 steps" }) },
            { value: "Not started", quiet: true },
            { value: "created 09:12", quiet: true },
            { value: "Dispatched by you" }
          ],
          action: /* @__PURE__ */ jsx(SplitButton, { ground: "card", items: [{ label: "Reject", danger: true }], children: "Approve" })
        },
        "a"
      ),
      /* @__PURE__ */ jsx(
        JobRowStacked,
        {
          status: "not-started",
          statusIcon: Cpu,
          statusLabel: "Waiting on resources",
          headline: "Retire the legacy poke path",
          jobId: "job_8b42",
          fields: [
            { value: WORKFLOW },
            { value: /* @__PURE__ */ jsx(StepBar, { total: 4, current: 0, label: "Not started, 4 steps" }) },
            // The step, like every other row's third field. It read "Waiting on a
            // drone", which said the reason a second time and said it wrong: the
            // Job is behind the cap rather than behind the one drone there used
            // to be. The badge carries the reason; a field does not repeat it.
            { value: "Not started", quiet: true },
            { value: "approved 09:20", quiet: true },
            { value: "Dispatched by you" }
          ],
          action: /* @__PURE__ */ jsx(SplitButton, { ground: "card", items: menu$1, children: "Open" })
        },
        "b"
      ),
      /* @__PURE__ */ jsx(
        JobRowStacked,
        {
          status: "running",
          statusIcon: CircleDot,
          statusLabel: "Running",
          headline: "Split the settings reducer",
          jobId: "job_2d90bb",
          pulsing: true,
          fields: [
            { value: "fix/settings-split", mono: true, icon: GitBranch, copyValue: "fix/settings-split" },
            { value: /* @__PURE__ */ jsx(StepBar, { total: 4, current: 2, activity: "running", label: "Step 2 of 4" }) },
            { value: "Implement", emphasis: true },
            { value: "11m 03s", mono: true },
            { value: "~$1.80", mono: true }
          ],
          action: /* @__PURE__ */ jsx(SplitButton, { ground: "card", items: menu$1, children: "Open" })
        },
        "c"
      ),
      /* @__PURE__ */ jsx(
        JobRowStacked,
        {
          status: "completed-failed",
          statusIcon: X,
          statusLabel: "Failed",
          headline: "Cache the manifest read",
          jobId: "job_91ab",
          fields: [
            { value: "feat/manifest-cache", mono: true, icon: GitBranch, copyValue: "feat/manifest-cache" },
            { value: /* @__PURE__ */ jsx(StepBar, { total: 4, current: 3, activity: "failed", label: "Step 3 of 4" }) },
            { value: "Run tests", emphasis: true },
            { value: "22m 41s", mono: true },
            { value: "~$2.10", mono: true }
          ],
          action: /* @__PURE__ */ jsx(SplitButton, { ground: "card", items: menu$1, children: "Open" })
        },
        "d"
      ),
      /* @__PURE__ */ jsx(
        JobRowStacked,
        {
          status: "completed-success",
          statusIcon: Check,
          statusLabel: "Done",
          headline: "Add a retry ceiling to the poke loop",
          jobId: "job_4f10",
          fields: [
            { value: "fix/poke-ceiling", mono: true, icon: GitBranch, copyValue: "fix/poke-ceiling" },
            { value: /* @__PURE__ */ jsx(StepBar, { total: 4, current: 5, activity: "advanced", label: "All 4 of 4 steps advanced" }) },
            { value: "Summarise" },
            { value: "18m 22s", mono: true },
            { value: "~$2.40", mono: true }
          ],
          action: /* @__PURE__ */ jsx(SplitButton, { ground: "card", items: menu$1, children: "Open" })
        },
        "e"
      ),
      /* @__PURE__ */ jsx(
        JobRowStacked,
        {
          status: "killed",
          statusIcon: Power,
          statusLabel: "Killed",
          headline: "Rename the session token field",
          jobId: "job_5e88",
          fields: [
            { value: "feat/session-rename", mono: true, icon: GitBranch, copyValue: "feat/session-rename" },
            { value: /* @__PURE__ */ jsx(StepBar, { total: 4, current: 2, activity: "killed", label: "Step 2 of 4" }) },
            { value: "Implement", emphasis: true },
            { value: "4m 09s", mono: true },
            { value: "~$0.60", mono: true }
          ],
          action: /* @__PURE__ */ jsx(SplitButton, { ground: "card", items: menu$1, children: "Open" })
        },
        "f"
      )
    ]
  }
};
const AtTheWidthFloor$1 = {
  render: () => /* @__PURE__ */ jsx("div", { style: { width: "calc(var(--window-floor) - var(--sidebar-rail))" }, children: /* @__PURE__ */ jsx(ActiveJobsList, { ...SixStates.args }) })
};
const EmptyWithNoEmptyState = {
  args: {
    heading: "Active jobs",
    summary: "No active jobs. 3 waiting on the Job Board.",
    action: /* @__PURE__ */ jsx(Button, { variant: "primary", children: "New job" }),
    children: []
  }
};
const AtAWideWindow = {
  render: () => /* @__PURE__ */ jsx("div", { style: { width: "calc(var(--window-floor) * 2)" }, children: /* @__PURE__ */ jsx(ActiveJobsList, { ...SixStates.args }) })
};
const Selectable = {
  args: {
    ...SixStates.args,
    selectable: true,
    label: "Active jobs",
    children: (SixStates.args?.children).map(
      (row, i) => cloneElement(row, { onOpen: () => {
      }, selected: i === 2 })
    )
  }
};
const TwoRunning = {
  args: {
    heading: "Active jobs",
    summary: "3 jobs.",
    selectable: true,
    label: "Active jobs",
    children: [
      /* @__PURE__ */ jsx(
        JobRowStacked,
        {
          status: "running",
          statusIcon: CircleDot,
          statusLabel: "Running",
          headline: "Split the settings reducer",
          jobId: "job_2d90bb",
          pulsing: true,
          onOpen: () => {
          },
          fields: [
            { value: "fix/settings-split", mono: true, icon: GitBranch, copyValue: "fix/settings-split" },
            { value: /* @__PURE__ */ jsx(StepBar, { total: 4, current: 2, activity: "running", label: "Step 2 of 4" }) },
            { value: "Implement", emphasis: true },
            { value: "11m 03s", mono: true },
            { value: "~$1.80", mono: true }
          ],
          action: /* @__PURE__ */ jsx(SplitButton, { ground: "card", items: menu$1, children: "Open" })
        },
        "a"
      ),
      /* @__PURE__ */ jsx(
        JobRowStacked,
        {
          status: "running",
          statusIcon: CircleDot,
          statusLabel: "Running",
          headline: "Coalesce concurrent token refreshes",
          jobId: "job_7c31",
          pulsing: true,
          onOpen: () => {
          },
          fields: [
            { value: "bug/token-refresh", mono: true, icon: GitBranch, copyValue: "bug/token-refresh" },
            { value: /* @__PURE__ */ jsx(StepBar, { total: 4, current: 1, activity: "running", label: "Step 1 of 4" }) },
            { value: "Plan", emphasis: true },
            { value: "2m 47s", mono: true },
            { value: "~$0.30", mono: true }
          ],
          action: /* @__PURE__ */ jsx(SplitButton, { ground: "card", items: menu$1, children: "Open" })
        },
        "b"
      ),
      /* @__PURE__ */ jsx(
        JobRowStacked,
        {
          status: "not-started",
          statusIcon: Cpu,
          statusLabel: "Waiting on resources",
          headline: "Retire the legacy poke path",
          jobId: "job_8b42",
          onOpen: () => {
          },
          fields: [
            { value: WORKFLOW },
            { value: /* @__PURE__ */ jsx(StepBar, { total: 4, current: 0, label: "Not started, 4 steps" }) },
            { value: "Not started", quiet: true },
            { value: "approved 09:20", quiet: true },
            { value: "Dispatched by you" }
          ],
          action: /* @__PURE__ */ jsx(SplitButton, { ground: "card", items: menu$1, children: "Open" })
        },
        "c"
      )
    ]
  }
};
const OneOption = {
  args: {
    heading: "Active jobs",
    summary: "1 job. 1 awaiting approval.",
    selectable: true,
    label: "Active jobs",
    children: [
      cloneElement((SixStates.args?.children)[0], {
        onOpen: () => {
        }
      })
    ]
  }
};
const __vite_glob_0_0 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AtAWideWindow,
  AtTheWidthFloor: AtTheWidthFloor$1,
  EmptyWithNoEmptyState,
  OneOption,
  Selectable,
  SixStates,
  TwoRunning,
  default: meta$17
}, Symbol.toStringTag, { value: "Module" }));
const NAMED$1 = {
  drone: "Drone",
  armada: "Armada",
  fleet: "Fleet"
};
const CHEVRON = 16;
const STROKE$6 = 2;
const MAX_LINES = 2e3;
function ActivityLog({
  entries: entries2,
  maxLines = MAX_LINES,
  cut,
  emptyNote = "Nothing has been recorded against this step yet.",
  openId
}) {
  const [open2, setOpen] = useState(
    () => openId === void 0 ? /* @__PURE__ */ new Set() : /* @__PURE__ */ new Set([openId])
  );
  const toggle = useCallback((entryId) => {
    setOpen((held) => {
      const next = new Set(held);
      if (next.has(entryId)) next.delete(entryId);
      else next.add(entryId);
      return next;
    });
  }, []);
  if (entries2.length === 0) {
    return /* @__PURE__ */ jsx("p", { className: "armada-activity__empty", role: "note", children: emptyNote });
  }
  return /* @__PURE__ */ jsxs("div", { className: "armada-activity", children: [
    /* @__PURE__ */ jsx("ol", { className: "armada-activity__entries", children: entries2.map((entry) => {
      const shown = open2.has(entry.id);
      const opens = entry.output !== void 0 || entry.payload !== void 0 || entry.ran !== void 0;
      const bounded = boundedTo(entry.output, maxLines);
      return /* @__PURE__ */ jsxs("li", { className: "armada-activity__entry", children: [
        /* @__PURE__ */ jsxs(
          "button",
          {
            type: "button",
            className: "armada-activity__row",
            "data-open": shown || void 0,
            "data-opens": opens || void 0,
            "aria-expanded": shown,
            onClick: () => toggle(entry.id),
            children: [
              /* @__PURE__ */ jsx("span", { className: "armada-activity__chevron", children: !opens ? null : shown ? /* @__PURE__ */ jsx(ChevronDown, { size: CHEVRON, strokeWidth: STROKE$6, "aria-hidden": true }) : /* @__PURE__ */ jsx(ChevronRight, { size: CHEVRON, strokeWidth: STROKE$6, "aria-hidden": true }) }),
              /* @__PURE__ */ jsx("span", { className: "armada-activity__at", children: entry.at }),
              /* @__PURE__ */ jsx("span", { className: "armada-activity__who", "data-actor": entry.actor, children: NAMED$1[entry.actor] }),
              /* @__PURE__ */ jsx("span", { className: "armada-activity__summary", children: entry.summary }),
              entry.subject === void 0 ? null : /* @__PURE__ */ jsx("span", { className: "armada-activity__subject", "data-named": entry.named, children: entry.subject })
            ]
          }
        ),
        !shown || !opens ? null : /* @__PURE__ */ jsxs("div", { className: "armada-activity__payload", children: [
          bounded === null ? null : /* @__PURE__ */ jsx("pre", { className: "armada-activity__output", children: bounded.text }),
          bounded === null || !bounded.cut ? null : /* @__PURE__ */ jsxs("p", { className: "armada-activity__cut", role: "note", children: [
            `Cut at ${maxLines} of ${bounded.lines} lines. `,
            entry.outputAt === void 0 ? "Nothing names where the whole of it was written, so the rest is not reachable from here." : `The whole of it is in ${entry.outputAt}.`
          ] }),
          entry.ran === void 0 ? null : /* @__PURE__ */ jsx("p", { className: "armada-activity__ran", children: entry.ran }),
          entry.payload === void 0 ? null : /* @__PURE__ */ jsx("div", { className: "armada-activity__extra", children: entry.payload })
        ] })
      ] }, entry.id);
    }) }),
    cut === void 0 ? null : /* @__PURE__ */ jsx("p", { className: "armada-activity__cut", role: "note", children: cut })
  ] });
}
function boundedTo(output, maxLines) {
  if (output === void 0) return null;
  const lines = output.split("\n");
  return lines.length <= maxLines ? { text: output, lines: lines.length, cut: false } : { text: lines.slice(0, maxLines).join("\n"), lines: lines.length, cut: true };
}
const meta$16 = {
  title: "Compositions/Activity log",
  component: ActivityLog
};
const BUILD_OUTPUT = [
  "$ cargo build --workspace --locked",
  "   Compiling armada-settings v0.1.0 (packages/settings)",
  "   Compiling armada-fleet v0.1.0 (crates/fleet)",
  "    Finished `dev` profile [unoptimized] in 47.61s"
].join("\n");
const STREAM = [
  {
    id: "1",
    at: "14:22:07",
    actor: "armada",
    summary: "Go on to Implement.",
    payload: "The injected turn that opens the step. Armada writes it; the Drone answers it."
  },
  {
    id: "2",
    at: "14:22:44",
    actor: "drone",
    summary: "Splitting the selector block into its own module so the tests can import it without the store."
  },
  { id: "3", at: "14:23:11", actor: "drone", summary: "Read", subject: "packages/settings/src/reducer.ts" },
  { id: "4", at: "14:26:31", actor: "drone", summary: "Edit", subject: "packages/settings/src/selectors.ts" },
  {
    id: "5",
    at: "14:29:40",
    actor: "drone",
    summary: "Bash",
    subject: "cargo build --workspace --locked",
    output: BUILD_OUTPUT,
    ran: "exit 0 · 47.61s · in .armada/worktrees/job_2d90bb"
  },
  {
    id: "6",
    at: "14:30:28",
    actor: "fleet",
    summary: "Heartbeat — the Drone has been quiet for 48 seconds"
  },
  { id: "7", at: "14:31:58", actor: "drone", summary: "thinking" }
];
const OneStream = {
  args: { entries: STREAM }
};
const AnEntryOpened = {
  args: { entries: STREAM, openId: "5" }
};
const AFleetEvent = {
  args: {
    openId: "f1",
    entries: [
      {
        id: "f0",
        at: "14:46:02",
        actor: "drone",
        summary: "Bash",
        subject: "cargo nextest run --workspace"
      },
      {
        id: "f1",
        at: "14:47:09",
        actor: "fleet",
        summary: "Check failed — 3 of 2034 tests. Handed back to the Drone, attempt 2 of 3.",
        subject: "test",
        named: "failed",
        output: [
          "FAIL settings::selectors::visible_manifests_memoises",
          "  expected the same reference on repeat calls, got a new object",
          "FAIL settings::selectors::hidden_manifests_excluded",
          "FAIL settings::reducer::identity_stable_across_actions"
        ].join("\n"),
        ran: "exit 101 · 1m 22s · in .armada/worktrees/job_2d90bb"
      }
    ]
  }
};
const APayloadCut = {
  args: {
    maxLines: 10,
    openId: "long",
    entries: [
      {
        id: "long",
        at: "14:47:09",
        actor: "fleet",
        summary: "Check failed — 218 of 2034 tests.",
        subject: "test",
        named: "failed",
        output: Array.from({ length: 218 }, (_, i) => `FAIL settings::selectors::case_${i + 1}`).join("\n"),
        outputAt: ".armada/logs/job_2d90bb/checks/test.log",
        ran: "exit 101 · 4m 02s · in .armada/worktrees/job_2d90bb"
      }
    ]
  }
};
const ACutThatNamesNoFile = {
  args: {
    maxLines: 10,
    openId: "long",
    entries: [
      {
        id: "long",
        at: "14:47:09",
        actor: "drone",
        summary: "Bash",
        subject: "cargo nextest run --workspace",
        output: Array.from({ length: 60 }, (_, i) => `line ${i + 1}`).join("\n")
      }
    ]
  }
};
const TheStreamCut = {
  args: {
    entries: STREAM,
    cut: "The newest 7 of 126 entries. The whole log is in .armada/logs/job_2d90bb.jsonl."
  }
};
const NothingYet = {
  args: { entries: [] }
};
const __vite_glob_0_1 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  ACutThatNamesNoFile,
  AFleetEvent,
  APayloadCut,
  AnEntryOpened,
  NothingYet,
  OneStream,
  TheStreamCut,
  default: meta$16
}, Symbol.toStringTag, { value: "Module" }));
function Input({ label: label2, invalid = false, message, mono = false, id, ...rest }) {
  const generated = useId();
  const inputId = id ?? generated;
  const messageId = `${inputId}-message`;
  const showMessage = invalid && message !== void 0;
  return /* @__PURE__ */ jsxs("div", { className: "armada-input-field", children: [
    label2 !== void 0 && /* @__PURE__ */ jsx("label", { className: "armada-input-field__label", htmlFor: inputId, children: label2 }),
    /* @__PURE__ */ jsx(
      "input",
      {
        ...rest,
        id: inputId,
        className: "armada-input",
        "data-mono": mono || void 0,
        "aria-invalid": invalid || void 0,
        "aria-describedby": showMessage ? messageId : void 0
      }
    ),
    showMessage && /* @__PURE__ */ jsx("span", { className: "armada-input-field__message", id: messageId, children: message })
  ] });
}
function Select({ label: label2, invalid = false, message, id, children, ...rest }) {
  const generated = useId();
  const selectId = id ?? generated;
  const messageId = `${selectId}-message`;
  const showMessage = invalid && message !== void 0;
  return /* @__PURE__ */ jsxs("div", { className: "armada-select-field", children: [
    label2 !== void 0 && /* @__PURE__ */ jsx("label", { className: "armada-select-field__label", htmlFor: selectId, children: label2 }),
    /* @__PURE__ */ jsxs("span", { className: "armada-select-shell", children: [
      /* @__PURE__ */ jsx(
        "select",
        {
          ...rest,
          id: selectId,
          className: "armada-select",
          "aria-invalid": invalid || void 0,
          "aria-describedby": showMessage ? messageId : void 0,
          children
        }
      ),
      /* @__PURE__ */ jsx(ChevronDown, { className: "armada-select__caret", size: 16, strokeWidth: 2, "aria-hidden": true })
    ] }),
    showMessage && /* @__PURE__ */ jsx("span", { className: "armada-select-field__message", id: messageId, children: message })
  ] });
}
function TabsWithCounts({
  items,
  value,
  defaultValue,
  onChange,
  suspended = false
}) {
  const [internal, setInternal] = useState(defaultValue ?? items[0]?.id);
  const active = value ?? internal;
  function select(id) {
    if (value === void 0) setInternal(id);
    onChange?.(id);
  }
  function onKey(event) {
    const at = items.findIndex((item) => item.id === active);
    if (items.length === 0) {
      return;
    }
    if (event.key === "ArrowRight") {
      event.preventDefault();
      select(items[(at + 1) % items.length].id);
    }
    if (event.key === "ArrowLeft") {
      event.preventDefault();
      select(items[(at - 1 + items.length) % items.length].id);
    }
  }
  return /* @__PURE__ */ jsx(
    "div",
    {
      className: "armada-tabs-counts",
      role: "tablist",
      "data-suspended": suspended || void 0,
      onKeyDown: onKey,
      children: items.map((item) => /* @__PURE__ */ jsxs(
        "button",
        {
          type: "button",
          role: "tab",
          "aria-selected": item.id === active,
          tabIndex: item.id === active ? 0 : -1,
          className: item.id === active ? "armada-tabs-counts__tab armada-tabs-counts__tab--active" : "armada-tabs-counts__tab",
          onClick: () => select(item.id),
          "aria-keyshortcuts": item.shortcut,
          children: [
            item.shortcut ? /* @__PURE__ */ jsx(Kbd, { "aria-hidden": true, children: item.shortcut }) : null,
            item.label,
            item.count ? /* @__PURE__ */ jsx("span", { className: "armada-tabs-counts__count", children: item.count }) : null
          ]
        },
        item.id
      ))
    }
  );
}
const SEARCHES_EVERYTHING = "Search every job";
function BoardControls({
  query,
  onQuery,
  placeholder = SEARCHES_EVERYTHING,
  searchRef,
  onLeaveSearch,
  sorts,
  sort,
  onSort,
  tabs,
  tab,
  onTab,
  suspended = false,
  searchKey
}) {
  function onSearchKey(event) {
    if (event.key !== "Escape") return;
    event.preventDefault();
    event.stopPropagation();
    onQuery("");
    onLeaveSearch?.();
  }
  return /* @__PURE__ */ jsxs("div", { className: "armada-board-controls", children: [
    /* @__PURE__ */ jsxs("div", { className: "armada-board-controls__line", children: [
      /* @__PURE__ */ jsx("div", { className: "armada-board-controls__search", children: /* @__PURE__ */ jsx(
        Input,
        {
          ref: searchRef,
          type: "search",
          value: query,
          placeholder,
          "aria-label": placeholder,
          onChange: (event) => onQuery(event.target.value),
          onKeyDown: onSearchKey
        }
      ) }),
      searchKey === void 0 ? null : /* @__PURE__ */ jsx(Kbd, { className: "armada-board-controls__hint", "aria-hidden": true, children: searchKey }),
      /* @__PURE__ */ jsx("div", { className: "armada-board-controls__sort", children: /* @__PURE__ */ jsx(Select, { "aria-label": "Sort", value: sort, onChange: (event) => onSort(event.target.value), children: sorts.map((option) => /* @__PURE__ */ jsx("option", { value: option.id, children: option.label }, option.id)) }) })
    ] }),
    /* @__PURE__ */ jsx(TabsWithCounts, { items: [...tabs], value: tab, onChange: onTab, suspended })
  ] });
}
const meta$15 = {
  title: "Compositions/Board controls",
  component: BoardControls
};
const SORTS = [
  { id: "critical_first", label: "Critical first" },
  { id: "oldest_first", label: "Oldest first" }
];
const TABS = [
  { id: "all", label: "All", count: 15, shortcut: "1" },
  { id: "needs-you", label: "Needs you", count: 4, shortcut: "2" },
  { id: "running", label: "Running", count: 6, shortcut: "3" },
  { id: "queued", label: "Queued", count: 2, shortcut: "4" },
  { id: "finished", label: "Finished", count: 3, shortcut: "5" }
];
function Live$1(props) {
  const [query, setQuery] = useState(props.query ?? "");
  const [sort, setSort] = useState(props.sort ?? "critical_first");
  const [tab, setTab] = useState(props.tab ?? "all");
  return /* @__PURE__ */ jsx("div", { className: "armada-screen", children: /* @__PURE__ */ jsx(
    BoardControls,
    {
      sorts: SORTS,
      tabs: TABS,
      searchKey: "/",
      ...props,
      query,
      onQuery: setQuery,
      sort,
      onSort: setSort,
      tab,
      onTab: (next) => {
        setTab(next);
        setQuery("");
      },
      suspended: query.trim() !== ""
    }
  ) });
}
const Resting$2 = { render: () => /* @__PURE__ */ jsx(Live$1, {}) };
const Searching = {
  render: () => /* @__PURE__ */ jsx(
    Live$1,
    {
      query: "poke",
      tab: "running",
      tabs: [
        { id: "all", label: "All", count: 3, shortcut: "1" },
        { id: "needs-you", label: "Needs you", count: 1, shortcut: "2" },
        { id: "running", label: "Running", count: 2, shortcut: "3" },
        { id: "queued", label: "Queued", shortcut: "4" },
        { id: "finished", label: "Finished", shortcut: "5" }
      ]
    }
  )
};
const NothingNeedsYou = {
  render: () => /* @__PURE__ */ jsx(
    Live$1,
    {
      tab: "needs-you",
      tabs: [
        { id: "all", label: "All", count: 9, shortcut: "1" },
        { id: "needs-you", label: "Needs you", shortcut: "2" },
        { id: "running", label: "Running", count: 6, shortcut: "3" },
        { id: "queued", label: "Queued", count: 3, shortcut: "4" },
        { id: "finished", label: "Finished", shortcut: "5" }
      ]
    }
  )
};
const __vite_glob_0_2 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  NothingNeedsYou,
  Resting: Resting$2,
  Searching,
  default: meta$15
}, Symbol.toStringTag, { value: "Module" }));
function BoardEmptyState({
  children,
  quiet = false,
  command,
  note,
  action,
  onCopied
}) {
  const copy = useCallback(
    (event, value) => {
      event.stopPropagation();
      void navigator.clipboard.writeText(value).then(
        () => onCopied?.(value),
        // A failed clipboard write is otherwise indistinguishable from a dead
        // element, so the surface is told either way.
        () => onCopied?.(value)
      );
    },
    [onCopied]
  );
  return /* @__PURE__ */ jsxs("div", { className: "armada-board-empty", children: [
    /* @__PURE__ */ jsx("span", { className: "armada-board-empty__line", "data-quiet": quiet || void 0, children }),
    command !== void 0 ? /* @__PURE__ */ jsx("span", { className: "armada-board-empty__command", onClick: (e) => copy(e, command), children: command }) : null,
    note ? /* @__PURE__ */ jsx("span", { className: "armada-board-empty__note", children: note }) : null,
    action
  ] });
}
const meta$14 = {
  title: "Compositions/Board empty state",
  component: BoardEmptyState
};
const FleetRunningNoJobs = {
  args: {
    quiet: true,
    children: "No jobs. Fleet has been up 6 days.",
    action: /* @__PURE__ */ jsx(Button, { variant: "primary", children: "New job" })
  }
};
const FleetIsNotRunning$1 = {
  args: {
    children: "Fleet is not running. Bridge has nothing to read.",
    command: "armada-fleet start",
    note: "Run that in a terminal. Bridge connects on its own once the runtime file appears."
  }
};
const __vite_glob_0_3 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  FleetIsNotRunning: FleetIsNotRunning$1,
  FleetRunningNoJobs,
  default: meta$14
}, Symbol.toStringTag, { value: "Module" }));
const OUTSIDE_PLAN$1 = "outside plan";
function changedFilesSummary(files, planDeclared) {
  const parts = [`${files.length} ${files.length === 1 ? "file" : "files"}`];
  const churn = countsOf({
    added: sum(files, (file) => file.added),
    deleted: sum(files, (file) => file.deleted)
  });
  if (churn !== void 0) {
    parts.push([churn.added, churn.deleted].filter((n) => n !== void 0).join(" "));
  }
  if (planDeclared === true) {
    const outside = files.filter((file) => file.outsidePlan === true).length;
    parts.push(outside === 0 ? "all inside the plan" : `${outside} outside the plan`);
  }
  return parts.join(" · ");
}
function sum(files, of) {
  const counted2 = files.map(of).filter((n) => n !== void 0);
  return counted2.length === 0 ? void 0 : counted2.reduce((a, b) => a + b, 0);
}
function countsOf({ added, deleted }) {
  const plus = added === void 0 || added === 0 ? void 0 : `+${added}`;
  const minus = deleted === void 0 || deleted === 0 ? void 0 : `−${deleted}`;
  if (plus === void 0 && minus === void 0) return void 0;
  return { added: plus, deleted: minus };
}
function ChangedFiles({ files, emptyNote, note, onCopied }) {
  const copy = useCallback(
    (event, value) => {
      event.stopPropagation();
      void navigator.clipboard.writeText(value).then(() => onCopied?.(value));
    },
    [onCopied]
  );
  if (files.length === 0) {
    return /* @__PURE__ */ jsx("p", { className: "armada-files__empty", role: "note", children: emptyNote });
  }
  const counted2 = files.some((file) => countsOf(file) !== void 0);
  return /* @__PURE__ */ jsxs("div", { className: "armada-files", children: [
    /* @__PURE__ */ jsx("ol", { className: "armada-files__list", "data-counts": counted2 || void 0, children: files.map((file) => /* @__PURE__ */ jsxs(
      "li",
      {
        className: "armada-files__file",
        "data-outside": file.outsidePlan === true || void 0,
        children: [
          /* @__PURE__ */ jsx("span", { className: "armada-files__change", children: file.change }),
          /* @__PURE__ */ jsx(
            "span",
            {
              className: "armada-files__path",
              title: file.path,
              onClick: (event) => copy(event, file.path),
              children: file.path
            }
          ),
          counted2 ? /* @__PURE__ */ jsx(Counts, { file }) : null,
          /* @__PURE__ */ jsx("span", { className: "armada-files__mark", children: file.outsidePlan === true ? OUTSIDE_PLAN$1 : null })
        ]
      },
      file.path
    )) }),
    note === void 0 ? null : /* @__PURE__ */ jsx("p", { className: "armada-files__note", children: note })
  ] });
}
function Counts({ file }) {
  const counts = countsOf(file);
  if (counts === void 0) return /* @__PURE__ */ jsx("span", { className: "armada-files__counts" });
  return /* @__PURE__ */ jsxs("span", { className: "armada-files__counts", children: [
    counts.added === void 0 ? null : /* @__PURE__ */ jsx("span", { className: "armada-files__added", children: counts.added }),
    counts.deleted === void 0 ? null : /* @__PURE__ */ jsx("span", { className: "armada-files__deleted", children: counts.deleted })
  ] });
}
const meta$13 = {
  title: "Compositions/Changed files",
  component: ChangedFiles
};
const NOTHING_YET$2 = "This drone has not changed anything yet.";
const COUNTED = [
  { path: "packages/settings/src/selectors.ts", change: "modified", added: 61, deleted: 4 },
  { path: "packages/settings/src/reducer.ts", change: "modified", added: 12, deleted: 27 },
  // No deletion beside it, as the drawing has it. `−0` measures nothing.
  { path: "packages/settings/src/index.ts", change: "added", added: 21 }
];
const DRIFTED = [
  ...COUNTED,
  { path: "packages/tokens/src/status.css", change: "modified", added: 3, deleted: 3, outsidePlan: true },
  { path: "scripts/legacy-dev", change: "deleted", deleted: 40, outsidePlan: true }
];
const WhatADroneHasTouched = {
  args: {
    emptyNote: NOTHING_YET$2,
    note: "Read from the worktree while the drone was working. This step declared no plan, so no row is marked.",
    files: [
      { path: "crates/api/src/routes.rs", change: "modified" },
      { path: "crates/ipc/src/history.rs", change: "added" },
      { path: "crates/ipc/src/lib.rs", change: "modified" },
      { path: "crates/ipc/src/legacy_events.rs", change: "deleted" }
    ]
  }
};
const TwoPathsOutsideThePlan = {
  args: {
    emptyNote: NOTHING_YET$2,
    note: "Read from the worktree while the drone was working. 2 of 5 paths are outside the plan this step declared.",
    files: [
      { path: "apps/desktop/src/renderer/src/JobDetail.tsx", change: "modified" },
      { path: "apps/desktop/src/shared/protocol.ts", change: "modified" },
      { path: "packages/tokens/src/status.css", change: "modified", outsidePlan: true },
      { path: "packages/components/src/compositions/ChangedFiles/ChangedFiles.tsx", change: "added" },
      { path: "scripts/dev", change: "type_changed", outsidePlan: true }
    ]
  }
};
const NoPlanIsRecordedAgainstIt = {
  args: {
    emptyNote: NOTHING_YET$2,
    note: "Read from this job's worktree when the job stopped, and kept — so it says the same thing whether or not anyone was watching. No plan is recorded against it, so no path is marked. Either no step declared one, or this job stopped before declarations were kept. An unmarked path here is not a path that was inside a plan.",
    files: [
      { path: "crates/fleet/src/dispatch.rs", change: "modified" },
      { path: "crates/fleet/src/footprint.rs", change: "modified" },
      { path: "crates/ipc/src/work.rs", change: "modified" }
    ]
  }
};
const TheKindsThatAreNotAnEdit = {
  args: {
    emptyNote: NOTHING_YET$2,
    files: [
      { path: "docs/scope.md", change: "renamed" },
      { path: "docs/journeys/watch-a-drone.md", change: "copied" },
      { path: "crates/fleet/src/serving.rs", change: "conflicted" },
      { path: "assets/AppIcon.icns", change: "unreadable" }
    ]
  }
};
const NothingChangedYet = {
  args: { files: [], emptyNote: NOTHING_YET$2 }
};
const WithLineCounts = {
  args: { emptyNote: NOTHING_YET$2, files: COUNTED }
};
const TheSummaryOverTheList = {
  render: () => /* @__PURE__ */ jsx("div", { style: { display: "flex", flexDirection: "column", gap: "var(--space-4)" }, children: [
    [COUNTED, true],
    [DRIFTED, true],
    [DRIFTED, void 0]
  ].map(([files, planDeclared], at) => /* @__PURE__ */ jsxs("div", { style: { display: "flex", flexDirection: "column", gap: "var(--space-2)" }, children: [
    /* @__PURE__ */ jsx("span", { className: "armada-chapter__meta", children: changedFilesSummary(files, planDeclared) }),
    /* @__PURE__ */ jsx(ChangedFiles, { files, emptyNote: NOTHING_YET$2 })
  ] }, at)) })
};
const __vite_glob_0_4 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  NoPlanIsRecordedAgainstIt,
  NothingChangedYet,
  TheKindsThatAreNotAnEdit,
  TheSummaryOverTheList,
  TwoPathsOutsideThePlan,
  WhatADroneHasTouched,
  WithLineCounts,
  default: meta$13
}, Symbol.toStringTag, { value: "Module" }));
const GLYPH$5 = 12;
const STROKE$5 = 2;
function Chapter({
  ordinal,
  name,
  meta: meta2,
  live,
  tone = "neutral",
  open: open2 = true,
  onToggle,
  children,
  moreLabel,
  onMore,
  moreCloses,
  bodyId
}) {
  const head = /* @__PURE__ */ jsxs(Fragment, { children: [
    /* @__PURE__ */ jsx("span", { className: "armada-chapter__n", "aria-hidden": true, children: ordinal }),
    /* @__PURE__ */ jsx("span", { className: "armada-chapter__name", children: name }),
    meta2 === void 0 && !live ? null : /* @__PURE__ */ jsxs("span", { className: "armada-chapter__meta", children: [
      live ? /* @__PURE__ */ jsx("span", { className: "armada-chapter__live", "aria-hidden": true }) : null,
      meta2
    ] })
  ] });
  return /* @__PURE__ */ jsxs("section", { className: "armada-chapter", "data-tone": tone, "data-open": open2 || void 0, children: [
    onToggle === void 0 ? /* @__PURE__ */ jsx("div", { className: "armada-chapter__head", children: head }) : /* @__PURE__ */ jsx(
      "button",
      {
        type: "button",
        className: "armada-chapter__head",
        "aria-expanded": open2,
        "aria-controls": bodyId,
        onClick: onToggle,
        children: head
      }
    ),
    /* @__PURE__ */ jsxs("div", { className: "armada-chapter__body", id: bodyId, hidden: !open2, children: [
      children,
      moreLabel === void 0 || onMore === void 0 ? null : /* @__PURE__ */ jsxs("button", { type: "button", className: "armada-chapter__more", onClick: onMore, children: [
        moreLabel,
        moreCloses ? /* @__PURE__ */ jsx(ChevronDown, { size: GLYPH$5, strokeWidth: STROKE$5, "aria-hidden": true }) : /* @__PURE__ */ jsx(ChevronRight, { size: GLYPH$5, strokeWidth: STROKE$5, "aria-hidden": true })
      ] })
    ] })
  ] });
}
const meta$12 = {
  title: "Compositions/Chapter",
  component: Chapter,
  decorators: [
    // The panel a chapter lives in. A well on the canvas would be judged
    // against the wrong ground: the chapter is --bg-sunken precisely because
    // the panel around it is --bg-raised.
    (Story) => /* @__PURE__ */ jsx(
      "div",
      {
        style: {
          width: "calc(var(--space-12) * 12)",
          padding: "var(--space-4)",
          borderRadius: "var(--radius-md)",
          border: "var(--border-width) solid var(--border-default)",
          background: "var(--bg-raised)"
        },
        children: /* @__PURE__ */ jsx(Story, {})
      }
    )
  ]
};
const Open$2 = {
  args: {
    ordinal: 1,
    name: "Drone instructions",
    meta: "14:22:07",
    open: true,
    onToggle: () => {
    },
    bodyId: "chapter-open",
    children: /* @__PURE__ */ jsx("p", { style: { margin: 0, fontSize: "var(--text-xs)", lineHeight: "var(--leading-xs)", color: "var(--fg-muted)" }, children: "Move the selector block into its own module so the tests can import it without constructing the store. Do not change reducer behaviour." }),
    moreLabel: "Criteria and what it was given — 2 and 2",
    onMore: () => {
    }
  }
};
const CollapsedToItsHeader = {
  args: {
    ordinal: 1,
    name: "Drone instructions",
    meta: "14:22:07",
    open: false,
    onToggle: () => {
    },
    bodyId: "chapter-collapsed",
    children: /* @__PURE__ */ jsx("p", { children: "Never seen while the chapter is shut." })
  }
};
const WithHeaderMeta = {
  args: {
    ordinal: 3,
    name: "Produced",
    meta: "3 files · +94 −31 · all inside the plan",
    open: false,
    onToggle: () => {
    },
    bodyId: "chapter-meta"
  }
};
const Live = {
  args: {
    ordinal: 2,
    name: "Activity log",
    live: true,
    meta: "live · 47 entries · every line opens",
    open: false,
    onToggle: () => {
    },
    bodyId: "chapter-live"
  }
};
const TheChapterThatNeedsYou = {
  args: {
    ordinal: 4,
    name: "Your decision",
    meta: "nothing advances until you answer",
    tone: "waiting",
    open: true,
    bodyId: "chapter-waiting",
    children: /* @__PURE__ */ jsx("p", { style: { margin: 0, fontSize: "var(--text-xs)", lineHeight: "var(--leading-xs)", color: "var(--fg-muted)" }, children: "Every Check passed and both criteria were met. This workflow asks for a person at this step whatever the gates came to." })
  }
};
const OneOpenAtATime = {
  render: () => /* @__PURE__ */ jsxs("div", { style: { display: "flex", flexDirection: "column", gap: "var(--space-3)" }, children: [
    /* @__PURE__ */ jsx(
      Chapter,
      {
        ordinal: 1,
        name: "Drone instructions",
        meta: "14:22:07",
        open: false,
        onToggle: () => {
        },
        bodyId: "one-1"
      }
    ),
    /* @__PURE__ */ jsx(
      Chapter,
      {
        ordinal: 2,
        name: "Activity log",
        live: true,
        meta: "47 entries",
        open: true,
        onToggle: () => {
        },
        bodyId: "one-2",
        moreLabel: "Close",
        moreCloses: true,
        onMore: () => {
        },
        children: /* @__PURE__ */ jsx(
          "p",
          {
            style: {
              margin: 0,
              fontSize: "var(--text-xs)",
              lineHeight: "var(--leading-xs)",
              color: "var(--fg-muted)"
            },
            children: "Forty-seven entries, in the order they happened."
          }
        )
      }
    ),
    /* @__PURE__ */ jsx(
      Chapter,
      {
        ordinal: 3,
        name: "Produced",
        meta: "3 files · +94 −31",
        open: false,
        onToggle: () => {
        },
        bodyId: "one-3"
      }
    )
  ] })
};
const __vite_glob_0_5 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  CollapsedToItsHeader,
  Live,
  OneOpenAtATime,
  Open: Open$2,
  TheChapterThatNeedsYou,
  WithHeaderMeta,
  default: meta$12
}, Symbol.toStringTag, { value: "Module" }));
const VERDICT_ICON = 12;
const VERDICT_STROKE = 2;
const CITED = [
  "expected",
  "produced",
  "consequence"
];
const LABELLED = {
  expected: "Expected",
  produced: "Produced",
  consequence: "Consequence"
};
function refusalsFirst(rows) {
  return [...rows].sort((a, b) => Number(b.named === "not_met") - Number(a.named === "not_met"));
}
function CriterionVerdicts({ rows, label: label2, onCopied }) {
  const copy = useCallback(
    (event, value) => {
      event.stopPropagation();
      void navigator.clipboard.writeText(value).then(
        () => onCopied?.(value),
        () => onCopied?.(value)
      );
    },
    [onCopied]
  );
  return /* @__PURE__ */ jsxs("div", { className: "armada-verdicts", children: [
    label2 ? /* @__PURE__ */ jsx("span", { className: "armada-verdicts__label", children: label2 }) : null,
    /* @__PURE__ */ jsx("ul", { className: "armada-verdicts__list", children: refusalsFirst(rows).map((row) => {
      const cited = CITED.filter((field) => row[field] !== void 0);
      return /* @__PURE__ */ jsxs("li", { className: "armada-verdicts__row", "data-verdict": row.named, children: [
        /* @__PURE__ */ jsxs("div", { className: "armada-verdicts__head", children: [
          /* @__PURE__ */ jsx("span", { className: "armada-verdicts__mark", children: row.icon ? /* @__PURE__ */ jsx(row.icon, { size: VERDICT_ICON, strokeWidth: VERDICT_STROKE, "aria-hidden": true }) : null }),
          row.ordinal === void 0 ? null : /* @__PURE__ */ jsx("span", { className: "armada-verdicts__ordinal", children: `${row.ordinal}.` }),
          row.text === void 0 ? /* @__PURE__ */ jsx("span", { className: "armada-verdicts__id", children: row.criterionId }) : /* @__PURE__ */ jsx("span", { className: "armada-verdicts__text", children: row.text }),
          row.verdict ? /* @__PURE__ */ jsx("span", { className: "armada-verdicts__verb", children: row.verdict }) : null,
          row.briefPath === void 0 ? null : (
            // The whole path is on the clipboard and in the title however
            // narrow the row gets, the way the rail's output path is: a
            // copy truncated with the display would be worse than the
            // overflow it fixed.
            /* @__PURE__ */ jsx(
              "span",
              {
                className: "armada-verdicts__brief",
                title: row.briefPath,
                "data-copies": "true",
                onClick: (e) => copy(e, row.briefPath),
                children: row.briefPath
              }
            )
          )
        ] }),
        cited.length === 0 ? null : (
          // A refusal's citation, in the Judge record's own field names.
          // Three labelled lines rather than one sentence: the fields
          // arrive named, and composing prose out of them here would be
          // writing copy the Judge did not.
          /* @__PURE__ */ jsx("dl", { className: "armada-verdicts__cited", children: cited.map((field) => (
            // Both halves are direct children of one grid, so the three
            // values share a column edge. A wrapper per pair would give
            // each its own grid and align nothing.
            /* @__PURE__ */ jsxs(Fragment$1, { children: [
              /* @__PURE__ */ jsx("dt", { className: "armada-verdicts__cite-label", children: LABELLED[field] }),
              /* @__PURE__ */ jsx("dd", { className: "armada-verdicts__cite-value", "data-field": field, children: row[field] })
            ] }, field)
          )) })
        )
      ] }, row.criterionId);
    }) })
  ] });
}
function Prose({ text }) {
  const blocks = read(text);
  if (blocks.length === 0) return null;
  return /* @__PURE__ */ jsx("div", { className: "armada-prose", children: blocks.map((block, at) => /* @__PURE__ */ jsx(Fragment$1, { children: drawn(block, at) }, at)) });
}
const FENCE = "```";
const BULLETS = ["- ", "* "];
function read(text) {
  const blocks = [];
  const lines = text.split("\n");
  let at = 0;
  while (at < lines.length) {
    const line = lines[at] ?? "";
    const trimmed = line.trim();
    if (trimmed.startsWith(FENCE)) {
      const held2 = [];
      at += 1;
      while (at < lines.length && !(lines[at] ?? "").trim().startsWith(FENCE)) {
        held2.push(lines[at] ?? "");
        at += 1;
      }
      at += 1;
      blocks.push({ kind: "code", lines: held2 });
      continue;
    }
    if (trimmed === "") {
      at += 1;
      continue;
    }
    if (bulleted(trimmed)) {
      const items = [];
      while (at < lines.length && bulleted((lines[at] ?? "").trim())) {
        items.push((lines[at] ?? "").trim().slice(2));
        at += 1;
      }
      blocks.push({ kind: "list", items });
      continue;
    }
    if (heading(trimmed)) {
      blocks.push({ kind: "said", line: trimmed.replace(/^#+\s+/, "") });
      at += 1;
      continue;
    }
    const held = [];
    while (at < lines.length) {
      const next = (lines[at] ?? "").trim();
      if (next === "" || next.startsWith(FENCE) || bulleted(next) || heading(next)) break;
      held.push(next);
      at += 1;
    }
    blocks.push({ kind: "paragraph", lines: held });
  }
  return blocks;
}
function bulleted(line) {
  return BULLETS.some((mark) => line.startsWith(mark));
}
function heading(line) {
  return /^#{1,6}\s+/.test(line);
}
function drawn(block, at) {
  if (block.kind === "code") {
    return /* @__PURE__ */ jsx("pre", { className: "armada-prose__block", children: /* @__PURE__ */ jsx("code", { children: block.lines.join("\n") }) });
  }
  if (block.kind === "list") {
    return /* @__PURE__ */ jsx("ul", { className: "armada-prose__list", children: block.items.map((item, i) => /* @__PURE__ */ jsx("li", { className: "armada-prose__item", children: inline(item, `${at}-${i}`) }, i)) });
  }
  if (block.kind === "said") {
    return /* @__PURE__ */ jsx("p", { className: "armada-prose__said", children: inline(block.line, `${at}`) });
  }
  return /* @__PURE__ */ jsx("p", { className: "armada-prose__paragraph", children: inline(block.lines.join(" "), `${at}`) });
}
function inline(text, key) {
  const out = [];
  text.split(/(`[^`]+`)/g).forEach((part, i) => {
    if (part === "") return;
    if (part.length > 2 && part.startsWith("`") && part.endsWith("`")) {
      out.push(
        /* @__PURE__ */ jsx("code", { className: "armada-prose__code", children: part.slice(1, -1) }, `${key}-c${i}`)
      );
      return;
    }
    out.push(...emphasised(part, `${key}-${i}`));
  });
  return out;
}
function emphasised(text, key) {
  return text.split(/(\*\*[^*]+\*\*|\*[^*\s][^*]*\*|_[^_\s][^_]*_)/g).flatMap((part, i) => {
    if (part === "") return [];
    if (part.startsWith("**") && part.endsWith("**")) {
      return [
        /* @__PURE__ */ jsx("strong", { className: "armada-prose__strong", children: part.slice(2, -2) }, `${key}-b${i}`)
      ];
    }
    if (part.startsWith("*") && part.endsWith("*") || part.startsWith("_") && part.endsWith("_")) {
      return [/* @__PURE__ */ jsx("em", { children: part.slice(1, -1) }, `${key}-i${i}`)];
    }
    return [/* @__PURE__ */ jsx(Fragment$1, { children: part }, `${key}-t${i}`)];
  });
}
function GamingFlags({
  flags,
  said,
  citation = "clipped",
  onOpenAt
}) {
  if (flags.length === 0) return null;
  return /* @__PURE__ */ jsxs("div", { className: "armada-gaming-flags", "data-citation": citation, children: [
    said === void 0 ? null : /* @__PURE__ */ jsx("span", { className: "armada-gaming-flags__said", children: said }),
    /* @__PURE__ */ jsx("ul", { className: "armada-gaming-flags__list", children: flags.map((flag, at) => /* @__PURE__ */ jsxs("li", { className: "armada-gaming-flags__flag", children: [
      /* @__PURE__ */ jsx(
        "span",
        {
          className: "armada-gaming-flags__pattern",
          "data-verb": flag.verb === void 0 ? void 0 : "true",
          children: flag.verb ?? flag.pattern
        }
      ),
      flag.cited === void 0 || flag.cited === "" ? null : citation === "whole" ? /* @__PURE__ */ jsx("div", { className: "armada-gaming-flags__cited", children: /* @__PURE__ */ jsx(Prose, { text: flag.cited }) }) : (
        // The whole citation stays in the title however narrow the row
        // gets, the way the Check's output path does.
        /* @__PURE__ */ jsx("span", { className: "armada-gaming-flags__cited", title: flag.cited, children: flag.cited })
      ),
      flag.at === void 0 ? null : /* @__PURE__ */ jsx(Where, { at: flag.at, onOpen: onOpenAt })
    ] }, `flag-${at}`)) })
  ] });
}
function Where({ at, onOpen }) {
  const where = at.line === void 0 ? at.file : `${at.file}:${at.line}`;
  if (onOpen === void 0) {
    return /* @__PURE__ */ jsx("span", { className: "armada-gaming-flags__at", title: where, children: where });
  }
  return /* @__PURE__ */ jsx(
    "button",
    {
      type: "button",
      className: "armada-gaming-flags__at",
      "data-opens": "true",
      title: where,
      onClick: () => onOpen(at),
      children: where
    }
  );
}
const GLYPH$4 = {
  not_started: void 0,
  running: CircleDot,
  awaiting_human: Eye,
  retrying: RotateCw,
  advanced: Check,
  stopped: Flag,
  killed: Power,
  failed: X
};
const MARK_ICON = 12;
const MARK_STROKE$1 = 2;
function StepActivityMark({ activity, label: label2, ordinal, pulsing = false }) {
  const Icon = GLYPH$4[activity];
  const animates = pulsing && activity === "running";
  return /* @__PURE__ */ jsxs("span", { className: "armada-step-mark", "data-activity": activity, "data-pulsing": animates || void 0, children: [
    Icon ? /* @__PURE__ */ jsx(Icon, { size: MARK_ICON, strokeWidth: MARK_STROKE$1, "aria-hidden": true }) : ordinal !== void 0 ? /* @__PURE__ */ jsx("span", { className: "armada-step-mark__ordinal", "aria-hidden": true, children: ordinal }) : (
      // A step that has not run, with no number to stand in for it: the
      // Journey 4 drawing gives it a hollow ring, and the slot rendered
      // nothing at all before. Not a glyph — the icon registry carries no
      // bare circle and `packages/icons/` is not this component's to write
      // — so it is a ring in the stylesheet, the same class of object as
      // the running dot and the degraded dot already drawn there. Reported.
      /* @__PURE__ */ jsx("span", { className: "armada-step-mark__ring", "aria-hidden": true })
    ),
    /* @__PURE__ */ jsx("span", { className: "armada-step-mark__name", children: label2 })
  ] });
}
const GATE_ICON = 12;
const GATE_STROKE = 2;
const FLAGGED = "the gaming check flagged this evidence";
function named(step) {
  return (step.evidence?.label ?? "") !== "";
}
function WorkflowRail({ steps, pulsing = false, onCopied }) {
  const copy = useCallback(
    (event, value) => {
      event.stopPropagation();
      void navigator.clipboard.writeText(value).then(
        // A failed clipboard write is otherwise indistinguishable from a dead
        // element, so the surface is told either way.
        () => onCopied?.(value),
        () => onCopied?.(value)
      );
    },
    [onCopied]
  );
  return /* @__PURE__ */ jsx("ol", { className: "armada-rail", children: steps.map((step, i) => {
    const gates = step.gates ?? [];
    const declarations = step.declarations ?? [];
    const flags = step.flags ?? [];
    return /* @__PURE__ */ jsxs("li", { className: "armada-rail__step", children: [
      /* @__PURE__ */ jsxs(
        "div",
        {
          className: "armada-rail__row",
          "data-activity": step.activity,
          "data-current": step.current || void 0,
          children: [
            /* @__PURE__ */ jsx(
              StepActivityMark,
              {
                activity: step.activity,
                label: step.status ?? step.activity,
                ordinal: i + 1,
                pulsing: pulsing && step.current
              }
            ),
            /* @__PURE__ */ jsx(
              "span",
              {
                className: "armada-rail__name",
                "data-identifier": step.labelIsAnIdentifier || void 0,
                children: step.label
              }
            ),
            step.trailing,
            step.verdict ? /* @__PURE__ */ jsx("span", { className: "armada-rail__verdict", "data-verdict": step.verdictNamed, children: step.verdict }) : null,
            step.overridden ? /* @__PURE__ */ jsx("span", { className: "armada-rail__overridden", children: step.overridden }) : null,
            step.elapsed ? /* @__PURE__ */ jsx("span", { className: "armada-rail__elapsed", children: step.elapsed }) : null,
            step.status ? /* @__PURE__ */ jsx("span", { className: "armada-rail__status", children: step.status }) : null
          ]
        }
      ),
      gates.length + declarations.length === 0 ? null : /* @__PURE__ */ jsxs("ul", { className: "armada-rail__gates", children: [
        gates.map((gate, g) => /* @__PURE__ */ jsxs("li", { className: "armada-rail__gate", children: [
          /* @__PURE__ */ jsxs("span", { className: "armada-rail__gate-mark", children: [
            gate.icon ? /* @__PURE__ */ jsx(gate.icon, { size: GATE_ICON, strokeWidth: GATE_STROKE, "aria-hidden": true }) : null,
            gate.iconLabel ? /* @__PURE__ */ jsx("span", { className: "armada-rail__sr", children: gate.iconLabel }) : null
          ] }),
          /* @__PURE__ */ jsx("span", { className: "armada-rail__gate-command", children: gate.command }),
          gate.covers ? /* @__PURE__ */ jsx("span", { className: "armada-rail__gate-covers", title: gate.covers, children: gate.covers }) : null,
          gate.result ? /* @__PURE__ */ jsx("span", { className: "armada-rail__gate-result", children: gate.result }) : null,
          gate.outputPath === void 0 ? null : (
            // The whole path is on the clipboard and in the title
            // however narrow the row gets: a copy truncated with the
            // display would be worse than the overflow it fixed.
            // `data-copies` carries a value rather than standing bare,
            // the way `Job row (stacked)` writes it.
            /* @__PURE__ */ jsx(
              "span",
              {
                className: "armada-rail__gate-output",
                title: gate.outputPath,
                "data-copies": "true",
                onClick: (e) => copy(e, gate.outputPath),
                children: gate.outputPath
              }
            )
          )
        ] }, g)),
        declarations.map((declared, d) => (
          // No glyph, and the mark column held open anyway: the row
          // reads as one of the list it sits in rather than starting a
          // second one at a different indent.
          /* @__PURE__ */ jsxs("li", { className: "armada-rail__gate", children: [
            /* @__PURE__ */ jsx("span", { className: "armada-rail__gate-mark" }),
            /* @__PURE__ */ jsx("span", { className: "armada-rail__gate-command", children: declared.label }),
            declared.result ? /* @__PURE__ */ jsx("span", { className: "armada-rail__gate-result", children: declared.result }) : null
          ] }, `declared-${d}`)
        ))
      ] }),
      gates.length + declarations.length + flags.length > 0 ? null : (
        // An ungated step says so in words. A step carrying no Check is
        // ordinary rather than exceptional, and a blank would read as a
        // gate that failed to render.
        /* @__PURE__ */ jsx("ul", { className: "armada-rail__gates", children: /* @__PURE__ */ jsxs("li", { className: "armada-rail__gate", "data-ungated": true, children: [
          /* @__PURE__ */ jsxs("span", { className: "armada-rail__gate-mark", children: [
            step.evidence?.icon ? /* @__PURE__ */ jsx(step.evidence.icon, { size: GATE_ICON, strokeWidth: GATE_STROKE, "aria-hidden": true }) : null,
            step.evidence?.iconLabel ? /* @__PURE__ */ jsx("span", { className: "armada-rail__sr", children: step.evidence.iconLabel }) : null
          ] }),
          named(step) ? /* @__PURE__ */ jsx("span", { className: "armada-rail__gate-command", children: step.evidence?.label }) : null,
          /* @__PURE__ */ jsx("span", { className: "armada-rail__gate-ungated", "data-alone": named(step) ? void 0 : "true", children: step.ungatedLabel ?? "no check on this step" })
        ] }) })
      ),
      flags.length === 0 ? null : /* @__PURE__ */ jsx("div", { className: "armada-rail__flags", children: /* @__PURE__ */ jsx(GamingFlags, { flags, said: FLAGGED, citation: "clipped" }) }),
      step.verdicts === void 0 || step.verdicts.length === 0 ? null : /* @__PURE__ */ jsx("div", { className: "armada-rail__verdicts", children: /* @__PURE__ */ jsx(CriterionVerdicts, { rows: step.verdicts, onCopied }) })
    ] }, step.id);
  }) });
}
const meta$11 = {
  title: "Compositions/Criterion verdicts",
  component: CriterionVerdicts
};
const MET = CircleCheck;
const NOT_MET = CircleX;
const refused = {
  ordinal: 2,
  criterionId: "c2",
  text: "A failed refresh signs the session out.",
  named: "not_met",
  verdict: "refused",
  icon: NOT_MET,
  expected: "A 401 from the refresh endpoint clears the session and returns the caller to sign-in.",
  produced: "The refresh error is swallowed in `session.ts:212` and the stale token is retried on the next request.",
  consequence: "A user whose refresh token has been revoked keeps a working-looking session until the next full reload, so a revoked device is not signed out."
};
const met = {
  ordinal: 1,
  criterionId: "c1",
  text: "Expired tokens refresh once rather than per request.",
  named: "met",
  verdict: "no objection",
  icon: MET
};
const ARefusal = {
  args: { rows: [refused] }
};
const NoObjection = {
  args: { rows: [met] }
};
const RefusalsSortFirst = {
  args: {
    label: "What the judge answered",
    rows: [
      met,
      refused,
      {
        ordinal: 3,
        criterionId: "c3",
        text: "The fix carries a regression test.",
        named: "met",
        verdict: "no objection",
        icon: MET
      }
    ]
  }
};
const TheCriterionIsNotOnScreen = {
  args: {
    rows: [
      {
        criterionId: "c4",
        named: "not_met",
        verdict: "refused",
        icon: NOT_MET,
        expected: "The migration is reversible.",
        produced: "`down()` is empty.",
        consequence: "A bad deploy cannot be rolled back without restoring from a snapshot."
      }
    ]
  }
};
const TheRegistryHasNoWordForIt = {
  args: {
    rows: [
      { ordinal: 1, criterionId: "c1", text: "The fix addresses the cause.", named: "unknown" }
    ]
  }
};
const WhereTheBriefWasKept = {
  args: {
    label: "What the judge answered",
    rows: [
      { ...met, briefPath: ".armada/briefs/01JOB/implement.1.c1.txt" },
      { ...refused, briefPath: ".armada/briefs/01JOB/implement.1.c2.txt" }
    ]
  }
};
const BeneathTheStepItJudged = {
  render: () => /* @__PURE__ */ jsx(
    WorkflowRail,
    {
      steps: [
        { id: "plan", label: "Plan the change", activity: "advanced", status: "advanced" },
        {
          id: "implement",
          label: "Implement",
          activity: "awaiting_human",
          status: "waiting on you",
          current: true,
          gates: [
            {
              command: "build · cargo build --workspace",
              result: "exit 0",
              icon: ShieldCheck,
              iconLabel: "Passed"
            }
          ],
          verdicts: [met, refused]
        },
        { id: "verify", label: "Run tests", activity: "not_started", status: "not started" }
      ]
    }
  )
};
const __vite_glob_0_6 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  ARefusal,
  BeneathTheStepItJudged,
  NoObjection,
  RefusalsSortFirst,
  TheCriterionIsNotOnScreen,
  TheRegistryHasNoWordForIt,
  WhereTheBriefWasKept,
  default: meta$11
}, Symbol.toStringTag, { value: "Module" }));
function Radio({ children, ...rest }) {
  return /* @__PURE__ */ jsxs("label", { className: "armada-radio", children: [
    /* @__PURE__ */ jsx("input", { ...rest, type: "radio", className: "armada-radio__input" }),
    /* @__PURE__ */ jsx("span", { className: "armada-radio__ring", "aria-hidden": "true", children: /* @__PURE__ */ jsx("span", { className: "armada-radio__dot" }) }),
    /* @__PURE__ */ jsx("span", { children })
  ] });
}
function RadioGroup({ label: label2, children }) {
  return /* @__PURE__ */ jsxs("div", { className: "armada-radio-group", role: "radiogroup", "aria-label": label2, children: [
    label2 !== void 0 && /* @__PURE__ */ jsx("span", { className: "armada-radio-group__label", children: label2 }),
    children
  ] });
}
function DroneQuestion({
  question,
  options,
  waiting,
  onAnswer,
  disabled = false,
  disabledNote,
  label: label2 = "The drone is waiting on you",
  redirectNote = "If none of these is right, redirect the drone instead — that is where your own words go.",
  answerLabel = "Send this answer"
}) {
  const [chosen, setChosen] = useState(null);
  return /* @__PURE__ */ jsxs("section", { className: "armada-question", "aria-label": "A question from the drone", children: [
    /* @__PURE__ */ jsxs("div", { className: "armada-question__head", children: [
      /* @__PURE__ */ jsx("span", { className: "armada-question__label", children: label2 }),
      waiting === void 0 ? null : /* @__PURE__ */ jsx("span", { className: "armada-question__waiting mono", children: waiting })
    ] }),
    /* @__PURE__ */ jsx("p", { className: "armada-question__asked", children: question }),
    /* @__PURE__ */ jsx(RadioGroup, { label: "Answers the drone offered", children: options.map((option) => /* @__PURE__ */ jsxs("div", { className: "armada-question__option", children: [
      /* @__PURE__ */ jsx(
        Radio,
        {
          name: "armada-question",
          value: option.label,
          checked: chosen === option.label,
          disabled,
          onChange: () => setChosen(option.label),
          children: option.label
        }
      ),
      /* @__PURE__ */ jsx("p", { className: "armada-question__means", children: option.consequence })
    ] }, option.label)) }),
    /* @__PURE__ */ jsx(
      Button,
      {
        variant: "primary",
        disabled: disabled || chosen === null,
        onClick: () => chosen !== null && onAnswer(chosen),
        children: answerLabel
      }
    ),
    /* @__PURE__ */ jsx("p", { className: "armada-question__said", children: redirectNote }),
    disabled && disabledNote !== void 0 ? /* @__PURE__ */ jsx("p", { className: "armada-question__said", role: "note", children: disabledNote }) : null
  ] });
}
const meta$10 = {
  title: "Compositions/Drone question",
  component: DroneQuestion
};
const TwoAnswers = {
  args: {
    question: "The store schema needs a column before three of these jobs can run. Should that be its own job?",
    options: [
      {
        label: "Its own job",
        consequence: "Dispatch a migration job first and make the other three depend on it. Nothing else starts until it lands."
      },
      {
        label: "Fold it in",
        consequence: "The first job that needs the column adds it. The other two may race it and one of them will have to wait anyway."
      }
    ],
    waiting: "12m",
    onAnswer: () => {
    }
  }
};
const FourAnswers = {
  args: {
    question: "How should this milestone's work be split?",
    options: [
      {
        label: "By crate",
        consequence: "One job per crate that changes — six jobs, each with its own scope."
      },
      {
        label: "By milestone step",
        consequence: "One job per step of the milestone as written — four jobs, two of which cross three crates."
      },
      {
        label: "By side of the seam",
        consequence: "Two jobs: everything in Rust, and everything in Bridge."
      },
      {
        label: "One job",
        consequence: "Do not split it. One drone works the whole milestone in one worktree."
      }
    ],
    waiting: "2h",
    onAnswer: () => {
    }
  }
};
const JustAsked = {
  args: {
    question: "Two issues in this milestone contradict each other on the gate. Which one holds?",
    options: [
      {
        label: "The Judge decides",
        consequence: "Gate the step on the Judge, and drop the human gate from it."
      },
      {
        label: "A person decides",
        consequence: "Keep the human gate, and the step stops for somebody every time."
      }
    ],
    onAnswer: () => {
    }
  }
};
const Sending = {
  args: {
    ...TwoAnswers.args,
    disabled: true,
    disabledNote: "That answer is already on its way to the drone."
  }
};
const NotConnected = {
  args: {
    ...TwoAnswers.args,
    disabled: true,
    disabledNote: "Fleet is not connected, so nothing can be sent. The drone is still waiting."
  }
};
const __vite_glob_0_7 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  FourAnswers,
  JustAsked,
  NotConnected,
  Sending,
  TwoAnswers,
  default: meta$10
}, Symbol.toStringTag, { value: "Module" }));
function DroneTurns({ turns: turns2, emptyNote, live = false }) {
  const [open2, setOpen] = useState(() => /* @__PURE__ */ new Set());
  const [list, setList] = useState(null);
  const following = useRef(false);
  const seeded = useRef(false);
  const drawn2 = useRef(0);
  const scroller = useRef(null);
  useEffect(() => {
    if (list === null || seeded.current) return;
    seeded.current = true;
    following.current = live;
  }, [list, live]);
  useEffect(() => {
    const found = scrollerFor(list);
    scroller.current = found;
    if (list === null || found === null) return void 0;
    const target = found === document.scrollingElement ? window : found;
    const read2 = () => {
      following.current = atBottom(found);
    };
    target.addEventListener("scroll", read2, { passive: true });
    return () => target.removeEventListener("scroll", read2);
  }, [list]);
  useEffect(() => {
    if (list === null || turns2.length === drawn2.current) return;
    drawn2.current = turns2.length;
    const pane = scroller.current;
    if (!following.current || pane === null) return;
    pane.scrollTop = pane.scrollHeight;
  }, [list, turns2.length]);
  if (turns2.length === 0) {
    return /* @__PURE__ */ jsx("p", { className: "armada-turns__empty", role: "note", children: emptyNote });
  }
  const entries2 = runs(turns2);
  return /* @__PURE__ */ jsx("ol", { className: "armada-turns", ref: setList, children: entries2.map(
    (entry, at) => entry.of === "step" ? /* @__PURE__ */ jsx(StepBoundary, { step: entry.step }, `step-${entry.above}`) : entry.of === "turn" ? /* @__PURE__ */ jsx(Row$1, { turn: entry.turn }, entry.turn.id) : /* @__PURE__ */ jsx(
      QuietRun,
      {
        turns: entry.turns,
        working: live && at === entries2.length - 1,
        open: open2.has(entry.turns[0].id),
        onToggle: () => setOpen(toggled(open2, entry.turns[0].id))
      },
      entry.turns[0].id
    )
  ) });
}
const BOTTOM_SLACK = 4;
function atBottom(scroller) {
  return scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight <= BOTTOM_SLACK;
}
function scrollerFor(from) {
  for (let at = from?.parentElement ?? null; at !== null; at = at.parentElement) {
    const overflow = getComputedStyle(at).overflowY;
    if (overflow === "auto" || overflow === "scroll") return at;
  }
  return document.scrollingElement;
}
function runs(turns2) {
  const attributed = turns2.some((turn) => turn.step !== void 0);
  const entries2 = [];
  let under;
  let opened = false;
  for (const turn of turns2) {
    if (attributed && (!opened || turn.step?.id !== under)) {
      entries2.push({ of: "step", step: turn.step, above: turn.id });
      under = turn.step?.id;
      opened = true;
    }
    const last = entries2[entries2.length - 1];
    if (turn.quiet !== true) {
      entries2.push({ of: "turn", turn });
      continue;
    }
    if (last !== void 0 && last.of === "quiet") {
      last.turns.push(turn);
      continue;
    }
    entries2.push({ of: "quiet", turns: [turn] });
  }
  return entries2;
}
function toggled(open2, id) {
  const next = new Set(open2);
  if (!next.delete(id)) next.add(id);
  return next;
}
const MARK$1 = 12;
const MARK_STROKE = 2;
const CARET = 16;
function QuietRun({ turns: turns2, working, open: open2, onToggle }) {
  const head = turns2[0];
  const held = open2 ? turns2.map((turn) => rowId(turn)).join(" ") : void 0;
  return /* @__PURE__ */ jsxs(Fragment$1, { children: [
    /* @__PURE__ */ jsxs("li", { className: "armada-turns__turn", "data-quiet": true, "data-open": open2 || void 0, children: [
      /* @__PURE__ */ jsx("span", { className: "armada-turns__at", children: head.at }),
      /* @__PURE__ */ jsx("span", { className: "armada-turns__mark", "data-working": working || void 0, children: /* @__PURE__ */ jsx(CircleDot, { size: MARK$1, strokeWidth: MARK_STROKE, "aria-hidden": true }) }),
      /* @__PURE__ */ jsxs("span", { className: "armada-turns__quiet-body", children: [
        working ? /* @__PURE__ */ jsx("span", { className: "armada-turns__working", children: "Working" }) : null,
        /* @__PURE__ */ jsx("span", { className: "armada-turns__count", children: counted(turns2.length) }),
        /* @__PURE__ */ jsxs(
          Button,
          {
            variant: "ghost",
            size: "sm",
            "aria-expanded": open2,
            "aria-controls": held,
            onClick: onToggle,
            children: [
              open2 ? /* @__PURE__ */ jsx(ChevronDown, { size: CARET, strokeWidth: MARK_STROKE, "aria-hidden": true }) : /* @__PURE__ */ jsx(ChevronRight, { size: CARET, strokeWidth: MARK_STROKE, "aria-hidden": true }),
              open2 ? "Hide details" : "Show details"
            ]
          }
        )
      ] })
    ] }),
    open2 ? turns2.map((turn) => /* @__PURE__ */ jsx(Row$1, { turn, nested: true }, turn.id)) : null
  ] });
}
const UNRECORDED = "Fleet recorded no step for the turns below";
function StepBoundary({ step }) {
  return /* @__PURE__ */ jsxs(
    "li",
    {
      className: "armada-turns__step",
      "data-unrecorded": step === void 0 || void 0,
      "data-identifier": step?.labelIsAnIdentifier || void 0,
      children: [
        /* @__PURE__ */ jsx("span", { className: "armada-turns__kind armada-turns__step-tag", children: "step" }),
        /* @__PURE__ */ jsx("span", { className: "armada-turns__step-name", children: step?.label ?? UNRECORDED })
      ]
    }
  );
}
function Row$1({ turn, nested = false }) {
  return /* @__PURE__ */ jsxs("li", { className: "armada-turns__turn", id: rowId(turn), "data-nested": nested || void 0, children: [
    /* @__PURE__ */ jsx("span", { className: "armada-turns__at", children: turn.at }),
    /* @__PURE__ */ jsx("span", { className: "armada-turns__kind", children: turn.kind }),
    /* @__PURE__ */ jsxs("span", { className: "armada-turns__body", children: [
      turn.subject === void 0 && turn.detail === void 0 ? null : /* @__PURE__ */ jsxs("span", { className: "armada-turns__head", children: [
        turn.subject === void 0 ? null : /* @__PURE__ */ jsx("span", { className: "armada-turns__subject", children: turn.subject }),
        turn.detail === void 0 ? null : /* @__PURE__ */ jsxs("span", { className: "armada-turns__detail", children: [
          turn.detail,
          turn.truncated === true ? /* @__PURE__ */ jsx(Cut, {}) : null
        ] })
      ] }),
      turn.answer === void 0 ? null : /* @__PURE__ */ jsx("span", { className: "armada-turns__answer", children: turn.answer }),
      turn.said === void 0 ? null : /* @__PURE__ */ jsx("span", { className: "armada-turns__said", children: turn.said })
    ] })
  ] });
}
function Cut() {
  return /* @__PURE__ */ jsxs("span", { className: "armada-turns__cut", children: [
    "…",
    /* @__PURE__ */ jsx("span", { className: "armada-turns__cut-note", children: " cut short" })
  ] });
}
function rowId(turn) {
  return `armada-turn-${turn.id}`;
}
function counted(rows) {
  return rows === 1 ? "1 turn" : `${rows} turns`;
}
const meta$$ = {
  title: "Compositions/Drone turns",
  component: DroneTurns
};
const NOTHING_YET$1 = "This job has no turns. It was never dispatched, so no drone has written one.";
function thinking(from, rows, at) {
  return Array.from({ length: rows }, (_, n) => ({
    id: String(from + n),
    at,
    kind: "unrecognised",
    subject: n % 4 === 3 ? "a turn with nothing in it Armada names" : "system/thinking_tokens",
    quiet: true
  }));
}
const turns = [
  {
    id: "1",
    at: "09:14:02",
    kind: "started",
    // The model is whatever the Job named. A vendor spelling belongs in
    // `adapters` and nowhere else, so the fixture carries a placeholder.
    subject: "sess_01JB4 · the job's model · 2 mcp servers"
  },
  {
    id: "2",
    at: "09:14:03",
    kind: "said",
    said: "Reading the settings module before I split anything, so the public signature survives."
  },
  {
    id: "3",
    at: "09:14:04",
    kind: "called",
    subject: "Read",
    detail: "src/settings.rs",
    answer: "Answered."
  },
  {
    id: "4",
    at: "09:14:09",
    kind: "called",
    subject: "Bash",
    detail: "cargo test -p settings --lib",
    answer: "Answered, and the tool itself failed."
  },
  {
    id: "5",
    at: "09:14:10",
    kind: "called",
    subject: "TodoWrite · call_7f23",
    answer: "Answered."
  },
  {
    id: "6",
    at: "09:14:11",
    kind: "called",
    subject: "Edit",
    detail: "src/settings.rs +42 -18",
    answer: "No answer yet."
  }
];
const ADroneWorking = {
  args: { turns, emptyNote: NOTHING_YET$1 }
};
const AJobWithNoTranscript = {
  args: { turns: [], emptyNote: NOTHING_YET$1 }
};
const RefusedUnrecognisedAndUnreadable = {
  args: {
    emptyNote: NOTHING_YET$1,
    turns: [
      {
        id: "1",
        at: "09:15:40",
        kind: "refused",
        subject: "Bash · call_7f31",
        said: "This command is not on the allowlist for this drone."
      },
      { id: "2", at: "09:15:41", kind: "unrecognised", subject: "thinking_delta", quiet: true },
      {
        id: "3",
        at: "09:15:42",
        kind: "unreadable",
        subject: '{"type":"assistant","message":{"content":[{"type":"to',
        said: "The line ended mid-object."
      }
    ]
  }
};
const WhatEachCallDid = {
  args: {
    emptyNote: NOTHING_YET$1,
    turns: [
      {
        id: "1",
        at: "09:16:02",
        kind: "called",
        subject: "Read",
        detail: "src/settings.rs",
        answer: "Answered."
      },
      {
        id: "2",
        at: "09:16:04",
        kind: "called",
        subject: "Edit",
        detail: "reducer.rs +42 -18",
        answer: "Answered."
      },
      {
        id: "3",
        at: "09:16:09",
        kind: "called",
        subject: "Grep",
        detail: "fn observe\\( in crates/",
        answer: "Answered."
      },
      {
        id: "4",
        at: "09:16:20",
        kind: "called",
        subject: "Write",
        detail: "docs/practices/bridge.md, 412 lines starting # Bridge practices",
        truncated: true,
        answer: "No answer yet."
      }
    ]
  }
};
const ADroneThinking = {
  args: {
    live: true,
    emptyNote: NOTHING_YET$1,
    turns: [
      { id: "1", at: "09:14:02", kind: "said", said: "Starting on the settings split." },
      ...thinking(10, 9, "09:14:03"),
      { id: "20", at: "09:14:12", kind: "called", subject: "Read", detail: "src/settings.rs", answer: "Answered." },
      ...thinking(30, 14, "09:14:15"),
      { id: "50", at: "09:14:31", kind: "called", subject: "Edit", detail: "src/settings.rs +42 -18", answer: "Answered." },
      ...thinking(60, 6, "09:14:40")
    ]
  }
};
const AFinishedRun = {
  args: {
    live: false,
    emptyNote: NOTHING_YET$1,
    turns: [
      { id: "1", at: "09:14:02", kind: "said", said: "Starting on the settings split." },
      ...thinking(10, 9, "09:14:03"),
      { id: "20", at: "09:14:12", kind: "called", subject: "Read", detail: "src/settings.rs", answer: "Answered." },
      ...thinking(30, 14, "09:14:15"),
      { id: "50", at: "09:14:31", kind: "called", subject: "Edit", detail: "src/settings.rs +42 -18", answer: "Answered." },
      { id: "60", at: "09:14:44", kind: "said", said: "The public signature is unchanged. Submitting." },
      { id: "61", at: "09:14:45", kind: "ended", subject: "18 turns · ~$0.42 · no calls refused" }
    ]
  }
};
const WhatTheRunCost = {
  args: {
    live: false,
    emptyNote: NOTHING_YET$1,
    turns: [
      { id: "1", at: "09:14:02", kind: "said", said: "Starting on the settings split." },
      { id: "2", at: "09:55:10", kind: "ended", subject: "41 turns · ~$1.53 · 6 calls refused" },
      { id: "3", at: "10:02:11", kind: "said", said: "Retrying the step. Reading the refusal first." },
      { id: "4", at: "10:03:40", kind: "ended", subject: "4 turns · ~$0.0018 · no calls refused" }
    ]
  }
};
const NothingButToolCalls = {
  args: {
    live: true,
    emptyNote: NOTHING_YET$1,
    turns: [
      { id: "1", at: "09:20:01", kind: "called", subject: "Bash", detail: "cargo xtask verify-foundations", answer: "Answered." },
      { id: "2", at: "09:20:31", kind: "called", subject: "Bash", detail: "cargo test -p ipc", answer: "Answered, and the tool itself failed." },
      { id: "3", at: "09:20:48", kind: "called", subject: "Read", detail: "crates/ipc/src/turn.rs", answer: "Answered." },
      { id: "4", at: "09:20:52", kind: "called", subject: "Grep", detail: "Saw::Called in crates/", answer: "No answer yet." }
    ]
  }
};
const REPRO = { id: "repro", label: "Reproduce the bug" };
const FIX = { id: "fix", label: "Fix the root cause" };
const TurnsUnderTheirSteps = {
  args: {
    live: true,
    emptyNote: NOTHING_YET$1,
    turns: [
      { id: "1", at: "09:14:02", step: REPRO, kind: "started", subject: "sess_01JB4 · the job's model · 2 mcp servers" },
      { id: "2", at: "09:14:03", step: REPRO, kind: "said", said: "Writing the failing test before I touch the reducer." },
      ...thinking(10, 5, "09:14:04").map((turn) => ({ ...turn, step: REPRO })),
      { id: "20", at: "09:14:22", step: REPRO, kind: "called", subject: "Write", detail: "tests/settings_split.rs", answer: "Answered." },
      { id: "21", at: "09:15:01", step: FIX, kind: "said", said: "The test reproduces it. Splitting the reducer now." },
      { id: "22", at: "09:15:09", step: FIX, kind: "called", subject: "Edit", detail: "src/settings.rs +42 -18", answer: "Answered." },
      { id: "23", at: "09:15:40", step: FIX, kind: "called", subject: "Bash", detail: "cargo test -p settings --lib", answer: "Answered, and the tool itself failed." },
      { id: "24", at: "09:18:02", step: REPRO, kind: "said", said: "The gate sent this back. Widening the reproduction first." },
      { id: "25", at: "09:18:30", step: REPRO, kind: "called", subject: "Edit", detail: "tests/settings_split.rs +11 -0", answer: "Answered." },
      { id: "26", at: "09:19:04", step: FIX, kind: "called", subject: "Edit", detail: "src/settings.rs +6 -2", answer: "No answer yet." }
    ]
  }
};
const AStepWithNoNameOfItsOwn = {
  args: {
    emptyNote: NOTHING_YET$1,
    turns: [
      { id: "1", at: "09:22:01", step: { id: "implement", label: "implement", labelIsAnIdentifier: true }, kind: "called", subject: "Edit", detail: "src/settings.rs +42 -18", answer: "Answered." },
      { id: "2", at: "09:24:40", step: { id: "regression_verify", label: "regression_verify", labelIsAnIdentifier: true }, kind: "called", subject: "Bash", detail: "cargo nextest run --workspace", answer: "Answered." },
      { id: "3", at: "09:26:12", step: { id: "write_up", label: "write_up", labelIsAnIdentifier: true }, kind: "said", said: "Submitting the evidence report." }
    ]
  }
};
const RowsWrittenBeforeTheStepWasRecorded = {
  args: {
    emptyNote: NOTHING_YET$1,
    turns: [
      { id: "1", at: "08:59:14", kind: "started", subject: "sess_01J9Z · the job's model · 2 mcp servers" },
      { id: "2", at: "08:59:20", kind: "called", subject: "Read", detail: "src/settings.rs", answer: "Answered." },
      { id: "3", at: "09:01:02", kind: "said", said: "Reading the reducer before I split it." },
      { id: "4", at: "09:12:41", step: FIX, kind: "called", subject: "Edit", detail: "src/settings.rs +42 -18", answer: "Answered." },
      { id: "5", at: "09:13:10", step: FIX, kind: "called", subject: "Bash", detail: "cargo test -p settings --lib", answer: "Answered." }
    ]
  }
};
const __vite_glob_0_8 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  ADroneThinking,
  ADroneWorking,
  AFinishedRun,
  AJobWithNoTranscript,
  AStepWithNoNameOfItsOwn,
  NothingButToolCalls,
  RefusedUnrecognisedAndUnreadable,
  RowsWrittenBeforeTheStepWasRecorded,
  TurnsUnderTheirSteps,
  WhatEachCallDid,
  WhatTheRunCost,
  default: meta$$
}, Symbol.toStringTag, { value: "Module" }));
const CARD_ICON = 12;
const CARD_STROKE = 2;
function EvidenceCard({
  icon: Icon,
  iconLabel,
  step,
  time,
  claimed,
  shownBy,
  notClaimed,
  emptyNotClaimed = "Nothing"
}) {
  return /* @__PURE__ */ jsxs("div", { className: "armada-evidence-card", children: [
    /* @__PURE__ */ jsxs("div", { className: "armada-evidence-card__head", children: [
      /* @__PURE__ */ jsxs("span", { className: "armada-evidence-card__mark", children: [
        Icon ? /* @__PURE__ */ jsx(Icon, { size: CARD_ICON, strokeWidth: CARD_STROKE, "aria-hidden": true }) : null,
        iconLabel ? /* @__PURE__ */ jsx("span", { className: "armada-evidence-card__sr", children: iconLabel }) : null
      ] }),
      /* @__PURE__ */ jsx("span", { className: "armada-evidence-card__step", children: step }),
      time ? /* @__PURE__ */ jsx("span", { className: "armada-evidence-card__time", children: time }) : null
    ] }),
    /* @__PURE__ */ jsxs("div", { className: "armada-evidence-card__field", children: [
      /* @__PURE__ */ jsx("span", { className: "armada-evidence-card__label", children: "Claimed" }),
      /* @__PURE__ */ jsx("span", { className: "armada-evidence-card__value", children: claimed })
    ] }),
    /* @__PURE__ */ jsxs("div", { className: "armada-evidence-card__field", children: [
      /* @__PURE__ */ jsx("span", { className: "armada-evidence-card__label", children: "Shown by" }),
      /* @__PURE__ */ jsx("span", { className: "armada-evidence-card__value", "data-mono": true, children: shownBy })
    ] }),
    /* @__PURE__ */ jsxs("div", { className: "armada-evidence-card__field", children: [
      /* @__PURE__ */ jsx("span", { className: "armada-evidence-card__label", children: "Not claimed" }),
      /* @__PURE__ */ jsx("span", { className: "armada-evidence-card__value", children: notClaimed ? notClaimed : emptyNotClaimed })
    ] })
  ] });
}
const meta$_ = {
  title: "Compositions/Evidence card",
  component: EvidenceCard
};
const NO_GLYPH_IN_REGISTRY$2 = void 0;
const PlanTheChange = {
  args: {
    icon: NO_GLYPH_IN_REGISTRY$2,
    iconLabel: "Evidence",
    step: "Plan the change",
    time: "09:14",
    claimed: "settings.rs is split into a reducer and a selector module, with no change in behaviour.",
    shownBy: "src/settings.rs → src/settings/reducer.rs, src/settings/selectors.rs",
    notClaimed: "Nothing about the settings UI, and no new tests — the existing suite is the only cover."
  }
};
const AnArtifactThatIsACommand = {
  args: {
    icon: NO_GLYPH_IN_REGISTRY$2,
    iconLabel: "Evidence",
    step: "Run tests",
    time: "14:16",
    claimed: "The ceiling holds at 3 and the counter increments once per poke.",
    shownBy: "cargo test --workspace · 86 passed 0 failed 5.1s · lease::poke_ceiling_holds",
    notClaimed: "No test covers a drone that answers on the third poke. The suite was green before this change and is green after."
  }
};
const NotClaimedEmpty$1 = {
  args: {
    icon: NO_GLYPH_IN_REGISTRY$2,
    iconLabel: "Evidence",
    step: "Summarise",
    time: "14:20",
    claimed: "The change is on fix/poke-ceiling and ready to read.",
    shownBy: "3 files +214 −96 · branch fix/poke-ceiling"
  }
};
const __vite_glob_0_9 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AnArtifactThatIsACommand,
  NotClaimedEmpty: NotClaimedEmpty$1,
  PlanTheChange,
  default: meta$_
}, Symbol.toStringTag, { value: "Module" }));
const ENTRY_ICON = 12;
const ENTRY_STROKE = 2;
function EvidenceTrail({ entries: entries2, emptyNotClaimed = "Nothing" }) {
  return /* @__PURE__ */ jsx("ol", { className: "armada-evidence-trail", children: entries2.map((entry, i) => /* @__PURE__ */ jsxs("li", { className: "armada-evidence-trail__entry", children: [
    /* @__PURE__ */ jsxs("span", { className: "armada-evidence-trail__mark", children: [
      entry.icon ? /* @__PURE__ */ jsx(entry.icon, { size: ENTRY_ICON, strokeWidth: ENTRY_STROKE, "aria-hidden": true }) : null,
      entry.iconLabel ? /* @__PURE__ */ jsx("span", { className: "armada-evidence-trail__sr", children: entry.iconLabel }) : null
    ] }),
    /* @__PURE__ */ jsxs("div", { className: "armada-evidence-trail__body", children: [
      /* @__PURE__ */ jsxs("div", { className: "armada-evidence-trail__head", children: [
        /* @__PURE__ */ jsx("span", { className: "armada-evidence-trail__step", children: entry.step }),
        entry.provenance ? /* @__PURE__ */ jsx("span", { className: "armada-evidence-trail__provenance", children: entry.provenance }) : null
      ] }),
      /* @__PURE__ */ jsxs("div", { className: "armada-evidence-trail__field", children: [
        /* @__PURE__ */ jsx("span", { className: "armada-evidence-trail__label", children: "Claimed" }),
        /* @__PURE__ */ jsx("span", { className: "armada-evidence-trail__value", children: entry.claimed })
      ] }),
      /* @__PURE__ */ jsxs("div", { className: "armada-evidence-trail__field", children: [
        /* @__PURE__ */ jsx("span", { className: "armada-evidence-trail__label", children: "Shown by" }),
        /* @__PURE__ */ jsx("span", { className: "armada-evidence-trail__value", "data-mono": true, children: entry.shownBy })
      ] }),
      /* @__PURE__ */ jsxs("div", { className: "armada-evidence-trail__field", children: [
        /* @__PURE__ */ jsx("span", { className: "armada-evidence-trail__label", children: "Not claimed" }),
        /* @__PURE__ */ jsx("span", { className: "armada-evidence-trail__value", children: entry.notClaimed ? entry.notClaimed : emptyNotClaimed })
      ] })
    ] })
  ] }, i)) });
}
const meta$Z = {
  title: "Compositions/Evidence trail",
  component: EvidenceTrail
};
const NO_GLYPH_IN_REGISTRY$1 = void 0;
const AFinishedJob$1 = {
  args: {
    entries: [
      {
        icon: NO_GLYPH_IN_REGISTRY$1,
        iconLabel: "Evidence",
        step: "Plan the change",
        provenance: "14:02 · facts_note · no check",
        claimed: "The poke loop stops after 3 attempts and the job records how many it spent.",
        shownBy: "core/fleet/src/lease.rs · the loop has no ceiling today",
        notClaimed: "Does not change the poke interval, and does not decide what happens at the third failure."
      },
      {
        icon: NO_GLYPH_IN_REGISTRY$1,
        iconLabel: "Evidence",
        step: "Implement",
        provenance: "14:11 · diff · build exit 0 · diff_nonempty passed",
        claimed: "A drone that stops answering is poked at most 3 times, and the count is on the job record.",
        shownBy: "core/fleet/src/lease.rs +38 −7 · core/model/src/job.rs +14 −0",
        notClaimed: "The count is not surfaced in Bridge. Nothing acts on reaching the ceiling yet — the loop exits and the job keeps its status."
      },
      {
        icon: NO_GLYPH_IN_REGISTRY$1,
        iconLabel: "Evidence",
        step: "Run tests",
        provenance: "14:16 · test_suite_run · test exit 0",
        claimed: "The ceiling holds at 3 and the counter increments once per poke.",
        shownBy: "cargo test --workspace · 86 passed 0 failed 5.1s · lease::poke_ceiling_holds, lease::poke_count_increments",
        notClaimed: "No test covers a drone that answers on the third poke. The suite was green before this change and is green after, so it does not prove the ceiling is reached in practice."
      },
      {
        icon: NO_GLYPH_IN_REGISTRY$1,
        iconLabel: "Evidence",
        step: "Summarise",
        provenance: "14:20 · facts_note · no check",
        claimed: "The change is on fix/poke-ceiling and ready to read.",
        shownBy: "3 files +214 −96 · branch fix/poke-ceiling",
        notClaimed: "The value 3 is a constant rather than config. Whether it is the right number is not established by anything here."
      }
    ]
  }
};
const NotClaimedEmpty = {
  args: {
    entries: [
      {
        icon: NO_GLYPH_IN_REGISTRY$1,
        iconLabel: "Evidence",
        step: "Run tests",
        provenance: "14:16 · test_suite_run · test exit 0",
        claimed: "The ceiling holds at 3 and the counter increments once per poke.",
        shownBy: "cargo test --workspace · 86 passed 0 failed 5.1s"
      }
    ]
  }
};
const OneEntry = {
  args: {
    entries: [
      {
        icon: NO_GLYPH_IN_REGISTRY$1,
        iconLabel: "Evidence",
        step: "Plan the change",
        provenance: "09:14 · facts_note · no check",
        claimed: "settings.rs is split into a reducer and a selector module, with no change in behaviour.",
        shownBy: "src/settings.rs → src/settings/reducer.rs, src/settings/selectors.rs",
        notClaimed: "Nothing about the settings UI, and no new tests — the existing suite is the only cover."
      }
    ]
  }
};
const __vite_glob_0_10 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AFinishedJob: AFinishedJob$1,
  NotClaimedEmpty,
  OneEntry,
  default: meta$Z
}, Symbol.toStringTag, { value: "Module" }));
function FactChip({ children, named: named2, title }) {
  return /* @__PURE__ */ jsx("span", { className: "armada-chip", "data-named": named2, title, children });
}
const meta$Y = {
  title: "Compositions/Fact chip",
  component: FactChip
};
const AShortValue = {
  args: { children: "not run" }
};
const TheFactsOnARunningStep = {
  render: () => /* @__PURE__ */ jsxs("div", { style: { display: "flex", gap: "var(--space-2)", flexWrap: "wrap" }, children: [
    /* @__PURE__ */ jsx(FactChip, { children: "3 files · +94 −31" }),
    /* @__PURE__ */ jsx(FactChip, { children: "not run" }),
    /* @__PURE__ */ jsx(FactChip, { children: "2 criteria" }),
    /* @__PURE__ */ jsx(FactChip, { children: "test" }),
    /* @__PURE__ */ jsx(FactChip, { children: "as it was at 14:20" })
  ] })
};
const AVerdictPerFact = {
  render: () => /* @__PURE__ */ jsxs("div", { style: { display: "flex", gap: "var(--space-2)", flexWrap: "wrap" }, children: [
    /* @__PURE__ */ jsx(FactChip, { named: "refused", children: "refused · reducer changed" }),
    /* @__PURE__ */ jsx(FactChip, { named: "advanced", children: "advanced" }),
    /* @__PURE__ */ jsx(FactChip, { named: "passed", children: "2 of 2 passed" }),
    /* @__PURE__ */ jsx(FactChip, { named: "not_met", children: "1 of 2 refused" }),
    /* @__PURE__ */ jsx(FactChip, { named: "waiting", children: "on you · 2m 04s" })
  ] })
};
const WiderThanItsColumn = {
  render: () => /* @__PURE__ */ jsx("div", { style: { width: "calc(var(--space-12) * 3)" }, children: /* @__PURE__ */ jsx(FactChip, { title: "3 files · +94 −31 · all inside the plan", children: "3 files · +94 −31 · all inside the plan" }) })
};
const __vite_glob_0_11 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AShortValue,
  AVerdictPerFact,
  TheFactsOnARunningStep,
  WiderThanItsColumn,
  default: meta$Y
}, Symbol.toStringTag, { value: "Module" }));
function ErrorCode({ kind, code }) {
  return /* @__PURE__ */ jsx("span", { className: `armada-error-code armada-error-code--${kind}`, "data-error-class": kind, children: code });
}
const ROW_ICON$1 = 12;
const ROW_STROKE$1 = 2;
const ACTION_ICON = 16;
function halves(value) {
  const end = value.endsWith("/") ? value.length - 1 : value.length;
  const cut = value.lastIndexOf("/", end - 1);
  return cut <= 0 ? [value, ""] : [value.slice(0, cut + 1), value.slice(cut + 1)];
}
function JobLogReference({ rows, children, actions, onCopied }) {
  const [unopened, setUnopened] = useState(null);
  const [opening, setOpening] = useState(null);
  const copy = useCallback(
    (event, value) => {
      event.stopPropagation();
      void navigator.clipboard.writeText(value).then(
        () => onCopied?.(value),
        // A failed clipboard write is otherwise indistinguishable from a dead
        // element, so the surface is told either way.
        () => onCopied?.(value)
      );
    },
    [onCopied]
  );
  const open2 = useCallback((row, go) => {
    setOpening(row);
    setUnopened(null);
    void go().then((why) => {
      if (why !== null) setUnopened({ row, because: why.because });
    }).finally(() => setOpening(null));
  }, []);
  return /* @__PURE__ */ jsxs("div", { className: "armada-log-ref", children: [
    rows.map((row, i) => {
      const [head, tail] = halves(row.value);
      const failed2 = unopened !== null && unopened.row === i ? unopened.because : null;
      const opens = row.open;
      return /* @__PURE__ */ jsxs(Fragment$1, { children: [
        /* @__PURE__ */ jsxs("div", { className: "armada-log-ref__row", "data-separated": row.separated || void 0, children: [
          /* @__PURE__ */ jsxs("span", { className: "armada-log-ref__mark", children: [
            row.icon ? /* @__PURE__ */ jsx(row.icon, { size: ROW_ICON$1, strokeWidth: ROW_STROKE$1, "aria-hidden": true }) : null,
            row.iconLabel ? /* @__PURE__ */ jsx("span", { className: "armada-log-ref__sr", children: row.iconLabel }) : null
          ] }),
          /* @__PURE__ */ jsxs(
            "span",
            {
              className: "armada-log-ref__value",
              title: row.value,
              "data-copies": row.copyValue !== void 0 || void 0,
              onClick: row.copyValue !== void 0 ? (e) => copy(e, row.copyValue) : void 0,
              children: [
                /* @__PURE__ */ jsx("span", { className: "armada-log-ref__head", children: head }),
                tail === "" ? null : /* @__PURE__ */ jsx("span", { className: "armada-log-ref__tail", children: tail })
              ]
            }
          ),
          row.meta ? /* @__PURE__ */ jsx("span", { className: "armada-log-ref__meta", children: row.meta }) : null,
          opens === void 0 ? null : /* @__PURE__ */ jsx(
            Button,
            {
              size: "sm",
              variant: "ghost",
              iconOnly: true,
              "aria-label": opens.label,
              disabled: opening !== null,
              onClick: () => open2(i, opens.go),
              children: /* @__PURE__ */ jsx(ExternalLink, { size: ACTION_ICON, strokeWidth: ROW_STROKE$1, "aria-hidden": true })
            }
          )
        ] }),
        failed2 === null ? null : /* @__PURE__ */ jsx("p", { className: "armada-log-ref__unopened", role: "status", children: failed2 })
      ] }, i);
    }),
    children || actions ? /* @__PURE__ */ jsxs("div", { className: "armada-log-ref__foot", children: [
      children ? /* @__PURE__ */ jsx("p", { className: "armada-log-ref__note", children }) : null,
      actions ? /* @__PURE__ */ jsx("div", { className: "armada-log-ref__actions", children: actions }) : null
    ] }) : null
  ] });
}
const FOLD_ICON$1 = 16;
const FOLD_STROKE$1 = 2;
function FailureNotice({
  headline,
  code,
  region,
  next,
  details,
  detailsLabel = "Details",
  values,
  note,
  actions,
  onCopied
}) {
  const [open2, setOpen] = useState(false);
  const folded = details !== void 0 && details.length > 0;
  const referenced = values !== void 0 && values.length > 0 || note !== void 0 || actions !== void 0;
  return /* @__PURE__ */ jsxs("section", { className: "armada-failure", role: "alert", children: [
    /* @__PURE__ */ jsxs("div", { className: "armada-failure__copy", children: [
      /* @__PURE__ */ jsx("span", { className: "armada-failure__headline", children: headline }),
      /* @__PURE__ */ jsx("span", { className: "armada-failure__next", children: next })
    ] }),
    code === void 0 && region === void 0 ? null : /* @__PURE__ */ jsx("div", { className: "armada-failure__code", children: /* @__PURE__ */ jsx(ErrorCode, { kind: "fault", code: code ?? region }) }),
    folded ? /* @__PURE__ */ jsxs(
      "details",
      {
        className: "armada-failure__details",
        onToggle: (event) => setOpen(event.currentTarget.open),
        children: [
          /* @__PURE__ */ jsxs("summary", { className: "armada-failure__summary", children: [
            open2 ? /* @__PURE__ */ jsx(ChevronDown, { size: FOLD_ICON$1, strokeWidth: FOLD_STROKE$1, "aria-hidden": true }) : /* @__PURE__ */ jsx(ChevronRight, { size: FOLD_ICON$1, strokeWidth: FOLD_STROKE$1, "aria-hidden": true }),
            detailsLabel
          ] }),
          /* @__PURE__ */ jsx("dl", { className: "armada-failure__list", children: details.map((detail, at) => /* @__PURE__ */ jsxs("div", { className: "armada-failure__pair", children: [
            /* @__PURE__ */ jsx("dt", { className: "armada-failure__label", children: detail.label }),
            /* @__PURE__ */ jsx("dd", { className: "armada-failure__value", children: detail.value })
          ] }, at)) })
        ]
      }
    ) : null,
    referenced ? /* @__PURE__ */ jsx(JobLogReference, { rows: values ?? [], actions, onCopied, children: note }) : null
  ] });
}
const meta$X = {
  title: "Compositions/Failure notice",
  component: FailureNotice
};
const AUDIT = "/Users/user/Library/Application Support/Armada/audit.jsonl";
const RUNTIME_FILE = "/Users/user/Library/Application Support/Armada/fleet.json";
const FLEET_RUN = "01M0ZTNSVD0004FY52G3PP82SJ";
function Acts({ reload = true, dismiss = false }) {
  return /* @__PURE__ */ jsxs(Fragment, { children: [
    reload ? /* @__PURE__ */ jsx(Button, { variant: "ghost", size: "sm", ground: "sunken", children: "Reload Bridge" }) : null,
    /* @__PURE__ */ jsx(Button, { variant: "ghost", size: "sm", ground: "sunken", children: "Copy report" }),
    dismiss ? /* @__PURE__ */ jsx(Button, { variant: "ghost", size: "sm", ground: "sunken", children: "Dismiss" }) : null
  ] });
}
const FleetIsNotRunningNoFile = {
  args: {
    headline: "Fleet is not running",
    next: "Start Fleet. Bridge reconnects on its own.",
    detailsLabel: "What the runtime file answered",
    details: [
      { label: "Answer", value: "no_runtime_file" },
      { label: "Runtime file", value: RUNTIME_FILE }
    ],
    values: [
      { icon: File, iconLabel: "Log", value: AUDIT, copyValue: AUDIT }
    ],
    note: "Bridge rereads the file every 2 seconds and connects when one appears. Nothing reached Fleet, so there is no run id to quote.",
    actions: /* @__PURE__ */ jsx(Acts, {})
  }
};
const FleetIsNotRunningDeadPid = {
  args: {
    headline: "Fleet is not running",
    next: "Start Fleet. Bridge reconnects on its own.",
    detailsLabel: "What the runtime file answered",
    details: [
      { label: "Answer", value: "pid_dead" },
      { label: "Pid", value: "48221" },
      { label: "Runtime file", value: RUNTIME_FILE }
    ],
    values: [
      { icon: File, iconLabel: "Log", value: AUDIT, copyValue: AUDIT }
    ],
    note: "Fleet exited without cleaning up. The file names a pid nothing holds.",
    actions: /* @__PURE__ */ jsx(Acts, {})
  }
};
const FleetIsNotRunningPidReused = {
  args: {
    headline: "Fleet is not running",
    next: "Start Fleet. Bridge reconnects on its own.",
    detailsLabel: "What the runtime file answered",
    details: [
      { label: "Answer", value: "pid_held_by_another" },
      { label: "Pid", value: "48221" },
      { label: "File wrote", value: "Sat Aug 23 09:14:02 2026" },
      { label: "Holder started", value: "Sun Aug 24 18:02:55 2026" },
      { label: "Runtime file", value: RUNTIME_FILE }
    ],
    values: [
      { icon: File, iconLabel: "Log", value: AUDIT, copyValue: AUDIT }
    ],
    note: "Bridge did not open a socket. The port in this file is not Fleet's.",
    actions: /* @__PURE__ */ jsx(Acts, {})
  }
};
const FleetIsUnreachable = {
  args: {
    headline: "Fleet unreachable",
    next: "Fleet is up and not answering. What is on the board is not live.",
    detailsLabel: "What the connection answered",
    details: [
      { label: "Pid", value: "48221" },
      { label: "Port", value: "7773" },
      { label: "Silent for", value: "1m" },
      { label: "Detail", value: "the connection closed" },
      { label: "Last read", value: "1m ago" }
    ],
    values: [
      { icon: File, iconLabel: "Log", value: AUDIT, copyValue: AUDIT }
    ],
    note: "Bridge is retrying every 2 seconds. Jobs keep progressing either way.",
    actions: /* @__PURE__ */ jsx(Acts, {})
  }
};
const TheRendererThrew = {
  args: {
    headline: "Bridge could not draw the job list",
    next: "Reload Bridge. Fleet keeps running and jobs keep progressing.",
    detailsLabel: "What threw",
    details: [
      { label: "Component", value: "JobRowStacked" },
      { label: "Message", value: "Cannot read properties of undefined (reading 'steps')" },
      {
        label: "Where",
        value: "    at JobRowStacked\n    at Row\n    at Jobs\n    at Boundary\n    at App"
      },
      {
        label: "Stack",
        value: "TypeError: Cannot read properties of undefined (reading 'steps')\n    at Row (Jobs.tsx:214:29)\n    at renderWithHooks (react-dom-client.js:5529:24)"
      }
    ],
    values: [
      { icon: File, iconLabel: "Log", value: AUDIT, copyValue: AUDIT }
    ],
    note: "The rest of the window is still usable. Only this region stopped drawing. This never reached Fleet, so there is no run id: the component and the log identify it.",
    actions: /* @__PURE__ */ jsx(Acts, {})
  }
};
const AJobCannotBeRead = {
  args: {
    headline: "Job 01K2Y0X6R4B7QW9V3N5T8CJ1MF did not load",
    next: "Every other job on the board is unaffected. Read the fault, or read the job's log.",
    detailsLabel: "What the store refused",
    details: [
      { label: "Job", value: "01K2Y0X6R4B7QW9V3N5T8CJ1MF" },
      {
        label: "Fault",
        value: "row 14: status `awaiting_attestation` has no attestation, which the model requires"
      }
    ],
    values: [
      {
        icon: File,
        iconLabel: "Log",
        value: ".armada/logs/01K2Y0X6R4B7QW9V3N5T8CJ1MF.jsonl",
        copyValue: ".armada/logs/01K2Y0X6R4B7QW9V3N5T8CJ1MF.jsonl"
      },
      { icon: File, iconLabel: "Log", value: AUDIT, copyValue: AUDIT, separated: true }
    ],
    note: "The log path is relative to the job's repository. Fleet does not send which one, and it does not send a run id for the read that refused this row.",
    actions: /* @__PURE__ */ jsx(Acts, {})
  }
};
const FleetRefusedTheCommand = {
  args: {
    headline: "Manifest 01K1M8Z5V2 is not one Fleet holds",
    next: "Nothing was sent. Change what the command names, or read the log.",
    detailsLabel: "What Fleet refused",
    details: [
      { label: "Code", value: "ARM-0412" },
      { label: "Message", value: "Manifest 01K1M8Z5V2 is not one Fleet holds" },
      { label: "manifest_id", value: "01K1M8Z5V2QW7H3N9TB4XC6RFD" },
      { label: "held", value: "3" },
      {
        label: "Chain",
        value: "propose_job\nresolve_manifest\nManifestNotHeld"
      }
    ],
    values: [
      { icon: File, iconLabel: "Log", value: AUDIT, copyValue: AUDIT },
      { value: FLEET_RUN, copyValue: FLEET_RUN, meta: "Fleet run" }
    ],
    note: "The run names Fleet's process for this session, not this one failure. It is what joins this to Fleet's log lines.",
    actions: /* @__PURE__ */ jsx(Acts, { reload: false, dismiss: true })
  }
};
const NothingButTheSentence = {
  args: {
    headline: "Bridge could not reach the main process",
    next: "Reload Bridge. If it happens again, quit and reopen."
  }
};
const AFaultWithItsCode = {
  args: {
    headline: "The job could not be read",
    code: "job_unreadable",
    next: "Open another job. This one stays on the board, greyed.",
    detailsLabel: "What the store answered",
    details: [{ label: "Job", value: "job_2d90bb" }],
    values: [{ icon: File, iconLabel: "Log", value: AUDIT, copyValue: AUDIT }],
    note: "The rest of the board is unaffected. One row failed to read, not the list.",
    actions: /* @__PURE__ */ jsx(Acts, { reload: false, dismiss: true })
  }
};
const ABoundaryFallbackWithNoCode = {
  args: {
    headline: "This part of Bridge stopped drawing",
    region: "Job detail",
    next: "Reload Bridge. Jobs keep running with the window closed.",
    detailsLabel: "What threw",
    details: [
      { label: "Component", value: "StepPanel" },
      { label: "Message", value: "Cannot read properties of undefined (reading 'judged')" }
    ],
    values: [{ icon: File, iconLabel: "Log", value: AUDIT, copyValue: AUDIT }],
    note: "Fleet is untouched. This is the window, not the daemon.",
    actions: /* @__PURE__ */ jsx(Acts, {})
  }
};
const __vite_glob_0_12 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  ABoundaryFallbackWithNoCode,
  AFaultWithItsCode,
  AJobCannotBeRead,
  FleetIsNotRunningDeadPid,
  FleetIsNotRunningNoFile,
  FleetIsNotRunningPidReused,
  FleetIsUnreachable,
  FleetRefusedTheCommand,
  NothingButTheSentence,
  TheRendererThrew,
  default: meta$X
}, Symbol.toStringTag, { value: "Module" }));
function ScrollArea({
  className,
  maxHeight,
  axis = "vertical",
  style,
  ...rest
}) {
  return /* @__PURE__ */ jsx(
    "div",
    {
      className: className ? `armada-scroll-area ${className}` : "armada-scroll-area",
      "data-axis": axis,
      style: maxHeight ? { ...style, maxHeight } : style,
      ...rest
    }
  );
}
function Dialog({
  open: open2,
  title,
  children,
  field,
  tone = "destructive",
  width = "default",
  confirmLabel,
  confirmDisabled = false,
  cancelLabel = "Cancel",
  onConfirm,
  onCancel
}) {
  const cancelRef = useRef(null);
  useEffect(() => {
    if (open2) cancelRef.current?.focus();
  }, [open2]);
  useEffect(() => {
    if (!open2) return;
    function onKey(event) {
      if (event.key === "Escape") {
        event.preventDefault();
        onCancel?.();
      }
      if (event.key === "Enter") {
        event.preventDefault();
        if (!confirmDisabled) onConfirm?.();
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open2, onCancel, onConfirm, confirmDisabled]);
  if (!open2) return null;
  const destructive = tone === "destructive";
  return /* @__PURE__ */ jsx("div", { className: "armada-dialog-scrim", children: /* @__PURE__ */ jsxs(
    "div",
    {
      className: "armada-dialog",
      "data-width": width,
      role: "dialog",
      "aria-modal": "true",
      "aria-label": title,
      children: [
        /* @__PURE__ */ jsxs("div", { className: "armada-dialog__head", children: [
          destructive ? /* @__PURE__ */ jsx(
            X,
            {
              className: "armada-dialog__glyph armada-dialog__glyph--destructive",
              size: 16,
              strokeWidth: 2,
              "aria-hidden": "true"
            }
          ) : /* @__PURE__ */ jsx(
            TriangleAlert,
            {
              className: "armada-dialog__glyph armada-dialog__glyph--escalated",
              size: 16,
              strokeWidth: 2,
              "aria-hidden": "true"
            }
          ),
          /* @__PURE__ */ jsx("h2", { className: "armada-dialog__title", children: title })
        ] }),
        /* @__PURE__ */ jsx(ScrollArea, { className: "armada-dialog__body", children }),
        field === void 0 ? null : /* @__PURE__ */ jsx("div", { className: "armada-dialog__field", children: field }),
        /* @__PURE__ */ jsxs("div", { className: "armada-dialog__actions", children: [
          /* @__PURE__ */ jsx(
            "button",
            {
              ref: cancelRef,
              type: "button",
              className: "armada-dialog__button armada-dialog__button--secondary",
              onClick: onCancel,
              children: cancelLabel
            }
          ),
          /* @__PURE__ */ jsx(
            "button",
            {
              type: "button",
              className: destructive ? "armada-dialog__button armada-dialog__button--destructive" : "armada-dialog__button armada-dialog__button--primary",
              disabled: confirmDisabled,
              onClick: onConfirm,
              children: confirmLabel
            }
          )
        ] })
      ]
    }
  ) });
}
function Textarea({
  label: label2,
  invalid = false,
  message,
  rows = 3,
  id,
  ...rest
}) {
  const generated = useId();
  const textareaId = id ?? generated;
  const messageId = `${textareaId}-message`;
  const showMessage = invalid && message !== void 0;
  return /* @__PURE__ */ jsxs("div", { className: "armada-textarea-field", children: [
    label2 !== void 0 && /* @__PURE__ */ jsx("label", { className: "armada-textarea-field__label", htmlFor: textareaId, children: label2 }),
    /* @__PURE__ */ jsx(
      "textarea",
      {
        ...rest,
        rows,
        id: textareaId,
        className: "armada-textarea",
        "aria-invalid": invalid || void 0,
        "aria-describedby": showMessage ? messageId : void 0
      }
    ),
    showMessage && /* @__PURE__ */ jsx("span", { className: "armada-textarea-field__message", id: messageId, children: message })
  ] });
}
const meta$W = {
  title: "Compositions/Gaming flags",
  component: GamingFlags
};
const OnTheRail = {
  args: {
    citation: "clipped",
    said: "the gaming check flagged this evidence",
    flags: [
      { pattern: "check_config_edited", cited: "armada.yml — checks.tests.command" },
      {
        pattern: "assertion_weakened",
        cited: "crates/api/src/tests/served.rs:214 — served_every_operation counts the filtered set"
      }
    ]
  }
};
const TwoFlagsReadInFull = {
  args: {
    citation: "whole",
    said: "What it flagged",
    flags: [
      {
        pattern: "check_config_edited",
        cited: "`armada.yml` — the `checks.tests.command` key was changed in the same step whose evidence it gates. The command it was changed **to** exits 0 on an empty test set:\n\n```\ncargo nextest run --workspace -E 'test(served_every_operation)'\n```\n\nThe previous command ran the whole workspace, so the step's evidence is a green run of a narrower set than the one the criterion names."
      },
      {
        pattern: "assertion_weakened",
        cited: '`crates/api/src/tests/served.rs:214` — `served_every_operation` walks the route table once per operation and the loop skips `forget_job`:\n\n```\nlet routes: Vec<&Route> = ROUTES.iter().filter(|r| r.operation != "forget_job").collect();\nassert_eq!(routes.len(), served.len());\n```\n\nThe count the assertion compares against was lowered by the same edit that made it pass, so the one operation the step was about is the one operation never read.'
      }
    ]
  }
};
const FlaggedAndNotCited = {
  args: {
    citation: "whole",
    flags: [{ pattern: "evidence_reused" }]
  }
};
const WhatItMeansAndWhereItIs = {
  args: {
    citation: "clipped",
    said: "the gaming check flagged this evidence",
    onOpenAt: () => {
    },
    flags: [
      {
        pattern: "assertion_weakened",
        verb: "an assertion now asserts less",
        cited: "served_every_operation counts the filtered set",
        at: { file: "crates/api/src/tests/served.rs", line: 214 }
      },
      {
        pattern: "test_deleted",
        verb: "a test file was removed whole",
        cited: "the whole of served_every_operation is gone",
        at: { file: "crates/api/src/tests/served.rs" }
      },
      {
        pattern: "no_findings_on_substantial_diff",
        verb: "a review that found nothing in a substantial change",
        cited: "1,204 lines across 18 files, and the review answered clean"
      }
    ]
  }
};
const APatternWithNoVerb = {
  args: {
    citation: "clipped",
    flags: [{ pattern: "evidence_reused", cited: "the same run is cited on two steps" }]
  }
};
const WhereWithNowhereToGo = {
  args: {
    citation: "whole",
    said: "What it flagged",
    flags: [
      {
        pattern: "tautological_test",
        verb: "a test that passes whatever the code does",
        cited: "`crates/api/src/tests/served.rs:214` — the assertion compares a value against itself, so it holds for every possible implementation.",
        at: { file: "crates/api/src/tests/served.rs", line: 214 }
      }
    ]
  }
};
const OverrulingTwoFlags = {
  args: TwoFlagsReadInFull.args,
  render: (args) => /* @__PURE__ */ jsxs(
    Dialog,
    {
      open: true,
      tone: "neutral",
      width: "wide",
      title: "Overrule the gaming flag on this step?",
      confirmLabel: "Overrule the flag",
      confirmDisabled: true,
      field: /* @__PURE__ */ jsx(Textarea, { label: "Why the flag is wrong", rows: 3 }),
      children: [
        /* @__PURE__ */ jsx("p", { children: "The gaming check flagged the evidence for Regression check. It did not refuse the work — it says the evidence for it is not to be trusted. Overruling says a person has read that evidence and takes responsibility for it; the step advances still recorded as failed against the flag." }),
        /* @__PURE__ */ jsx(GamingFlags, { ...args }),
        /* @__PURE__ */ jsx("p", { children: "It is not the last step, so the job carries on at the next one. Your reason is written to this job's log and stays there — the log is append-only, and nothing takes an override back. It is not sent to the drone, which did nothing wrong and is told only that the step was accepted." })
      ]
    }
  )
};
const __vite_glob_0_13 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  APatternWithNoVerb,
  FlaggedAndNotCited,
  OnTheRail,
  OverrulingTwoFlags,
  TwoFlagsReadInFull,
  WhatItMeansAndWhereItIs,
  WhereWithNowhereToGo,
  default: meta$W
}, Symbol.toStringTag, { value: "Module" }));
function JobBrief({
  criteria: criteria2,
  criteriaAbsent,
  facts: facts2,
  factsAbsent,
  waiting,
  criteriaLabel = "Done means",
  factsLabel = "What it was told",
  waitingLabel = "Waiting to be told",
  only
}) {
  return /* @__PURE__ */ jsxs("div", { className: "armada-job-brief", children: [
    waiting === void 0 ? null : /* @__PURE__ */ jsxs("div", { className: "armada-job-brief__block", "data-waiting": true, children: [
      /* @__PURE__ */ jsx(Label, { children: waitingLabel }),
      /* @__PURE__ */ jsx("p", { className: "armada-job-brief__facts", children: waiting })
    ] }),
    only === "facts" ? null : /* @__PURE__ */ jsxs("div", { className: "armada-job-brief__block", children: [
      /* @__PURE__ */ jsx(Label, { children: criteriaLabel }),
      criteria2.length === 0 ? /* @__PURE__ */ jsx("p", { className: "armada-job-brief__note", children: criteriaAbsent }) : /* @__PURE__ */ jsx("ol", { className: "armada-job-brief__criteria", children: criteria2.map((criterion, i) => /* @__PURE__ */ jsxs("li", { className: "armada-job-brief__criterion", children: [
        /* @__PURE__ */ jsx("span", { className: "armada-job-brief__ordinal", children: i + 1 }),
        /* @__PURE__ */ jsx("span", { className: "armada-job-brief__text", children: criterion.text }),
        criterion.source === void 0 ? /* @__PURE__ */ jsx("span", {}) : /* @__PURE__ */ jsx("span", { className: "armada-job-brief__source", children: criterion.source })
      ] }, i)) })
    ] }),
    only === "criteria" ? null : /* @__PURE__ */ jsxs("div", { className: "armada-job-brief__block", children: [
      /* @__PURE__ */ jsx(Label, { children: factsLabel }),
      facts2 === void 0 ? /* @__PURE__ */ jsx("p", { className: "armada-job-brief__note", children: factsAbsent }) : /* @__PURE__ */ jsx("p", { className: "armada-job-brief__facts", children: facts2 })
    ] })
  ] });
}
function Label({ children }) {
  if (children === null) return null;
  return /* @__PURE__ */ jsx("span", { className: "armada-job-brief__label", children });
}
const meta$V = {
  title: "Compositions/Job brief",
  component: JobBrief
};
const criteria = [
  { text: "A burst of 401s produces one refresh call, not one per request.", source: "check" },
  { text: "The retry ceiling is unchanged.", source: "check" },
  { text: "No token is written to a log line at any sink.", source: "judge" }
];
const facts = "The refresh path is in `auth/session.ts`. Two callers hit it concurrently on a cold start, and the second one wins. Keep the public signature.";
const Brief = {
  args: { criteria, facts }
};
const NoCriteria = {
  args: {
    criteria: [],
    criteriaAbsent: "This job was proposed with no acceptance criteria, so nothing states what done means for it.",
    facts
  }
};
const NoFacts = {
  args: {
    criteria,
    factsAbsent: "This job was given no context beyond its title."
  }
};
const CriteriaOnly = {
  args: { criteria, only: "criteria" }
};
const FactsOnly = {
  args: { criteria: [], facts, only: "facts" }
};
const AsALine = {
  args: { criteria: [], facts, only: "facts", factsLabel: null }
};
const __vite_glob_0_14 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AsALine,
  Brief,
  CriteriaOnly,
  FactsOnly,
  NoCriteria,
  NoFacts,
  default: meta$V
}, Symbol.toStringTag, { value: "Module" }));
function joined$1(base, extra) {
  return extra ? `${base} ${extra}` : base;
}
function Card$7({ className, ...rest }) {
  return /* @__PURE__ */ jsx("div", { className: joined$1("armada-card", className), ...rest });
}
function CardHeader({ className, ...rest }) {
  return /* @__PURE__ */ jsx("div", { className: joined$1("armada-card-header", className), ...rest });
}
function CardTitle({ className, ...rest }) {
  return /* @__PURE__ */ jsx("h3", { className: joined$1("armada-card-title", className), ...rest });
}
function CardDescription({ className, ...rest }) {
  return /* @__PURE__ */ jsx("p", { className: joined$1("armada-card-description", className), ...rest });
}
function CardContent({ className, ...rest }) {
  return /* @__PURE__ */ jsx("div", { className: joined$1("armada-card-content", className), ...rest });
}
function CardFooter({ className, ...rest }) {
  return /* @__PURE__ */ jsx("div", { className: joined$1("armada-card-footer", className), ...rest });
}
function JobComposer({
  title,
  brief,
  workflows,
  project,
  glance,
  provenance,
  onCancel,
  onDispatch
}) {
  return /* @__PURE__ */ jsxs(Card$7, { className: "armada-job-composer", children: [
    /* @__PURE__ */ jsx(Input, { label: "Title", defaultValue: title }),
    /* @__PURE__ */ jsx(Textarea, { label: "Brief", defaultValue: brief }),
    /* @__PURE__ */ jsxs("div", { className: "armada-job-composer__pair", children: [
      /* @__PURE__ */ jsx(Select, { label: "Workflow", children: workflows }),
      /* @__PURE__ */ jsxs("div", { className: "armada-job-composer__readonly", children: [
        /* @__PURE__ */ jsx("span", { className: "armada-job-composer__label", children: "Project" }),
        /* @__PURE__ */ jsx("span", { className: "armada-job-composer__static", children: project })
      ] })
    ] }),
    /* @__PURE__ */ jsx("div", { className: "armada-job-composer__glance", children: glance.map((field, i) => /* @__PURE__ */ jsxs("div", { className: "armada-job-composer__field", children: [
      /* @__PURE__ */ jsx("span", { className: "armada-job-composer__field-label", children: field.label }),
      /* @__PURE__ */ jsx("span", { className: "armada-job-composer__field-value", children: field.value })
    ] }, i)) }),
    /* @__PURE__ */ jsxs(CardFooter, { className: "armada-job-composer__foot", children: [
      provenance ? /* @__PURE__ */ jsx("span", { className: "armada-job-composer__provenance", children: provenance }) : null,
      /* @__PURE__ */ jsxs("div", { className: "armada-job-composer__actions", children: [
        /* @__PURE__ */ jsx(Button, { onClick: onCancel, children: "Cancel" }),
        /* @__PURE__ */ jsx(Button, { variant: "primary", onClick: onDispatch, children: "Approve and dispatch" })
      ] })
    ] })
  ] });
}
const meta$U = {
  title: "Compositions/Job composer",
  component: JobComposer
};
const WhatM1Renders = {
  args: {
    title: "Coalesce concurrent token refreshes",
    brief: "A burst of 401s should produce one refresh call, not one per request. Keep the retry ceiling where it is.",
    workflows: /* @__PURE__ */ jsx("option", { children: "bug — 4 steps" }),
    project: "armada",
    glance: [
      { label: "Steps", value: "4 · 2 gated" },
      { label: "Checks", value: "build, test" }
    ],
    provenance: "Dispatched by you"
  }
};
const NoChecksOnTheWorkflow = {
  args: {
    title: "Draft the release note",
    brief: "One paragraph per merged job since the last tag. No links out.",
    workflows: /* @__PURE__ */ jsx("option", { children: "note — 2 steps" }),
    project: "armada",
    glance: [
      { label: "Steps", value: "2 · 0 gated" },
      { label: "Checks", value: "none" }
    ],
    provenance: "Dispatched by you"
  }
};
const __vite_glob_0_15 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  NoChecksOnTheWorkflow,
  WhatM1Renders,
  default: meta$U
}, Symbol.toStringTag, { value: "Module" }));
function JobDetailHeaderActions({
  status,
  statusIcon,
  statusLabel,
  headline,
  jobId,
  fields,
  actions,
  onCopied
}) {
  const copy = useCallback(
    (event, value) => {
      event.stopPropagation();
      void navigator.clipboard.writeText(value).then(
        () => onCopied?.(value),
        // A failed clipboard write is otherwise indistinguishable from a dead
        // element, so the surface is told either way.
        () => onCopied?.(value)
      );
    },
    [onCopied]
  );
  const runs2 = [];
  for (const field of fields) {
    const previous = runs2[runs2.length - 1];
    if (field.continues && previous) previous.push(field);
    else runs2.push([field]);
  }
  return /* @__PURE__ */ jsxs("div", { className: "armada-job-head", children: [
    /* @__PURE__ */ jsxs("div", { className: "armada-job-head__ident", children: [
      /* @__PURE__ */ jsxs("div", { className: "armada-job-head__line", children: [
        /* @__PURE__ */ jsx(Badge, { status, icon: statusIcon, children: statusLabel }),
        /* @__PURE__ */ jsx("span", { className: "armada-job-head__title", children: headline }),
        jobId ? /* @__PURE__ */ jsx("span", { className: "armada-job-head__id", children: jobId }) : null
      ] }),
      /* @__PURE__ */ jsx("div", { className: "armada-job-head__facts", children: runs2.map((run, i) => /* @__PURE__ */ jsx("span", { className: "armada-job-head__fact", children: run.map((field, j) => /* @__PURE__ */ jsxs(Fragment$1, { children: [
        j > 0 ? ", " : null,
        field.label ? /* @__PURE__ */ jsxs(Fragment, { children: [
          field.label,
          field.value !== void 0 ? " " : null
        ] }) : null,
        field.value !== void 0 ? /* @__PURE__ */ jsx(
          "span",
          {
            className: "armada-job-head__value",
            "data-mono": field.mono || void 0,
            "data-copies": field.copyValue !== void 0 || void 0,
            onClick: field.copyValue !== void 0 ? (e) => copy(e, field.copyValue) : void 0,
            children: field.value
          }
        ) : null,
        field.suffix ? /* @__PURE__ */ jsxs(Fragment, { children: [
          " ",
          field.suffix
        ] }) : null
      ] }, j)) }, i)) })
    ] }),
    actions ? /* @__PURE__ */ jsx("div", { className: "armada-job-head__actions", children: actions }) : null
  ] });
}
const meta$T = {
  title: "Compositions/Job detail header actions",
  component: JobDetailHeaderActions
};
const ARunningJob = {
  args: {
    status: "running",
    statusIcon: CircleDot,
    statusLabel: "Running",
    headline: "Split the settings reducer",
    jobId: "job_2d90bb",
    fields: [
      { label: "Step", value: "2 of 4", mono: true },
      {
        label: "Branch",
        value: "fix/settings-split",
        mono: true,
        copyValue: "fix/settings-split"
      },
      { label: "Elapsed", value: "11m 03s", mono: true },
      { label: "Spend, estimated", value: "~$1.80", mono: true },
      { label: "Dispatched by you" }
    ],
    actions: /* @__PURE__ */ jsxs(Fragment, { children: [
      /* @__PURE__ */ jsx(Button, { variant: "ghost", children: "Watch the turns" }),
      /* @__PURE__ */ jsx(Button, { variant: "destructive", children: "Kill job" })
    ] })
  }
};
const AFailedJob = {
  args: {
    status: "completed-failed",
    statusIcon: X,
    statusLabel: "Failed",
    headline: "Cache the manifest read",
    jobId: "job_91ab",
    fields: [
      { label: "Stopped at", value: "Run tests" },
      { label: "step", value: "3 of 4", mono: true, continues: true },
      { label: "Ran", value: "22m 41s", mono: true },
      { label: "Spend, estimated", value: "~$2.10", mono: true },
      { label: "Dispatched by you" }
    ]
  }
};
const AFinishedJob = {
  args: {
    status: "completed-success",
    statusIcon: Check,
    statusLabel: "Done",
    headline: "Add a retry ceiling to the poke loop",
    jobId: "job_4f10",
    fields: [
      { label: "All", value: "4 of 4", mono: true, suffix: "steps advanced" },
      { label: "Ran", value: "18m 22s", mono: true },
      { label: "Spend, estimated", value: "~$2.40", mono: true },
      { label: "Dispatched by you" }
    ]
  }
};
const BothKills = {
  args: {
    ...ARunningJob.args,
    fields: [
      { label: "Step", value: "2 of 4", mono: true },
      { label: "at", value: "implement", mono: true, continues: true },
      { label: "Elapsed", value: "11m 03s", mono: true },
      { label: "Branch", value: "fix/settings-split", mono: true, copyValue: "fix/settings-split" },
      { label: "Drone", value: "drn_7c21", mono: true, copyValue: "drn_7c21" },
      { label: "Writes", value: "src/settings/reducer.ts", mono: true }
    ],
    actions: /* @__PURE__ */ jsxs(Fragment, { children: [
      /* @__PURE__ */ jsx(Button, { variant: "ghost", children: "Watch the turns" }),
      /* @__PURE__ */ jsx(
        SplitButton,
        {
          variant: "destructive",
          menuLabel: "What else ends this job",
          items: [{ label: "Kill drone, the job stays open" }],
          children: "Kill job"
        }
      )
    ] })
  }
};
const BothKillsMenuOpen = {
  args: {
    ...BothKills.args,
    actions: /* @__PURE__ */ jsxs(Fragment, { children: [
      /* @__PURE__ */ jsx(Button, { variant: "ghost", children: "Watch the turns" }),
      /* @__PURE__ */ jsx(
        SplitButton,
        {
          variant: "destructive",
          defaultOpen: true,
          menuLabel: "What else ends this job",
          items: [{ label: "Kill drone, the job stays open" }],
          children: "Kill job"
        }
      )
    ] })
  }
};
const StoppedWithARedispatch = {
  args: {
    status: "escalated",
    statusIcon: OctagonAlert,
    statusLabel: "stalled",
    headline: "Cache the manifest read",
    jobId: "job_91ab04",
    fields: [
      { label: "Step", value: "3 of 4", mono: true },
      { label: "at", value: "verify", mono: true, continues: true },
      { label: "Elapsed", value: "22m 41s", mono: true },
      { label: "Branch", value: "feat/manifest-cache", mono: true, copyValue: "feat/manifest-cache" },
      { label: "Scope undetermined" }
    ],
    actions: /* @__PURE__ */ jsxs(Fragment, { children: [
      /* @__PURE__ */ jsx(Button, { variant: "ghost", children: "Watch the turns" }),
      /* @__PURE__ */ jsx(
        SplitButton,
        {
          variant: "destructive",
          defaultOpen: true,
          menuLabel: "What else ends this job",
          items: [{ label: "Kill job, it ends here", danger: true }],
          children: "Redispatch as a new job"
        }
      )
    ] })
  }
};
const AtTheApprovalGate = {
  args: {
    status: "awaiting-approval",
    statusIcon: UserCheck,
    statusLabel: "needs approval",
    headline: "Cache the manifest read",
    jobId: "job_91ab04",
    fields: [
      { label: "Step", value: "1 of 4", mono: true },
      { label: "at", value: "plan", mono: true, continues: true },
      { label: "Waiting", value: "4m 12s", mono: true },
      { label: "Writes", value: "crates/config/src/manifest.rs", mono: true }
    ],
    actions: /* @__PURE__ */ jsxs(Fragment, { children: [
      /* @__PURE__ */ jsx(Button, { variant: "ghost", children: "Watch the turns" }),
      /* @__PURE__ */ jsx(Button, { variant: "destructive", children: "Kill job" }),
      /* @__PURE__ */ jsx(Button, { variant: "primary", children: "Approve dispatch" })
    ] })
  }
};
const AsTheWindowNarrows = {
  args: AtTheApprovalGate.args,
  render: (args) => /* @__PURE__ */ jsx("div", { style: { display: "flex", flexDirection: "column", gap: "var(--space-8)" }, children: [
    ["the narrowest window Bridge opens", "var(--window-floor)"],
    ["the panel inside one", "var(--w-dialog-wide)"],
    ["narrower still", "var(--w-sheet)"],
    ["narrower than anything ships", "var(--w-dialog)"]
  ].map(([said, width]) => /* @__PURE__ */ jsxs("div", { style: { width, maxWidth: "100%" }, children: [
    /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", children: said }),
    /* @__PURE__ */ jsx(JobDetailHeaderActions, { ...args })
  ] }, width)) })
};
const __vite_glob_0_16 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AFailedJob,
  AFinishedJob,
  ARunningJob,
  AsTheWindowNarrows,
  AtTheApprovalGate,
  BothKills,
  BothKillsMenuOpen,
  StoppedWithARedispatch,
  default: meta$T
}, Symbol.toStringTag, { value: "Module" }));
const meta$S = {
  title: "Compositions/Job log reference",
  component: JobLogReference
};
const LOG = File;
const OPENS = { label: "Open the log", go: () => Promise.resolve(null) };
const OnARunningJob = {
  args: {
    rows: [
      {
        icon: LOG,
        iconLabel: "Log",
        value: ".armada/logs/job_2d90bb.jsonl",
        copyValue: ".armada/logs/job_2d90bb.jsonl",
        meta: "142 lines · 0 error",
        open: OPENS
      }
    ],
    children: "Fleet, the drone and Bridge in one order, keyed on this job. It is being written now."
  }
};
const OnAFailedJob = {
  args: {
    rows: [
      {
        icon: GitBranch,
        iconLabel: "Branch",
        value: "feat/manifest-cache",
        copyValue: "feat/manifest-cache",
        meta: "2 files +48 −11"
      },
      {
        icon: Folder,
        iconLabel: "Worktree",
        value: "~/.armada/worktrees/job_91ab",
        copyValue: "~/.armada/worktrees/job_91ab",
        open: { label: "Open the worktree", go: () => Promise.resolve(null) }
      },
      {
        icon: LOG,
        iconLabel: "Log",
        value: ".armada/logs/job_91ab.jsonl",
        copyValue: ".armada/logs/job_91ab.jsonl",
        meta: "318 lines · 4 error",
        separated: true,
        open: OPENS
      }
    ],
    children: "The worktree and the branch are left in place. Armada will not touch either. The log holds Fleet, the drone and Bridge in one order, keyed on this job."
  }
};
const OnAFinishedJob = {
  args: {
    rows: [
      {
        icon: LOG,
        iconLabel: "Log",
        value: ".armada/logs/job_4f10.jsonl",
        copyValue: ".armada/logs/job_4f10.jsonl",
        meta: "204 lines · 0 error"
      }
    ],
    actions: /* @__PURE__ */ jsx(Button, { ground: "sunken", children: "Open the log" })
  }
};
const WithErrors = {
  args: {
    rows: [
      {
        icon: LOG,
        iconLabel: "Log",
        value: ".armada/logs/job_91ab.jsonl",
        copyValue: ".armada/logs/job_91ab.jsonl",
        meta: "318 lines · 4 error"
      }
    ],
    children: "Whether the error count is computed per view or carried on the job record is open.",
    actions: /* @__PURE__ */ jsx(Button, { ground: "sunken", children: "Open the log" })
  }
};
const LongPaths = {
  decorators: [
    (Story) => /* @__PURE__ */ jsx("div", { className: "armada-log-ref-narrow", children: /* @__PURE__ */ jsx(Story, {}) })
  ],
  args: {
    rows: [
      {
        icon: Folder,
        iconLabel: "Worktree",
        value: "/Users/user/Development/armada/.armada/worktrees/01JQ8ZK4T7WY3N2VXB6RGM5D9C",
        copyValue: "/Users/user/Development/armada/.armada/worktrees/01JQ8ZK4T7WY3N2VXB6RGM5D9C"
      },
      {
        icon: GitBranch,
        iconLabel: "Branch",
        value: "armada/01JQ8ZK4T7WY3N2VXB6RGM5D9C",
        copyValue: "armada/01JQ8ZK4T7WY3N2VXB6RGM5D9C"
      },
      {
        icon: LOG,
        iconLabel: "Log",
        value: "/Users/user/Development/armada/.armada/logs/01JQ8ZK4T7WY3N2VXB6RGM5D9C.jsonl",
        copyValue: "/Users/user/Development/armada/.armada/logs/01JQ8ZK4T7WY3N2VXB6RGM5D9C.jsonl",
        separated: true
      },
      {
        iconLabel: "Transcript",
        value: "/Users/user/Development/armada/.armada/transcripts/",
        copyValue: "/Users/user/Development/armada/.armada/transcripts/",
        meta: "named by a drone id nothing serves"
      }
    ],
    children: "The worktree, the log and the transcripts directory follow from this job's id and the repository its manifest was read from. The branch is served."
  }
};
const WhatOpensAndWhatOnlyCopies = {
  args: {
    rows: [
      {
        icon: Folder,
        iconLabel: "Worktree",
        value: "/Users/user/Development/armada/.armada/worktrees/01JQ8ZK4T7WY3N2VXB6RGM5D9C",
        copyValue: "/Users/user/Development/armada/.armada/worktrees/01JQ8ZK4T7WY3N2VXB6RGM5D9C",
        open: { label: "Open the worktree", go: () => Promise.resolve(null) }
      },
      {
        icon: GitBranch,
        iconLabel: "Branch",
        value: "armada/01JQ8ZK4T7WY3N2VXB6RGM5D9C",
        copyValue: "armada/01JQ8ZK4T7WY3N2VXB6RGM5D9C"
      },
      {
        icon: LOG,
        iconLabel: "Log",
        value: "/Users/user/Development/armada/.armada/logs/01JQ8ZK4T7WY3N2VXB6RGM5D9C.jsonl",
        copyValue: "/Users/user/Development/armada/.armada/logs/01JQ8ZK4T7WY3N2VXB6RGM5D9C.jsonl",
        separated: true,
        open: OPENS
      }
    ],
    children: "The worktree and the log open in whatever this machine opens them with. The branch copies."
  }
};
const WhenThePathIsGone = {
  args: {
    rows: [
      {
        icon: Folder,
        iconLabel: "Worktree",
        value: "/Users/user/Development/armada/.armada/worktrees/01JQ8ZK4T7WY3N2VXB6RGM5D9C",
        copyValue: "/Users/user/Development/armada/.armada/worktrees/01JQ8ZK4T7WY3N2VXB6RGM5D9C",
        open: {
          label: "Open the worktree",
          go: () => Promise.resolve({
            because: "Nothing is at that worktree. It was reclaimed, and the branch survives it."
          })
        }
      }
    ],
    children: "The branch is still there. The directory it was checked out into is not."
  }
};
const __vite_glob_0_17 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  LongPaths,
  OnAFailedJob,
  OnAFinishedJob,
  OnARunningJob,
  WhatOpensAndWhatOnlyCopies,
  WhenThePathIsGone,
  WithErrors,
  default: meta$S
}, Symbol.toStringTag, { value: "Module" }));
const ROW_ICON = 12;
const ROW_STROKE = 2;
function JobOutcome({ parts, note, onCopied }) {
  const copy = useCallback(
    (event, value) => {
      event.stopPropagation();
      void navigator.clipboard.writeText(value).then(() => onCopied?.(value));
    },
    [onCopied]
  );
  return /* @__PURE__ */ jsxs("div", { className: "armada-outcome", children: [
    /* @__PURE__ */ jsx("ol", { className: "armada-outcome__parts", children: parts.map((part, i) => /* @__PURE__ */ jsxs("li", { className: "armada-outcome__part", children: [
      /* @__PURE__ */ jsxs("span", { className: "armada-outcome__mark", children: [
        part.icon ? /* @__PURE__ */ jsx(part.icon, { size: ROW_ICON, strokeWidth: ROW_STROKE, "aria-hidden": true }) : null,
        part.iconLabel ? /* @__PURE__ */ jsx("span", { className: "armada-outcome__sr", children: part.iconLabel }) : null
      ] }),
      /* @__PURE__ */ jsx("span", { className: "armada-outcome__name", children: part.name }),
      part.value === void 0 ? /* @__PURE__ */ jsx("span", { className: "armada-outcome__absent", children: part.absent }) : (
        /* The title carries the whole value however narrow the row gets,
           and so does the clipboard: a copy that truncated with the
           display would be worse than the overflow it was fixing. */
        /* @__PURE__ */ jsx(
          "span",
          {
            className: "armada-outcome__value",
            title: part.value,
            onClick: (event) => copy(event, part.value),
            children: part.value
          }
        )
      ),
      /* @__PURE__ */ jsx("span", { className: "armada-outcome__meta", children: part.meta }),
      /* @__PURE__ */ jsx("span", { className: "armada-outcome__action", children: part.action })
    ] }, i)) }),
    note === void 0 ? null : /* @__PURE__ */ jsx("p", { className: "armada-outcome__note", children: note })
  ] });
}
const meta$R = {
  title: "Compositions/Job outcome",
  component: JobOutcome
};
const NOTE$1 = "Armada does not merge. The branch is pushed and the review is yours to take.";
const WhatIsServedToday = {
  args: {
    note: NOTE$1,
    parts: [
      {
        name: "Branch",
        icon: GitBranch,
        iconLabel: "Branch",
        value: "armada/01M130Y1380016YK5S0JXBXDQ5"
      },
      {
        name: "Commit",
        icon: GitCommitHorizontal,
        iconLabel: "Commit",
        value: "5375d705cb7713a21a91681c1028166b98a0d6de",
        meta: "origin/armada/01M1CNPKTV0018H2M1CXDNBK06"
      },
      {
        name: "Pull request",
        icon: GitPullRequest,
        iconLabel: "Pull request",
        value: "https://example.invalid/armada/pull/229"
      },
      {
        /* No glyph. `file` is reserved to the log row and `file-check` to a
           submission that landed, so a changed-file row has nothing in the
           registry to take. The mark column stays and renders empty. */
        name: "Files changed",
        absent: "job.files_changed is published while a drone is working. Nothing serves a finished job's footprint."
      },
      {
        name: "Evidence",
        icon: FileCheck,
        iconLabel: "Evidence",
        absent: "No operation serves a work submission, so there is nothing to draw."
      }
    ]
  }
};
const EveryPartServed = {
  args: {
    note: NOTE$1,
    parts: [
      {
        name: "Branch",
        icon: GitBranch,
        iconLabel: "Branch",
        value: "armada/01M130Y1380016YK5S0JXBXDQ5",
        meta: "from main"
      },
      {
        name: "Commit",
        icon: GitCommitHorizontal,
        iconLabel: "Commit",
        value: "9f2c1ab",
        meta: "1 commit"
      },
      {
        name: "Pull request",
        icon: GitPullRequest,
        iconLabel: "Pull request",
        value: "armada#42"
      },
      { name: "Files changed", value: "4 files", meta: "+214 −96" },
      { name: "Evidence", icon: FileCheck, iconLabel: "Evidence", value: "3 submissions" }
    ]
  }
};
const NoBranch = {
  args: {
    parts: [
      {
        name: "Branch",
        icon: GitBranch,
        iconLabel: "Branch",
        absent: "This job has no worktree, so it has no branch."
      }
    ]
  }
};
const __vite_glob_0_18 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  EveryPartServed,
  NoBranch,
  WhatIsServedToday,
  default: meta$R
}, Symbol.toStringTag, { value: "Module" }));
function Tabs({ items, value, defaultValue, onChange }) {
  const [internal, setInternal] = useState(defaultValue ?? items[0]?.id);
  const active = value ?? internal;
  function select(id) {
    if (value === void 0) setInternal(id);
    onChange?.(id);
  }
  function onKey(event) {
    const at = items.findIndex((item) => item.id === active);
    if (items.length === 0) {
      return;
    }
    if (event.key === "ArrowRight") {
      event.preventDefault();
      select(items[(at + 1) % items.length].id);
    }
    if (event.key === "ArrowLeft") {
      event.preventDefault();
      select(items[(at - 1 + items.length) % items.length].id);
    }
  }
  return /* @__PURE__ */ jsx("div", { className: "armada-tabs", role: "tablist", onKeyDown: onKey, children: items.map((item) => /* @__PURE__ */ jsx(
    "button",
    {
      type: "button",
      role: "tab",
      "aria-selected": item.id === active,
      tabIndex: item.id === active ? 0 : -1,
      className: item.id === active ? "armada-tabs__tab armada-tabs__tab--active" : "armada-tabs__tab",
      onClick: () => select(item.id),
      children: item.label
    },
    item.id
  )) });
}
function JobRecord({
  sections: sections2,
  value,
  defaultValue,
  onChange,
  emptyNote = "Nothing about this job is recorded yet."
}) {
  const [internal, setInternal] = useState(defaultValue ?? sections2[0]?.id);
  const asked = value ?? internal;
  const open2 = sections2.find((section) => section.id === asked) ?? sections2[0];
  if (sections2.length === 0) {
    return /* @__PURE__ */ jsx("div", { className: "armada-record", children: /* @__PURE__ */ jsx("p", { className: "armada-record__note", children: emptyNote }) });
  }
  return /* @__PURE__ */ jsxs("div", { className: "armada-record", children: [
    /* @__PURE__ */ jsx(
      Tabs,
      {
        items: sections2.map((section) => ({ id: section.id, label: section.label })),
        value: open2?.id,
        onChange: (id) => {
          if (value === void 0) setInternal(id);
          onChange?.(id);
        }
      }
    ),
    /* @__PURE__ */ jsx("div", { className: "armada-record__panel", role: "tabpanel", children: open2?.panel })
  ] });
}
const meta$Q = {
  title: "Compositions/Job record",
  component: JobRecord
};
const sections = [
  { id: "steps", label: "Steps and checks", panel: /* @__PURE__ */ jsx(Panel, { children: "The workflow rail goes here." }) },
  { id: "turns", label: "The drone's turns", panel: /* @__PURE__ */ jsx(Panel, { children: "The transcript goes here." }) },
  {
    id: "told",
    label: "What it was told",
    panel: /* @__PURE__ */ jsx(
      JobBrief,
      {
        criteria: [],
        only: "facts",
        facts: "The refresh path is in `auth/session.ts`. Keep the public signature."
      }
    )
  },
  {
    id: "paths",
    label: "Where the work is",
    panel: /* @__PURE__ */ jsx(
      JobLogReference,
      {
        rows: [
          { value: "/w/api/.armada/worktrees/01M130Y1380016YK5S0JXBXDQ5" },
          { value: "/w/api/.armada/logs/01M130Y1380016YK5S0JXBXDQ5.jsonl", separated: true }
        ],
        children: "The worktree, the log and the transcripts directory follow from this job's id."
      }
    )
  }
];
const FoldedRecord = {
  args: { sections, defaultValue: "steps" }
};
const ASectionOpened = {
  args: { sections, defaultValue: "told" }
};
const NothingRecorded = {
  args: {
    sections: [],
    emptyNote: "Fleet has not answered for this job, so there is no record to fold."
  }
};
function Panel({ children }) {
  return /* @__PURE__ */ jsx("p", { className: "armada-record__note", children });
}
const __vite_glob_0_19 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  ASectionOpened,
  FoldedRecord,
  NothingRecorded,
  default: meta$Q
}, Symbol.toStringTag, { value: "Module" }));
const meta$P = {
  title: "Compositions/Job row (stacked)",
  component: JobRowStacked
};
const open$1 = /* @__PURE__ */ jsx(
  SplitButton,
  {
    ground: "card",
    items: [
      { label: "Copy job id", shortcut: "⌘C" },
      { label: "Kill", shortcut: "x", danger: true }
    ],
    children: "Open"
  }
);
const GATE_TRACKS = [
  "var(--armada-track-origin)",
  "var(--armada-track-bar)",
  "var(--armada-track-step)",
  "var(--armada-track-created)",
  "var(--armada-track-provenance)"
].join(" ");
const NeedsApproval = {
  args: {
    status: "awaiting-approval",
    statusIcon: UserCheck,
    statusLabel: "Needs approval",
    headline: "Coalesce concurrent token refreshes",
    jobId: "job_7c31",
    tracks: GATE_TRACKS,
    fields: [
      { label: "Workflow", value: "bug, 4 steps", quiet: true },
      { value: /* @__PURE__ */ jsx(StepBar, { total: 4, current: 0, label: "Not started, 4 steps" }) },
      { value: "Not started", quiet: true },
      { value: "created 09:12", quiet: true },
      { value: "Dispatched by you" }
    ],
    action: /* @__PURE__ */ jsx(SplitButton, { ground: "card", items: [{ label: "Reject", danger: true }], children: "Approve" })
  }
};
const Queued$2 = {
  args: {
    status: "not-started",
    statusIcon: Clock,
    statusLabel: "Queued",
    headline: "Retire the legacy poke path",
    jobId: "job_8b42",
    tracks: GATE_TRACKS,
    fields: [
      { label: "Workflow", value: "bug, 4 steps", quiet: true },
      { value: /* @__PURE__ */ jsx(StepBar, { total: 4, current: 0, label: "Not started, 4 steps" }) },
      { value: "Not started", quiet: true },
      { value: "approved 09:20", quiet: true },
      { value: "Dispatched by you" }
    ],
    action: open$1
  }
};
const QueuedAfterARestart = {
  args: {
    status: "not-started",
    statusIcon: Cpu,
    statusLabel: "Waiting on resources",
    headline: "Retire the legacy poke path, restarted",
    jobId: "job_8b42",
    tracks: GATE_TRACKS,
    fields: [
      { label: "Workflow", value: "bug, 4 steps", quiet: true },
      { value: /* @__PURE__ */ jsx(StepBar, { total: 4, current: 2, label: "Step 2 of 4" }) },
      { value: "Run tests", mono: true, emphasis: true },
      { value: "queued 09:41", quiet: true },
      { value: "Dispatched by you" }
    ],
    action: open$1
  }
};
const Running$8 = {
  args: {
    pulsing: true,
    status: "running",
    statusIcon: CircleDot,
    statusLabel: "Running",
    headline: "Split the settings reducer",
    jobId: "job_2d90bb",
    fields: [
      { value: "fix/settings-split", mono: true, icon: GitBranch, copyValue: "fix/settings-split" },
      { value: /* @__PURE__ */ jsx(StepBar, { total: 4, current: 2, activity: "running", label: "Step 2 of 4" }) },
      { value: "Implement", emphasis: true },
      { value: "11m 03s", mono: true },
      { value: "~$1.80", mono: true },
      { value: "Dispatched by you" }
    ],
    action: open$1
  }
};
const RunningFocused = {
  args: { ...Running$8.args, focused: true }
};
const FocusedWithItsKey = {
  args: { ...Running$8.args, focused: true, actionKey: "o" }
};
const UnfocusedWithItsKey = {
  args: { ...Running$8.args, actionKey: "o" }
};
const Selected$1 = {
  args: { ...Running$8.args, selected: true }
};
const Dimmed$2 = {
  args: { ...Running$8.args, dimmed: true }
};
const EscalatedStalled = {
  args: {
    status: "escalated",
    statusIcon: OctagonAlert,
    statusLabel: "Stalled",
    headline: "Job 12 stalled at step 3",
    jobId: "job_12",
    fields: [
      { value: "auth/session.rs", mono: true, icon: GitBranch, copyValue: "auth/session.rs" },
      { value: /* @__PURE__ */ jsx(StepBar, { total: 5, current: 3, activity: "stopped", label: "Step 3 of 5" }) },
      { value: "3 pokes", emphasis: true },
      { value: "12m", mono: true },
      { value: "~$1.80", mono: true },
      { value: "Found by Fleet" }
    ],
    action: /* @__PURE__ */ jsx(SplitButton, { ground: "card", items: [{ label: "Kill", danger: true }], children: "Pilot" })
  }
};
const EscalatedSecondTime = {
  args: {
    ...EscalatedStalled.args,
    headline: "Job 12 stalled at step 3, 2nd time"
  }
};
const Failed$7 = {
  args: {
    status: "completed-failed",
    statusIcon: X,
    statusLabel: "Failed",
    headline: "Cache the manifest read",
    jobId: "job_91ab",
    fields: [
      { value: "feat/manifest-cache", mono: true, icon: GitBranch, copyValue: "feat/manifest-cache" },
      { value: /* @__PURE__ */ jsx(StepBar, { total: 4, current: 3, activity: "failed", label: "Step 3 of 4" }) },
      { value: "Run tests", emphasis: true },
      { value: "22m 41s", mono: true },
      { value: "~$2.10", mono: true },
      { value: "Found by Fleet" }
    ],
    action: open$1
  }
};
const Killed$6 = {
  args: {
    status: "killed",
    statusIcon: Power,
    statusLabel: "Killed",
    headline: "Rename the session token field",
    jobId: "job_5e88",
    fields: [
      { value: "feat/session-rename", mono: true, icon: GitBranch, copyValue: "feat/session-rename" },
      { value: /* @__PURE__ */ jsx(StepBar, { total: 4, current: 2, activity: "killed", label: "Step 2 of 4" }) },
      { value: "Implement", emphasis: true },
      { value: "4m 09s", mono: true },
      { value: "~$0.60", mono: true },
      { value: "Workflow-triggered" }
    ],
    action: open$1
  }
};
const Done$1 = {
  args: {
    status: "completed-success",
    statusIcon: Check,
    statusLabel: "Done",
    headline: "Add a retry ceiling to the poke loop",
    jobId: "job_4f10",
    fields: [
      { value: "fix/poke-ceiling", mono: true, icon: GitBranch, copyValue: "fix/poke-ceiling" },
      { value: /* @__PURE__ */ jsx(StepBar, { total: 4, current: 5, activity: "advanced", label: "All 4 of 4 steps advanced" }) },
      { value: "Summarise" },
      { value: "18m 22s", mono: true },
      { value: "~$2.40", mono: true },
      { value: "Drafted in Helm" }
    ],
    action: open$1
  }
};
const SpendAsQuota = {
  args: {
    ...Running$8.args,
    fields: [
      { value: "fix/settings-split", mono: true, icon: GitBranch, copyValue: "fix/settings-split" },
      { value: /* @__PURE__ */ jsx(StepBar, { total: 4, current: 2, activity: "running", label: "Step 2 of 4" }) },
      { value: "Implement", emphasis: true },
      { value: "11m 03s", mono: true },
      { value: "68% quota", mono: true },
      { value: "Dispatched by you" }
    ]
  }
};
const AtTheWidthFloor = {
  render: () => /* @__PURE__ */ jsx("div", { style: { width: "calc(var(--window-floor) - var(--sidebar-rail))" }, children: /* @__PURE__ */ jsx(
    JobRowStacked,
    {
      status: "running",
      statusIcon: CircleDot,
      statusLabel: "Running",
      headline: "Split the settings reducer so the selectors can be tested alone",
      jobId: "job_2d90bb",
      fields: [
        { value: "fix/settings-split-selectors", mono: true, icon: GitBranch, copyValue: "fix/settings-split-selectors" },
        { value: /* @__PURE__ */ jsx(StepBar, { total: 4, current: 2, activity: "running", label: "Step 2 of 4" }) },
        { value: "Implement", emphasis: true },
        { value: "11m 03s", mono: true },
        { value: "~$1.80", mono: true },
        { value: "Dispatched by you" }
      ],
      action: open$1
    }
  ) })
};
const Convoy = {
  args: {
    ...Running$8.args,
    headline: "Retire the poke path across the fleet",
    fields: [
      { value: "crates/fleet +2", mono: true, icon: Folder },
      { value: /* @__PURE__ */ jsx(StepBar, { total: 4, current: 2, activity: "running", label: "Step 2 of 4" }) },
      { value: "Implement", emphasis: true },
      { value: "11m 03s", mono: true },
      { value: "~$4.20", mono: true },
      { value: "Workflow-triggered" }
    ]
  }
};
const SubDispatchedWaitingOnResources = {
  args: {
    status: "not-started",
    statusIcon: Cpu,
    statusLabel: "Waiting on resources",
    headline: "Precompute embeddings for the batch import step",
    jobId: "job_9f21",
    dimmed: true,
    tracks: GATE_TRACKS,
    fields: [
      { label: "Workflow", value: "chore, 3 steps", quiet: true },
      { value: /* @__PURE__ */ jsx(StepBar, { total: 3, current: 1, label: "Step 2 of 3, waiting" }) },
      { value: "embed-batch", mono: true, emphasis: true },
      { value: "queued 09:41", quiet: true },
      { value: "Sub-dispatched by job_2d90bb" }
    ],
    action: /* @__PURE__ */ jsx(SplitButton, { ground: "card", variant: "destructive", items: [], children: "Kill" })
  }
};
const TheLongestVerbAtTheWidthFloor = {
  render: () => /* @__PURE__ */ jsx("div", { style: { width: "calc(var(--window-floor) - var(--sidebar-rail))" }, children: /* @__PURE__ */ jsx(
    JobRowStacked,
    {
      status: "escalated",
      statusIcon: OctagonAlert,
      statusLabel: "A required command did not succeed",
      headline: "Reconcile orphaned drones on Fleet start",
      jobId: "job_31c7",
      fields: [
        { value: "fix/orphan-reconcile", mono: true, icon: GitBranch, copyValue: "fix/orphan-reconcile" },
        { value: /* @__PURE__ */ jsx(StepBar, { total: 5, current: 3, activity: "failed", label: "Step 4 of 5" }) },
        { value: "Regression check", emphasis: true },
        { value: "1h 12m", mono: true },
        { value: "~$3.40", mono: true },
        { value: "Dispatched by you" }
      ],
      action: open$1
    }
  ) })
};
const __vite_glob_0_20 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AtTheWidthFloor,
  Convoy,
  Dimmed: Dimmed$2,
  Done: Done$1,
  EscalatedSecondTime,
  EscalatedStalled,
  Failed: Failed$7,
  FocusedWithItsKey,
  Killed: Killed$6,
  NeedsApproval,
  Queued: Queued$2,
  QueuedAfterARestart,
  Running: Running$8,
  RunningFocused,
  Selected: Selected$1,
  SpendAsQuota,
  SubDispatchedWaitingOnResources,
  TheLongestVerbAtTheWidthFloor,
  UnfocusedWithItsKey,
  default: meta$P
}, Symbol.toStringTag, { value: "Module" }));
const NAMED = {
  armada: "Armada",
  drone: "Drone",
  fleet: "Fleet"
};
const GLYPH$3 = 12;
const STROKE$4 = 2;
function LogEntry({
  at,
  actor,
  message,
  mono,
  working,
  open: open2 = false,
  onToggle,
  payload,
  payloadAbsent,
  payloadId
}) {
  const who = NAMED[actor];
  const row = /* @__PURE__ */ jsxs(Fragment, { children: [
    /* @__PURE__ */ jsx("span", { className: "armada-entry__t", children: at }),
    /* @__PURE__ */ jsx("span", { className: "armada-entry__who", children: who }),
    /* @__PURE__ */ jsxs("span", { className: "armada-entry__msg", "data-mono": mono || void 0, children: [
      working ? /* @__PURE__ */ jsx("span", { className: "armada-entry__working", "aria-hidden": true }) : null,
      message
    ] }),
    open2 ? /* @__PURE__ */ jsx(ChevronDown, { size: GLYPH$3, strokeWidth: STROKE$4, className: "armada-entry__mark", "aria-hidden": true }) : /* @__PURE__ */ jsx(ChevronRight, { size: GLYPH$3, strokeWidth: STROKE$4, className: "armada-entry__mark", "aria-hidden": true })
  ] });
  return /* @__PURE__ */ jsxs("div", { className: "armada-entry-group", children: [
    onToggle === void 0 ? /* @__PURE__ */ jsx("div", { className: "armada-entry", "data-actor": actor, "data-open": open2 || void 0, children: row }) : /* @__PURE__ */ jsx(
      "button",
      {
        type: "button",
        className: "armada-entry",
        "data-actor": actor,
        "data-open": open2 || void 0,
        "aria-expanded": open2,
        "aria-controls": payloadId,
        onClick: onToggle,
        children: row
      }
    ),
    open2 ? /* @__PURE__ */ jsx("div", { className: "armada-entry__payload", id: payloadId, children: payload ?? /* @__PURE__ */ jsx("p", { className: "armada-entry__absent", children: payloadAbsent ?? "This line carried no payload." }) }) : null
  ] });
}
function PayloadLine({ children, named: named2 }) {
  return /* @__PURE__ */ jsx("span", { className: "armada-entry__pl", "data-named": named2, children });
}
const meta$O = {
  title: "Compositions/Log entry",
  component: LogEntry,
  decorators: [
    // The chapter the log streams inside. Every surface value on the row is
    // picked against --bg-sunken, including the payload's step below it.
    (Story) => /* @__PURE__ */ jsx(
      "div",
      {
        style: {
          width: "calc(var(--space-12) * 12)",
          padding: "var(--space-2)",
          borderRadius: "var(--radius-md)",
          background: "var(--bg-sunken)"
        },
        children: /* @__PURE__ */ jsx(Story, {})
      }
    )
  ]
};
const Armada = {
  args: {
    at: "14:22:07",
    actor: "armada",
    message: "Go on to Implement.",
    onToggle: () => {
    },
    payloadId: "entry-armada"
  }
};
const Drone = {
  args: {
    at: "14:26:31",
    actor: "drone",
    message: "Edit  packages/settings/src/selectors.ts",
    mono: true,
    onToggle: () => {
    },
    payloadId: "entry-drone"
  }
};
const Fleet = {
  args: {
    at: "14:30:28",
    actor: "fleet",
    message: "Heartbeat — the Drone has been quiet for 48 seconds",
    onToggle: () => {
    },
    payloadId: "entry-fleet"
  }
};
const Closed$1 = {
  args: {
    at: "14:23:11",
    actor: "drone",
    message: "Read  packages/settings/src/reducer.ts",
    mono: true,
    open: false,
    onToggle: () => {
    },
    payloadId: "entry-closed"
  }
};
const OpenWithItsPayload = {
  args: {
    at: "14:29:40",
    actor: "drone",
    message: "Bash  cargo build --workspace --locked",
    mono: true,
    open: true,
    onToggle: () => {
    },
    payloadId: "entry-open",
    payload: /* @__PURE__ */ jsxs(Fragment, { children: [
      /* @__PURE__ */ jsx(PayloadLine, { named: "echo", children: "$ cargo build --workspace --locked" }),
      /* @__PURE__ */ jsx(PayloadLine, { children: "   Compiling armada-settings v0.1.0 (packages/settings)" }),
      /* @__PURE__ */ jsx(PayloadLine, { children: "   Compiling armada-fleet v0.1.0 (crates/fleet)" }),
      /* @__PURE__ */ jsx(PayloadLine, { named: "passed", children: "    Finished `dev` profile [unoptimized] in 47.61s" }),
      /* @__PURE__ */ jsx(PayloadLine, { named: "meta", children: "exit 0 · 47.61s · in .armada/worktrees/job_2d90bb" })
    ] })
  }
};
const StillProducing = {
  args: {
    at: "14:31:58",
    actor: "drone",
    message: "thinking",
    working: true,
    onToggle: () => {
    },
    payloadId: "entry-working"
  }
};
const OpenAndEmpty = {
  args: {
    at: "14:32:40",
    actor: "fleet",
    message: "Heartbeat — the Drone has been quiet for 48 seconds",
    open: true,
    onToggle: () => {
    },
    payloadId: "entry-empty",
    payloadAbsent: "A heartbeat carries only its time."
  }
};
const TheStream = {
  render: () => /* @__PURE__ */ jsxs("div", { style: { display: "flex", flexDirection: "column" }, children: [
    /* @__PURE__ */ jsx(LogEntry, { at: "14:22:07", actor: "armada", message: "Go on to Implement.", onToggle: () => {
    } }),
    /* @__PURE__ */ jsx(
      LogEntry,
      {
        at: "14:26:31",
        actor: "drone",
        mono: true,
        message: "Edit  packages/settings/src/selectors.ts",
        onToggle: () => {
        }
      }
    ),
    /* @__PURE__ */ jsx(
      LogEntry,
      {
        at: "14:29:40",
        actor: "drone",
        mono: true,
        open: true,
        message: "Bash  cargo build --workspace --locked",
        onToggle: () => {
        },
        payload: /* @__PURE__ */ jsxs(Fragment, { children: [
          /* @__PURE__ */ jsx(PayloadLine, { named: "echo", children: "$ cargo build --workspace --locked" }),
          /* @__PURE__ */ jsx(PayloadLine, { children: "   Compiling armada-settings v0.1.0 (packages/settings)" }),
          /* @__PURE__ */ jsx(PayloadLine, { named: "passed", children: "    Finished `dev` profile [unoptimized] in 47.61s" }),
          /* @__PURE__ */ jsx(PayloadLine, { named: "meta", children: "exit 0 · 47.61s · in .armada/worktrees/job_2d90bb" })
        ] })
      }
    ),
    /* @__PURE__ */ jsx(
      LogEntry,
      {
        at: "14:30:28",
        actor: "fleet",
        message: "Heartbeat — the Drone has been quiet for 48 seconds",
        onToggle: () => {
        }
      }
    ),
    /* @__PURE__ */ jsx(LogEntry, { at: "14:31:58", actor: "drone", working: true, message: "thinking", onToggle: () => {
    } })
  ] })
};
const __vite_glob_0_21 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Armada,
  Closed: Closed$1,
  Drone,
  Fleet,
  OpenAndEmpty,
  OpenWithItsPayload,
  StillProducing,
  TheStream,
  default: meta$O
}, Symbol.toStringTag, { value: "Module" }));
function PathChip({ directory, basename, note, title, onCopy }) {
  const whole = title ?? `${directory ?? ""}${basename}`;
  const body2 = /* @__PURE__ */ jsxs(Fragment, { children: [
    directory === void 0 || directory === "" ? null : (
      // `dir="ltr"` inside an `rtl` box: the characters stay in reading
      // order and only the overflow end moves. Without it a path renders
      // its separators on the wrong side.
      /* @__PURE__ */ jsx("span", { className: "armada-path__dir", dir: "ltr", children: directory })
    ),
    /* @__PURE__ */ jsx("span", { className: "armada-path__base", children: basename }),
    note === void 0 ? null : /* @__PURE__ */ jsx("span", { className: "armada-path__note", children: note })
  ] });
  if (onCopy === void 0) {
    return /* @__PURE__ */ jsx("span", { className: "armada-path", title: whole, children: body2 });
  }
  return /* @__PURE__ */ jsx(
    "button",
    {
      type: "button",
      className: "armada-path",
      title: whole,
      onClick: (event) => {
        event.stopPropagation();
        void navigator.clipboard.writeText(whole).then(
          // A failed clipboard write is otherwise indistinguishable from a
          // dead element, so the surface is told either way.
          () => onCopy(whole),
          () => onCopy(whole)
        );
      },
      children: body2
    }
  );
}
const meta$N = {
  title: "Compositions/Path chip",
  component: PathChip
};
const AShortPath = {
  args: {
    directory: "packages/settings/test/",
    basename: "useColumnSelectors.test.ts"
  }
};
const ALongPathTruncatingItsDirectory = {
  render: () => /* @__PURE__ */ jsxs(
    "div",
    {
      style: {
        width: "calc(var(--space-12) * 4)",
        display: "flex",
        flexDirection: "column",
        gap: "var(--space-1)"
      },
      children: [
        /* @__PURE__ */ jsx(PathChip, { directory: ".armada/artifacts/job_2d90bb/", basename: "root_cause.md" }),
        /* @__PURE__ */ jsx(PathChip, { directory: "packages/settings/src/", basename: "selectors.ts" }),
        /* @__PURE__ */ jsx(PathChip, { directory: "packages/settings/src/", basename: "reducer.ts" }),
        /* @__PURE__ */ jsx(PathChip, { directory: "packages/settings/src/", basename: "index.ts" })
      ]
    }
  )
};
const WithWhatItIs = {
  args: {
    directory: "packages/settings/src/",
    basename: "selectors.ts",
    note: "+61 −4"
  }
};
const AtTheRoot = {
  args: { basename: "armada.yml" }
};
const __vite_glob_0_22 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  ALongPathTruncatingItsDirectory,
  AShortPath,
  AtTheRoot,
  WithWhatItIs,
  default: meta$N
}, Symbol.toStringTag, { value: "Module" }));
const GLYPHS = {
  phase: {
    ahead: void 0,
    current: CircleDot,
    cleared: Check,
    waiting: Eye,
    failed: X
  },
  checks: {
    ahead: ShieldMinus,
    current: ShieldMinus,
    cleared: ShieldCheck,
    waiting: ShieldMinus,
    failed: ShieldX
  },
  judge: {
    ahead: CircleMinus,
    current: CircleMinus,
    cleared: CircleCheck,
    waiting: CircleMinus,
    failed: CircleX
  },
  human: {
    ahead: UserCheck,
    current: UserCheck,
    cleared: UserCheck,
    waiting: UserCheck,
    failed: UserCheck
  }
};
function phaseGlyph(kind, state) {
  return GLYPHS[kind][state];
}
const SAID = {
  phase: void 0,
  checks: "Commands this repository declares in its own Manifest. Fleet runs them and the Drone never does — a Drone reporting its own tests is a claim, not a result.",
  judge: "A model reading the work against this step's acceptance criteria, the ones written when the Job was dispatched. It answers per criterion, and it never sees the Drone's transcript, so it cannot be argued at by the thing it is judging.",
  human: "The human gate, where the workflow asks for one. Everything mechanical has already cleared by the time this tier is lit, so a step sitting here is stopped with nothing wrong."
};
const CLOSES_WITH = {
  phase: void 0,
  checks: "A command and an exit code. Nothing to interpret, and the same answer every time it is run.",
  judge: "It can only refuse. A Judge never turns a failed Check into a pass.",
  human: "Amber, not red. It is waiting on you, not broken."
};
function verdictState(named2) {
  switch (named2) {
    case "passed":
    case "met":
      return "cleared";
    case "failed":
    case "not_met":
    case "refused":
      return "failed";
    case "running":
      return "current";
    default:
      return void 0;
  }
}
const HEAD_GLYPH = 16;
const ROW_GLYPH = 12;
const STROKE$3 = 2;
function PhaseCard({
  kind,
  name,
  state,
  stands,
  said,
  rows = [],
  note,
  detail,
  floating,
  align = "start"
}) {
  const Head = phaseGlyph(kind, state);
  const says = said === void 0 ? SAID[kind] : said;
  const closes = detail === void 0 ? CLOSES_WITH[kind] : detail;
  return /* @__PURE__ */ jsxs(
    "div",
    {
      className: "armada-phase-card",
      "data-kind": kind,
      "data-state": state,
      "data-floating": floating || void 0,
      "data-align": align,
      children: [
        /* @__PURE__ */ jsxs("div", { className: "armada-phase-card__head", children: [
          Head === void 0 ? null : /* @__PURE__ */ jsx(Head, { size: HEAD_GLYPH, strokeWidth: STROKE$3, className: "armada-phase-card__mark", "aria-hidden": true }),
          /* @__PURE__ */ jsx("span", { className: "armada-phase-card__name", children: name }),
          stands === void 0 ? null : /* @__PURE__ */ jsx("span", { className: "armada-phase-card__stands", children: stands })
        ] }),
        says === null || says === void 0 ? null : /* @__PURE__ */ jsx("p", { className: "armada-phase-card__said", children: says }),
        rows.length === 0 ? null : /* @__PURE__ */ jsx("ul", { className: "armada-phase-card__rows", children: rows.map((row, at) => {
          const where = row.state ?? verdictState(row.named) ?? state;
          const Mark = phaseGlyph(kind, where);
          return /* @__PURE__ */ jsxs("li", { className: "armada-phase-card__row", children: [
            /* @__PURE__ */ jsxs("span", { className: "armada-phase-card__row-line", children: [
              Mark === void 0 ? null : /* @__PURE__ */ jsx(
                Mark,
                {
                  size: ROW_GLYPH,
                  strokeWidth: STROKE$3,
                  className: "armada-phase-card__row-mark",
                  "data-state": where,
                  "aria-hidden": true
                }
              ),
              /* @__PURE__ */ jsx(
                "span",
                {
                  className: "armada-phase-card__row-label",
                  "data-mono": row.mono ? "true" : void 0,
                  children: row.label
                }
              ),
              row.result === void 0 ? null : /* @__PURE__ */ jsx("span", { className: "armada-phase-card__row-result", children: row.result })
            ] }),
            row.cited === void 0 ? null : /* @__PURE__ */ jsx("span", { className: "armada-phase-card__cited", children: row.cited })
          ] }, at);
        }) }),
        note === void 0 ? null : /* @__PURE__ */ jsx("div", { className: "armada-phase-card__note", children: note }),
        closes === null || closes === void 0 ? null : /* @__PURE__ */ jsx("p", { className: "armada-phase-card__detail", children: closes })
      ]
    }
  );
}
const meta$M = {
  title: "Compositions/Phase card",
  component: PhaseCard
};
const CHECKS$1 = {
  kind: "checks",
  name: "Checks",
  state: "current",
  stands: "1 of 2 · running",
  rows: [
    {
      label: "cargo build --workspace --locked",
      mono: true,
      state: "cleared",
      result: "exit 0 · 47s"
    },
    {
      label: "cargo nextest run --workspace",
      mono: true,
      state: "current",
      result: "running · 1m 04s"
    }
  ]
};
const CHECKS_FAILED = {
  kind: "checks",
  name: "Checks",
  state: "failed",
  stands: "1 of 2 failed",
  rows: [
    { label: "cargo nextest run --workspace", mono: true, state: "cleared", result: "exit 0" },
    { label: "cargo build --workspace", mono: true, state: "failed", result: "exit 101" }
  ]
};
const JUDGE = {
  kind: "judge",
  name: "Judge",
  state: "current",
  stands: "2 criteria",
  rows: [
    { label: "Selectors import without the store", state: "cleared" },
    { label: "No behaviour change in the reducer", state: "failed" }
  ]
};
const YOU = {
  kind: "human",
  name: "You",
  state: "waiting",
  stands: "waiting 2m 04s",
  note: "Approve, or send it back with a reason. Both are recorded on the Job."
};
const Checks = { args: CHECKS$1 };
const AChecksFailed = { args: CHECKS_FAILED };
const AChecksSkipped = {
  args: {
    kind: "checks",
    name: "Checks",
    state: "cleared",
    stands: "1 of 2 passed",
    rows: [
      {
        label: "cargo nextest run --workspace",
        mono: true,
        state: "cleared",
        result: "exit 0 · 47s"
      },
      {
        label: "pnpm -C packages/components build-storybook",
        mono: true,
        state: "ahead",
        result: "not run · no changed file is under packages/**, package.json, pnpm-lock.yaml, pnpm-workspace.yaml, apps/desktop/**, tsconfig.base.json"
      }
    ]
  }
};
const Judge = { args: JUDGE };
const JudgeRefusedAndWhy = {
  args: {
    kind: "judge",
    name: "Judge",
    state: "failed",
    stands: "1 of 2 refused",
    rows: [
      { label: "Selectors import without the store", named: "met", result: "met" },
      {
        label: "No behaviour change in the reducer",
        named: "not_met",
        result: "not met",
        cited: "packages/settings/src/reducer.ts:88 — the SETTINGS_RESET branch now clears manifests as well as columns, which it did not before this step."
      }
    ]
  }
};
const You = { args: YOU };
const FloatingOffTheStrip = {
  args: { ...CHECKS$1, floating: true },
  decorators: [
    (Story) => /* @__PURE__ */ jsx("div", { style: { padding: "var(--space-4) 0" }, children: /* @__PURE__ */ jsx(Story, {}) })
  ]
};
const TheThree = {
  render: () => /* @__PURE__ */ jsxs(
    "div",
    {
      style: {
        display: "grid",
        gridTemplateColumns: "repeat(3, minmax(0, 1fr))",
        gap: "var(--space-6)",
        alignItems: "start"
      },
      children: [
        /* @__PURE__ */ jsx(PhaseCard, { ...CHECKS_FAILED }),
        /* @__PURE__ */ jsx(PhaseCard, { ...JUDGE }),
        /* @__PURE__ */ jsx(PhaseCard, { ...YOU })
      ]
    }
  )
};
const __vite_glob_0_23 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AChecksFailed,
  AChecksSkipped,
  Checks,
  FloatingOffTheStrip,
  Judge,
  JudgeRefusedAndWhy,
  TheThree,
  You,
  default: meta$M
}, Symbol.toStringTag, { value: "Module" }));
const GLYPH$2 = 12;
const STROKE$2 = 2;
function PhaseStrip({
  stages,
  label: label2 = "Where this step is",
  note,
  pinnedId,
  pinnedStage,
  onPin
}) {
  const [held, setHeld] = useState(pinnedId ?? null);
  const [hovered, setHovered] = useState(null);
  const controlled = pinnedStage !== void 0;
  const pinned = controlled ? pinnedStage : held;
  const panelId = useId();
  const open2 = pinned ?? hovered;
  const shown = stages.find((stage) => stage.id === open2) ?? null;
  const pin = useCallback(
    (stageId) => {
      const next = pinned === stageId ? null : stageId;
      if (!controlled) setHeld(next);
      onPin?.(next);
    },
    [controlled, onPin, pinned]
  );
  useEffect(() => {
    if (pinned === null) return;
    function onKey(event) {
      if (event.key !== "Escape") return;
      if (!controlled) setHeld(null);
      onPin?.(null);
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [controlled, pinned, onPin]);
  return /* @__PURE__ */ jsxs("section", { className: "armada-phases", children: [
    label2 === void 0 ? null : /* @__PURE__ */ jsx("span", { className: "armada-phases__label", children: label2 }),
    /* @__PURE__ */ jsx("ol", { className: "armada-phases__strip", children: stages.map((stage, at) => {
      const kind = stage.kind ?? "phase";
      const Mark = phaseGlyph(kind, stage.state);
      const opens = stage.opens ?? kind !== "phase";
      const isOpen = open2 === stage.id;
      const align = at * 2 >= stages.length ? "end" : "start";
      const chip = /* @__PURE__ */ jsxs(Fragment, { children: [
        Mark === void 0 ? null : /* @__PURE__ */ jsx(Mark, { size: GLYPH$2, strokeWidth: STROKE$2, "aria-hidden": true }),
        stage.label
      ] });
      return /* @__PURE__ */ jsxs("li", { className: "armada-phases__stage", children: [
        at === 0 ? null : /* @__PURE__ */ jsx("span", { className: "armada-phases__conn", "aria-hidden": true }),
        opens ? /* @__PURE__ */ jsx(
          "button",
          {
            type: "button",
            className: "armada-phases__control",
            "data-state": stage.state,
            "data-kind": kind,
            "data-open": isOpen || void 0,
            "data-pinned": pinned === stage.id || void 0,
            "aria-expanded": isOpen,
            "aria-controls": panelId,
            onMouseEnter: () => setHovered(stage.id),
            onMouseLeave: () => setHovered((was) => was === stage.id ? null : was),
            onFocus: () => setHovered(stage.id),
            onBlur: () => setHovered((was) => was === stage.id ? null : was),
            onClick: () => pin(stage.id),
            children: chip
          }
        ) : /* @__PURE__ */ jsx("span", { className: "armada-phases__control", "data-state": stage.state, "data-kind": kind, children: chip }),
        isOpen && shown !== null ? /* @__PURE__ */ jsx("div", { className: "armada-phases__pop", "data-align": align, id: panelId, role: "dialog", children: /* @__PURE__ */ jsx(
          PhaseCard,
          {
            floating: true,
            align,
            kind,
            name: shown.label,
            state: shown.state,
            stands: shown.stands,
            said: shown.said,
            rows: shown.rows,
            note: shown.cardNote,
            detail: shown.detail
          }
        ) }) : null
      ] }, stage.id);
    }) }),
    note === void 0 ? null : /* @__PURE__ */ jsx("p", { className: "armada-phases__note", children: note })
  ] });
}
const meta$L = {
  title: "Compositions/Phase strip",
  component: PhaseStrip,
  decorators: [
    // The panel the strip lives in. Every tint on it is picked against
    // --bg-raised, and the card that opens off it lands on the same ground.
    (Story) => /* @__PURE__ */ jsx(
      "div",
      {
        style: {
          width: "calc(var(--space-12) * 16)",
          padding: "var(--space-4) var(--space-6)",
          borderRadius: "var(--radius-md)",
          border: "var(--border-width) solid var(--border-default)",
          background: "var(--bg-raised)",
          // The card opens beneath the strip and has to be visible in the
          // story, not clipped by the frame around it.
          minHeight: "calc(var(--space-12) * 8)"
        },
        children: /* @__PURE__ */ jsx(Story, {})
      }
    )
  ]
};
const CHECKS = [
  {
    label: "cargo build --workspace --locked",
    mono: true,
    state: "cleared",
    result: "exit 0 · 47s"
  },
  {
    label: "cargo nextest run --workspace",
    mono: true,
    state: "current",
    result: "running · 1m 04s"
  }
];
const CRITERIA = [
  { label: "Selectors import without the store", state: "cleared" },
  { label: "No behaviour change in the reducer", state: "cleared" }
];
const AllFourStates = {
  args: {
    note: "The Drone is working. Nothing has been submitted, so no gate has been asked anything yet.",
    stages: [
      { id: "instructed", label: "Instructed", state: "cleared" },
      { id: "working", label: "Working", state: "current" },
      { id: "submitted", label: "Submitted", state: "ahead" },
      { id: "checks", label: "build, test", kind: "checks", state: "ahead", rows: CHECKS },
      {
        id: "judge",
        label: "Judge · 2 criteria",
        kind: "judge",
        state: "ahead",
        rows: CRITERIA
      },
      {
        id: "you",
        label: "You",
        kind: "human",
        state: "ahead",
        cardNote: "Approve, or send it back with a reason. Both are recorded on the Job."
      }
    ]
  }
};
const WithYouPresent = {
  args: {
    note: "Every Check passed and both criteria were met. This step is waiting on you.",
    stages: [
      { id: "instructed", label: "Instructed", state: "cleared" },
      { id: "working", label: "Working", state: "cleared" },
      { id: "submitted", label: "Submitted", state: "cleared" },
      {
        id: "checks",
        label: "build, test",
        kind: "checks",
        state: "cleared",
        stands: "2 of 2 passed",
        rows: [
          { label: "cargo build --workspace --locked", mono: true, state: "cleared", result: "exit 0 · 47s" },
          { label: "cargo nextest run --workspace", mono: true, state: "cleared", result: "exit 0 · 1m 21s" }
        ]
      },
      {
        id: "judge",
        label: "Judge · 2 of 2 met",
        kind: "judge",
        state: "cleared",
        stands: "2 of 2 met",
        rows: CRITERIA
      },
      {
        id: "you",
        label: "You",
        kind: "human",
        state: "waiting",
        stands: "waiting 2m 04s",
        cardNote: "Approve, or send it back with a reason. Both are recorded on the Job."
      }
    ]
  }
};
const AJudgeRefused = {
  args: {
    pinnedId: "judge",
    note: "The commands were fine and one criterion was refused. Nothing past it ran.",
    stages: [
      { id: "instructed", label: "Instructed", state: "cleared" },
      { id: "working", label: "Working", state: "cleared" },
      { id: "submitted", label: "Submitted", state: "cleared" },
      {
        id: "checks",
        label: "build, test",
        kind: "checks",
        state: "cleared",
        stands: "2 of 2 passed",
        rows: CHECKS.map((row) => ({ ...row, state: "cleared", result: "exit 0" }))
      },
      {
        id: "judge",
        label: "Judge · 1 of 2 refused",
        kind: "judge",
        state: "failed",
        stands: "1 of 2 refused",
        rows: [
          { label: "Selectors import without the store", state: "cleared", result: "met" },
          {
            label: "No behaviour change in the reducer",
            state: "failed",
            result: "not met",
            cited: "packages/settings/src/reducer.ts:88 — the SETTINGS_RESET branch now clears manifests as well as columns, which it did not before this step."
          }
        ]
      },
      { id: "you", label: "You", kind: "human", state: "ahead" }
    ]
  }
};
const ACheckFailed$1 = {
  args: {
    pinnedId: "checks",
    note: "A command exited non-zero. The Drone is repairing it; no model has been asked anything.",
    stages: [
      { id: "instructed", label: "Instructed", state: "cleared" },
      { id: "working", label: "Working", state: "cleared" },
      { id: "submitted", label: "Submitted", state: "cleared" },
      {
        id: "checks",
        label: "test failed · fixing",
        kind: "checks",
        state: "failed",
        stands: "1 of 2 failed",
        rows: [
          { label: "cargo build --workspace", mono: true, state: "cleared", result: "exit 0" },
          { label: "cargo nextest run --workspace", mono: true, state: "failed", result: "exit 101" }
        ]
      },
      { id: "judge", label: "Judge · 2 criteria", kind: "judge", state: "ahead", rows: CRITERIA },
      { id: "you", label: "You", kind: "human", state: "ahead" }
    ]
  }
};
const OpenedNearTheTrailingEdge = {
  args: {
    ...WithYouPresent.args,
    pinnedId: "you"
  }
};
const NoGateAtAll = {
  args: {
    note: "This step declares no command and asks no model. Its evidence is what advances it.",
    stages: [
      { id: "instructed", label: "Instructed", state: "cleared" },
      { id: "working", label: "Working", state: "current" },
      { id: "submitted", label: "Submitted", state: "ahead" },
      { id: "you", label: "You", kind: "human", state: "ahead" }
    ]
  }
};
const HeldByTheCaller$2 = {
  args: {
    ...WithYouPresent.args,
    pinnedStage: "judge"
  }
};
const __vite_glob_0_24 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  ACheckFailed: ACheckFailed$1,
  AJudgeRefused,
  AllFourStates,
  HeldByTheCaller: HeldByTheCaller$2,
  NoGateAtAll,
  OpenedNearTheTrailingEdge,
  WithYouPresent,
  default: meta$L
}, Symbol.toStringTag, { value: "Module" }));
function Separator({
  className,
  orientation = "horizontal",
  decorative = true,
  ...rest
}) {
  return /* @__PURE__ */ jsx(
    "div",
    {
      className: className ? `armada-separator ${className}` : "armada-separator",
      "data-orientation": orientation,
      role: decorative ? "none" : "separator",
      "aria-orientation": decorative ? void 0 : orientation,
      ...rest
    }
  );
}
function ReviewDecision({
  note,
  onNote,
  onApprove,
  onRequestChanges,
  onReject,
  disabled = false,
  disabledNote,
  noteLabel = "What should change",
  keptNote = "Approving takes the work. Requesting changes sends this note to the drone as a turn — it keeps the worktree and the step, and comes back running.",
  rejectNote = "Rejecting is a verdict on the work and the job ends there. The drone is stopped and nothing resumes it. Its branch stays where the drone left it.",
  approveLabel = "Approve the work",
  requestChangesLabel = "Request changes",
  rejectLabel = "Reject the work"
}) {
  const blank = note.trim() === "";
  return /* @__PURE__ */ jsxs("div", { className: "armada-decision", children: [
    /* @__PURE__ */ jsx(
      Textarea,
      {
        label: noteLabel,
        rows: 4,
        value: note,
        disabled,
        onChange: (event) => onNote(event.target.value)
      }
    ),
    /* @__PURE__ */ jsxs("div", { className: "armada-decision__kept", children: [
      /* @__PURE__ */ jsx(Button, { variant: "primary", disabled, onClick: onApprove, children: approveLabel }),
      /* @__PURE__ */ jsx(
        Button,
        {
          variant: "secondary",
          disabled: disabled || blank,
          onClick: onRequestChanges,
          children: requestChangesLabel
        }
      )
    ] }),
    /* @__PURE__ */ jsx("p", { className: "armada-decision__said", children: keptNote }),
    /* @__PURE__ */ jsx(Separator, { decorative: false, className: "armada-decision__rule" }),
    /* @__PURE__ */ jsxs("div", { className: "armada-decision__terminal", children: [
      /* @__PURE__ */ jsx(Button, { variant: "destructive", disabled, onClick: onReject, children: rejectLabel }),
      /* @__PURE__ */ jsx("p", { className: "armada-decision__said", "data-terminal": true, children: rejectNote })
    ] }),
    disabled && disabledNote !== void 0 ? /* @__PURE__ */ jsx("p", { className: "armada-decision__said", role: "note", children: disabledNote }) : null
  ] });
}
const meta$K = {
  title: "Compositions/Review decision",
  component: ReviewDecision,
  args: {
    note: "",
    onNote: () => {
    },
    onApprove: () => {
    },
    onRequestChanges: () => {
    },
    onReject: () => {
    }
  }
};
const NothingWrittenYet = {
  args: { note: "" }
};
const ANoteWritten = {
  args: {
    note: "The gate change is right, but AdvanceGate::HumanAlways is handled in gate.rs and not in config's loader, so a workflow declaring it is still refused at load. Add the arm there and a test that loads one."
  }
};
const ADecisionAlreadySent = {
  args: {
    note: "Add the arm in config's loader and a test that loads one.",
    disabled: true,
    disabledNote: "A decision on this job is already in flight. It was not sent twice."
  }
};
const NotConnectedToFleet = {
  args: {
    note: "",
    disabled: true,
    disabledNote: "Fleet is not connected, so nothing here can be sent."
  }
};
const __vite_glob_0_25 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  ADecisionAlreadySent,
  ANoteWritten,
  NotConnectedToFleet,
  NothingWrittenYet,
  default: meta$K
}, Symbol.toStringTag, { value: "Module" }));
const NO_DURATION = "—";
const GLYPH$1 = 12;
const STROKE$1 = 2;
function StepRow({
  label: label2,
  labelIsAnIdentifier,
  activity,
  status,
  elapsed,
  ordinal,
  selected,
  hovered,
  open: open2 = false,
  onToggle,
  onSelect,
  locked,
  lockedLabel,
  facts: facts2 = [],
  factsAbsent,
  pulsing = false,
  factsId
}) {
  const lockSays = lockedLabel ?? "Cannot be skipped, even on retry";
  return /* @__PURE__ */ jsxs("div", { className: "armada-srow-group", children: [
    /* @__PURE__ */ jsxs(
      "div",
      {
        className: "armada-srow",
        "data-activity": activity,
        "data-sel": selected || void 0,
        "data-hover": hovered || void 0,
        children: [
          onToggle === void 0 ? /* @__PURE__ */ jsx("span", { className: "armada-srow__chevron", "aria-hidden": true }) : /* @__PURE__ */ jsx(
            "button",
            {
              type: "button",
              className: "armada-srow__chevron",
              "aria-expanded": open2,
              "aria-controls": factsId,
              "aria-label": open2 ? "Close this step's facts" : "Open this step's facts",
              onClick: onToggle,
              children: open2 ? /* @__PURE__ */ jsx(ChevronDown, { size: GLYPH$1, strokeWidth: STROKE$1, "aria-hidden": true }) : /* @__PURE__ */ jsx(ChevronRight, { size: GLYPH$1, strokeWidth: STROKE$1, "aria-hidden": true })
            }
          ),
          /* @__PURE__ */ jsx(
            StepActivityMark,
            {
              activity,
              label: status ?? activity,
              ordinal,
              pulsing
            }
          ),
          onSelect === void 0 ? /* @__PURE__ */ jsxs("span", { className: "armada-srow__name", "data-identifier": labelIsAnIdentifier || void 0, children: [
            label2,
            locked ? /* @__PURE__ */ jsx(StepRowLock, { says: lockSays }) : null
          ] }) : /* @__PURE__ */ jsxs(
            "button",
            {
              type: "button",
              className: "armada-srow__name",
              "data-identifier": labelIsAnIdentifier || void 0,
              "aria-current": selected ? "true" : void 0,
              onClick: onSelect,
              children: [
                label2,
                locked ? /* @__PURE__ */ jsx(StepRowLock, { says: lockSays }) : null
              ]
            }
          ),
          /* @__PURE__ */ jsx("span", { className: "armada-srow__dur", children: elapsed ?? NO_DURATION })
        ]
      }
    ),
    /* @__PURE__ */ jsx("div", { className: "armada-srow__facts", id: factsId, hidden: !open2, children: facts2.length === 0 ? /* @__PURE__ */ jsx("p", { className: "armada-srow__absent", children: factsAbsent ?? "Nothing was recorded against this step." }) : facts2.map((fact, at) => /* @__PURE__ */ jsxs("div", { className: "armada-srow__fact", children: [
      /* @__PURE__ */ jsx("span", { className: "armada-srow__fact-label", children: fact.label }),
      fact.value
    ] }, at)) })
  ] });
}
function StepRowLock({ says }) {
  return /* @__PURE__ */ jsxs("span", { className: "armada-srow__lock", title: says, children: [
    /* @__PURE__ */ jsx(Lock, { size: GLYPH$1, strokeWidth: STROKE$1, "aria-hidden": true }),
    /* @__PURE__ */ jsx("span", { className: "armada-srow__sr", children: says })
  ] });
}
function RunTree({
  steps,
  pulsing = false,
  onSelect,
  openSteps,
  onOpen,
  onCopied
}) {
  const [held, setHeld] = useState(
    () => new Set(steps.filter((step) => step.factsOpen).map((step) => step.id))
  );
  const controlled = openSteps !== void 0;
  const open2 = useMemo(
    () => openSteps === void 0 ? held : new Set(openSteps),
    [openSteps, held]
  );
  const toggle = useCallback(
    (stepId) => {
      const next = !open2.has(stepId);
      if (!controlled) {
        setHeld((was) => {
          const now = new Set(was);
          if (next) now.add(stepId);
          else now.delete(stepId);
          return now;
        });
      }
      onOpen?.(stepId, next);
    },
    [controlled, onOpen, open2]
  );
  return /* @__PURE__ */ jsx("ol", { className: "armada-run", children: steps.map((step) => /* @__PURE__ */ jsx("li", { className: "armada-run__step", children: /* @__PURE__ */ jsx(
    StepRow,
    {
      label: step.label,
      labelIsAnIdentifier: step.labelIsAnIdentifier,
      activity: step.activity,
      status: step.status ?? step.activity,
      elapsed: step.elapsed,
      selected: step.current,
      open: open2.has(step.id),
      onToggle: () => toggle(step.id),
      onSelect: onSelect === void 0 ? void 0 : () => onSelect(step.id),
      locked: step.locked,
      lockedLabel: step.lockedLabel,
      pulsing: pulsing && (step.current ?? false),
      factsId: `armada-run-facts-${step.id}`,
      factsAbsent: step.factsAbsent,
      facts: (step.facts ?? []).map((fact) => ({
        label: fact.label,
        value: /* @__PURE__ */ jsxs(Fragment, { children: [
          fact.value === void 0 ? null : /* @__PURE__ */ jsx(FactChip, { named: fact.named, children: fact.value }),
          (fact.paths ?? []).map((path, p) => /* @__PURE__ */ jsx(
            PathChip,
            {
              directory: path.directory,
              basename: path.basename,
              note: path.note,
              onCopy: onCopied
            },
            p
          ))
        ] })
      }))
    }
  ) }, step.id)) });
}
const meta$J = {
  title: "Compositions/Run tree",
  component: RunTree
};
const ARTIFACTS = ".armada/artifacts/job_2d90bb/";
const BUG = [
  {
    id: "repro",
    label: "Reproduction",
    activity: "advanced",
    elapsed: "1m 12s",
    status: "advanced",
    facts: [
      {
        label: "Produced",
        paths: [{ directory: "packages/settings/test/", basename: "useColumnSelectors.test.ts" }]
      },
      { label: "Cleared", value: "test", named: "passed" }
    ]
  },
  {
    id: "root_cause",
    label: "Root cause",
    activity: "advanced",
    elapsed: "3m 40s",
    status: "advanced",
    facts: [
      { label: "Attempt 1", value: "refused", named: "refused" },
      { label: "Attempt 2", value: "advanced", named: "advanced" },
      { label: "Produced", paths: [{ directory: ARTIFACTS, basename: "root_cause.md" }] }
    ]
  },
  {
    id: "fix",
    label: "Fix",
    activity: "running",
    elapsed: "6m 11s",
    status: "running",
    current: true,
    factsOpen: true,
    facts: [
      { label: "Produced", value: "3 files · +94 −31" },
      { label: "Checks", value: "not run" },
      { label: "Judge", value: "2 criteria" }
    ]
  },
  {
    id: "regression_verify",
    label: "Regression check",
    activity: "not_started",
    facts: [],
    factsAbsent: "This step has not run, so it has produced nothing."
  },
  {
    id: "consumers",
    label: "Check the consumers still compile",
    activity: "not_started",
    facts: [],
    factsAbsent: "This step has not run, so it has produced nothing."
  },
  {
    id: "land",
    label: "Land",
    activity: "not_started",
    locked: true,
    facts: [],
    factsAbsent: "This step has not run, so it has produced nothing."
  }
];
const Running$7 = {
  args: { steps: BUG, pulsing: true, onSelect: () => {
  } }
};
const EverythingClosed = {
  args: {
    steps: BUG.map((step) => ({ ...step, factsOpen: false })),
    pulsing: true,
    onSelect: () => {
    }
  }
};
const WaitingOnYou$2 = {
  args: {
    pulsing: true,
    onSelect: () => {
    },
    steps: [
      { id: "fix", label: "Fix", activity: "advanced", elapsed: "6m 11s", status: "advanced", facts: [] },
      {
        id: "regression_verify",
        label: "Regression check",
        activity: "awaiting_human",
        elapsed: "2m 04s",
        status: "waiting on you",
        current: true,
        factsOpen: true,
        facts: [
          { label: "Checks", value: "2 of 2 passed", named: "passed" },
          { label: "Judge", value: "2 of 2 met", named: "passed" },
          { label: "Waiting", value: "on you · 2m 04s" }
        ]
      }
    ]
  }
};
const Stopped$2 = {
  args: {
    onSelect: () => {
    },
    steps: [
      { id: "root_cause", label: "Root cause", activity: "advanced", elapsed: "3m 40s", status: "advanced", facts: [] },
      {
        id: "fix",
        label: "Fix",
        activity: "stopped",
        elapsed: "14m 22s",
        status: "retries spent",
        current: true,
        factsOpen: true,
        facts: [
          { label: "Attempt 1", value: "refused · reducer changed", named: "refused" },
          { label: "Attempt 2", value: "refused · same criterion", named: "refused" },
          { label: "Attempt 3", value: "refused · same criterion", named: "refused" },
          { label: "Held", value: "retries spent · waiting on you" }
        ]
      }
    ]
  }
};
const Failed$6 = {
  args: {
    onSelect: () => {
    },
    steps: [
      { id: "fix", label: "Fix", activity: "advanced", elapsed: "6m 11s", status: "advanced", facts: [] },
      {
        id: "regression_verify",
        label: "Regression check",
        activity: "failed",
        elapsed: "2m 51s",
        status: "failed",
        current: true,
        factsOpen: true,
        facts: [
          { label: "Checks", value: "test failed · exit 101", named: "failed" },
          { label: "Judge", value: "not reached" },
          { label: "Job", value: "completed_failed", named: "failed" }
        ]
      }
    ]
  }
};
const HardPrerequisite$1 = {
  args: {
    onSelect: () => {
    },
    steps: [
      { id: "land", label: "Land", activity: "not_started", locked: true, facts: [] },
      {
        id: "announce",
        label: "Announce",
        activity: "not_started",
        locked: true,
        lockedLabel: "Cannot be skipped, even on retry",
        facts: []
      }
    ]
  }
};
const NoHumanName = {
  args: {
    onSelect: () => {
    },
    steps: [
      {
        id: "regression_verify",
        label: "regression_verify",
        labelIsAnIdentifier: true,
        activity: "running",
        elapsed: "2m 04s",
        status: "running",
        current: true,
        facts: [{ label: "Checks", value: "1 of 2 · running" }]
      }
    ]
  }
};
const ReadOnly$1 = {
  args: { steps: BUG }
};
const HeldByTheCaller$1 = {
  args: { steps: BUG, onSelect: () => {
  }, openSteps: ["root_cause", "fix"] }
};
const __vite_glob_0_26 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  EverythingClosed,
  Failed: Failed$6,
  HardPrerequisite: HardPrerequisite$1,
  HeldByTheCaller: HeldByTheCaller$1,
  NoHumanName,
  ReadOnly: ReadOnly$1,
  Running: Running$7,
  Stopped: Stopped$2,
  WaitingOnYou: WaitingOnYou$2,
  default: meta$J
}, Symbol.toStringTag, { value: "Module" }));
const NAV_ICON = 16;
const NAV_STROKE = 2;
function Item({
  item,
  active,
  collapsed,
  onSelect
}) {
  return /* @__PURE__ */ jsxs(
    "button",
    {
      type: "button",
      className: "armada-sidebar__item",
      "data-active": active || void 0,
      "aria-current": active ? "page" : void 0,
      "aria-label": collapsed ? item.label : void 0,
      onClick: () => onSelect?.(item.id),
      children: [
        /* @__PURE__ */ jsx(item.icon, { size: NAV_ICON, strokeWidth: NAV_STROKE, "aria-hidden": true }),
        collapsed ? null : /* @__PURE__ */ jsx("span", { className: "armada-sidebar__label", children: item.label }),
        !collapsed && item.count !== void 0 ? /* @__PURE__ */ jsx("span", { className: "armada-sidebar__count", children: item.count }) : null
      ]
    }
  );
}
function Sidebar({
  surfaces: surfaces2,
  sibling,
  sectionLabel = "Bridge",
  activeId,
  appName,
  header,
  collapsed = false,
  width,
  onSelect
}) {
  return /* @__PURE__ */ jsxs(
    "nav",
    {
      className: "armada-sidebar",
      "data-collapsed": collapsed || void 0,
      style: { width: collapsed ? "var(--sidebar-rail)" : width ?? "var(--sidebar-default)" },
      children: [
        appName ? /* @__PURE__ */ jsx("div", { className: "armada-sidebar__chrome", children: appName }) : null,
        !collapsed && header ? /* @__PURE__ */ jsx("div", { className: "armada-sidebar__header", children: header }) : null,
        !collapsed && sectionLabel ? /* @__PURE__ */ jsx("div", { className: "armada-sidebar__section", children: sectionLabel }) : null,
        /* @__PURE__ */ jsx("div", { className: "armada-sidebar__group", children: surfaces2.map((item) => /* @__PURE__ */ jsx(
          Item,
          {
            item,
            active: item.id === activeId,
            collapsed,
            onSelect
          },
          item.id
        )) }),
        sibling ? /* @__PURE__ */ jsxs(Fragment, { children: [
          /* @__PURE__ */ jsx(Separator, { decorative: false, className: "armada-sidebar__rule" }),
          /* @__PURE__ */ jsx("div", { className: "armada-sidebar__group", children: /* @__PURE__ */ jsx(
            Item,
            {
              item: sibling,
              active: sibling.id === activeId,
              collapsed,
              onSelect
            }
          ) })
        ] }) : null
      ]
    }
  );
}
const meta$I = {
  title: "Compositions/Sidebar",
  component: Sidebar
};
const surfaces = [
  { id: "board", label: "Job Board", icon: ClipboardList },
  { id: "active", label: "Active jobs", icon: Activity, count: 6 },
  { id: "alerts", label: "Alerts", icon: Bell },
  { id: "reviews", label: "Reviews", icon: Eye },
  { id: "feed", label: "Activity feed", icon: ScrollText },
  { id: "doctor", label: "Doctor", icon: Stethoscope },
  { id: "manifest", label: "Manifest", icon: FileCog }
];
const helm = { id: "helm", label: "Helm", icon: MessageSquare };
const Expanded = {
  args: { surfaces, sibling: helm, activeId: "active", appName: "Armada" }
};
const HelmActive = {
  args: { surfaces, sibling: helm, activeId: "helm", appName: "Armada" }
};
const CollapsedRail$1 = {
  args: { surfaces, sibling: helm, activeId: "active", appName: "Armada", collapsed: true }
};
const AtMinimumWidth = {
  args: { surfaces, sibling: helm, activeId: "active", appName: "Armada", width: "var(--sidebar-min)" }
};
const AtMaximumWidth = {
  args: { surfaces, sibling: helm, activeId: "active", appName: "Armada", width: "var(--sidebar-max)" }
};
const M1OneSurface = {
  args: {
    surfaces: [{ id: "active", label: "Active jobs", icon: Activity, count: 6 }],
    activeId: "active",
    appName: "Armada",
    header: /* @__PURE__ */ jsx(Select, { "aria-label": "Project", children: /* @__PURE__ */ jsx("option", { children: "armada" }) })
  }
};
const FlatForContrast = {
  args: { surfaces, activeId: "active", sectionLabel: void 0, appName: "Armada" }
};
const __vite_glob_0_27 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AtMaximumWidth,
  AtMinimumWidth,
  CollapsedRail: CollapsedRail$1,
  Expanded,
  FlatForContrast,
  HelmActive,
  M1OneSurface,
  default: meta$I
}, Symbol.toStringTag, { value: "Module" }));
function count(n, one2, many) {
  return `${n} ${n === 1 ? one2 : many}`;
}
function StatusBar({
  fleet,
  fleetLabel,
  detail,
  advice,
  items,
  spend,
  escalations,
  approvals
}) {
  return /* @__PURE__ */ jsxs("div", { className: "armada-status-bar", role: "status", "data-fleet": fleet, children: [
    /* @__PURE__ */ jsxs("span", { className: "armada-status-bar__fleet", children: [
      /* @__PURE__ */ jsx("span", { className: "armada-status-bar__dot", "aria-hidden": true }),
      fleetLabel
    ] }),
    detail ? /* @__PURE__ */ jsx("span", { className: "armada-status-bar__mono", children: detail }) : null,
    advice ? /* @__PURE__ */ jsx("span", { className: "armada-status-bar__advice", children: advice }) : null,
    items?.map((item, i) => /* @__PURE__ */ jsx("span", { className: "armada-status-bar__item", children: item }, i)),
    escalations ? /* @__PURE__ */ jsx("span", { className: "armada-status-bar__count", "data-kind": "escalated", children: count(escalations, "escalation", "escalations") }) : null,
    approvals ? /* @__PURE__ */ jsx("span", { className: "armada-status-bar__count", "data-kind": "awaiting-review", children: count(approvals, "approval", "approvals") }) : null,
    spend ? /* @__PURE__ */ jsx("span", { className: "armada-status-bar__spend", children: spend }) : null
  ] });
}
const meta$H = {
  title: "Compositions/StatusBar",
  component: StatusBar
};
const FleetRunningIdle = {
  args: { fleet: "running", fleetLabel: "Fleet running" }
};
const FleetRunningPersonalMachine = {
  args: {
    fleet: "running",
    fleetLabel: "Fleet running",
    detail: "pid 4417 · port 7411 · 1 of 2 drones",
    items: ["3 jobs"],
    spend: "68% quota left"
  }
};
const FleetRunningWorkMachine = {
  args: {
    fleet: "running",
    fleetLabel: "Fleet running",
    detail: "pid 4417 · port 7411 · 1 of 2 drones",
    items: ["3 jobs"],
    spend: "~$2.40 of $20"
  }
};
const FleetNotRunning = {
  args: {
    fleet: "not-running",
    fleetLabel: "Fleet is not running",
    detail: "no runtime file at ~/.armada/fleet.json",
    advice: "Start it from the terminal."
  }
};
const FleetUnreachable = {
  args: {
    fleet: "unreachable",
    fleetLabel: "Fleet unreachable",
    detail: "pid 4417 alive on port 7411 · no response for 20s",
    advice: "The last job state read is 20s old."
  }
};
const WithBothCounts = {
  args: {
    fleet: "running",
    fleetLabel: "Fleet running",
    items: ["3 jobs"],
    escalations: 2,
    approvals: 4,
    spend: "68% quota left"
  }
};
const WithOneOfEach = {
  args: {
    fleet: "running",
    fleetLabel: "Fleet running",
    items: ["1 job"],
    escalations: 1,
    approvals: 1
  }
};
const WithEscalationsOnly = {
  args: {
    fleet: "running",
    fleetLabel: "Fleet running",
    items: ["3 jobs"],
    escalations: 2,
    spend: "~$2.40 of $20"
  }
};
const FleetRunningAtItsBound = {
  args: {
    fleet: "running",
    fleetLabel: "Fleet running",
    detail: "pid 4417 · port 7411 · 2 of 2 drones",
    items: ["4 jobs", "waiting on a free drone"]
  }
};
const FleetRunningShortOfDisk = {
  args: {
    fleet: "running",
    fleetLabel: "Fleet running",
    detail: "pid 4417 · port 7411 · 0 of 2 drones",
    items: ["4 jobs", "waiting on disk"]
  }
};
const AtTheItemCeiling = {
  args: {
    fleet: "running",
    fleetLabel: "Fleet running",
    detail: "pid 4417 · port 7411 · 1 of 2 drones",
    items: ["3 jobs"],
    escalations: 2,
    spend: "~$2.40 of $20"
  }
};
const __vite_glob_0_28 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AtTheItemCeiling,
  FleetNotRunning,
  FleetRunningAtItsBound,
  FleetRunningIdle,
  FleetRunningPersonalMachine,
  FleetRunningShortOfDisk,
  FleetRunningWorkMachine,
  FleetUnreachable,
  WithBothCounts,
  WithEscalationsOnly,
  WithOneOfEach,
  default: meta$H
}, Symbol.toStringTag, { value: "Module" }));
const meta$G = {
  title: "Compositions/Step activity mark",
  component: StepActivityMark
};
const NotStarted$2 = {
  args: { activity: "not_started", label: "Not started", ordinal: 3 }
};
const NotStartedWithNoOrdinal = {
  args: { activity: "not_started", label: "Not started" }
};
const Running$6 = {
  args: { activity: "running", label: "Running" }
};
const RunningPulsing$1 = {
  args: { activity: "running", label: "Running", pulsing: true }
};
const AwaitingHuman$1 = {
  args: { activity: "awaiting_human", label: "Waiting on you" }
};
const Retrying = {
  args: { activity: "retrying", label: "Retrying" }
};
const Advanced$1 = {
  args: { activity: "advanced", label: "Advanced" }
};
const Stopped$1 = {
  args: { activity: "stopped", label: "Stopped" }
};
const Killed$5 = {
  args: { activity: "killed", label: "Killed" }
};
const Failed$5 = {
  args: { activity: "failed", label: "Failed a check" }
};
const EveryValue = {
  render: () => {
    const values = [
      ["not_started", "Not started", 3],
      ["running", "Running"],
      ["awaiting_human", "Waiting on you"],
      ["retrying", "Retrying"],
      ["advanced", "Advanced"],
      ["stopped", "Stopped"],
      ["killed", "Killed"],
      ["failed", "Failed a check"]
    ];
    return /* @__PURE__ */ jsx("div", { style: { display: "flex", flexDirection: "column", gap: "var(--space-2)" }, children: values.map(([activity, label2, ordinal]) => /* @__PURE__ */ jsxs("div", { style: { display: "flex", alignItems: "center", gap: "var(--space-2)" }, children: [
      /* @__PURE__ */ jsx(StepActivityMark, { activity, label: label2, ordinal }),
      /* @__PURE__ */ jsx("span", { style: { fontFamily: "var(--font-mono)", fontSize: "var(--text-2xs)", color: "var(--fg-subtle)" }, children: activity })
    ] }, activity)) });
  }
};
const __vite_glob_0_29 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Advanced: Advanced$1,
  AwaitingHuman: AwaitingHuman$1,
  EveryValue,
  Failed: Failed$5,
  Killed: Killed$5,
  NotStarted: NotStarted$2,
  NotStartedWithNoOrdinal,
  Retrying,
  Running: Running$6,
  RunningPulsing: RunningPulsing$1,
  Stopped: Stopped$1,
  default: meta$G
}, Symbol.toStringTag, { value: "Module" }));
const meta$F = {
  title: "Compositions/Step bar",
  component: StepBar
};
const NotStarted$1 = {
  args: { total: 4, current: 0, label: "Not started, 4 steps" }
};
const Running$5 = {
  args: { total: 4, current: 2, activity: "running", label: "Step 2 of 4" }
};
const RunningLongWorkflow = {
  args: { total: 7, current: 5, activity: "running", label: "Step 5 of 7" }
};
const AwaitingHuman = {
  args: { total: 4, current: 3, activity: "awaiting_human", label: "Step 3 of 4" }
};
const Failed$4 = {
  args: { total: 4, current: 3, activity: "failed", label: "Step 3 of 4" }
};
const Killed$4 = {
  args: { total: 4, current: 2, activity: "killed", label: "Step 2 of 4" }
};
const AllAdvanced = {
  args: { total: 4, current: 5, activity: "advanced", label: "All 4 of 4 steps advanced" }
};
const RunningNeverPulses = {
  render: () => /* @__PURE__ */ jsxs("div", { style: { display: "flex", flexDirection: "column", gap: "var(--space-2)", width: "var(--sidebar-rail)" }, children: [
    /* @__PURE__ */ jsx(StepBar, { total: 4, current: 2, activity: "running", label: "Step 2 of 4" }),
    /* @__PURE__ */ jsx(StepBar, { total: 4, current: 3, activity: "running", label: "Step 3 of 4" })
  ] })
};
const __vite_glob_0_30 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AllAdvanced,
  AwaitingHuman,
  Failed: Failed$4,
  Killed: Killed$4,
  NotStarted: NotStarted$1,
  Running: Running$5,
  RunningLongWorkflow,
  RunningNeverPulses,
  default: meta$F
}, Symbol.toStringTag, { value: "Module" }));
const meta$E = {
  title: "Compositions/Step row",
  component: StepRow,
  decorators: [
    // The well the run sits in. A row drawn on the canvas would be judged
    // against the wrong ground: every surface value on it is picked against
    // --bg-sunken.
    (Story) => /* @__PURE__ */ jsx(
      "div",
      {
        style: {
          width: "calc(var(--space-12) * 8)",
          padding: "var(--space-2)",
          borderRadius: "var(--radius-md)",
          background: "var(--bg-sunken)"
        },
        children: /* @__PURE__ */ jsx(Story, {})
      }
    )
  ]
};
const Selected = {
  args: {
    label: "Fix",
    activity: "running",
    status: "running",
    elapsed: "6m 11s",
    selected: true,
    open: true,
    factsId: "facts-selected",
    onToggle: () => {
    },
    onSelect: () => {
    },
    facts: [
      { label: "Produced", value: /* @__PURE__ */ jsx(FactChip, { children: "3 files · +94 −31" }) },
      { label: "Checks", value: /* @__PURE__ */ jsx(FactChip, { children: "not run" }) },
      { label: "Judge", value: /* @__PURE__ */ jsx(FactChip, { children: "2 criteria" }) }
    ]
  }
};
const Hover$1 = {
  args: {
    label: "Root cause",
    activity: "advanced",
    status: "advanced",
    elapsed: "3m 40s",
    hovered: true,
    factsId: "facts-hover",
    onToggle: () => {
    },
    onSelect: () => {
    }
  }
};
const Advanced = {
  args: {
    label: "Reproduction",
    activity: "advanced",
    status: "advanced",
    elapsed: "1m 12s",
    open: true,
    factsId: "facts-advanced",
    onToggle: () => {
    },
    onSelect: () => {
    },
    facts: [
      {
        label: "Produced",
        value: /* @__PURE__ */ jsx(
          PathChip,
          {
            directory: "packages/settings/test/",
            basename: "useColumnSelectors.test.ts"
          }
        )
      },
      { label: "Cleared", value: /* @__PURE__ */ jsx(FactChip, { named: "passed", children: "test" }) }
    ]
  }
};
const Running$4 = {
  args: {
    label: "Fix",
    activity: "running",
    status: "running",
    elapsed: "6m 11s",
    pulsing: true,
    factsId: "facts-running",
    onToggle: () => {
    },
    onSelect: () => {
    }
  }
};
const Unreached = {
  args: {
    label: "Regression check",
    activity: "not_started",
    status: "not started",
    factsId: "facts-unreached",
    onSelect: () => {
    }
  }
};
const Failed$3 = {
  args: {
    label: "Regression check",
    activity: "failed",
    status: "failed",
    elapsed: "2m 51s",
    selected: true,
    open: true,
    factsId: "facts-failed",
    onToggle: () => {
    },
    onSelect: () => {
    },
    facts: [
      { label: "Checks", value: /* @__PURE__ */ jsx(FactChip, { named: "failed", children: "test failed · exit 101" }) },
      { label: "Judge", value: /* @__PURE__ */ jsx(FactChip, { children: "not reached" }) },
      { label: "Job", value: /* @__PURE__ */ jsx(FactChip, { named: "failed", children: "completed_failed" }) }
    ]
  }
};
const Held = {
  args: {
    label: "Fix",
    activity: "stopped",
    status: "retries spent",
    elapsed: "14m 22s",
    selected: true,
    open: true,
    factsId: "facts-held",
    onToggle: () => {
    },
    onSelect: () => {
    },
    facts: [
      { label: "Attempt 1", value: /* @__PURE__ */ jsx(FactChip, { named: "refused", children: "refused · reducer changed" }) },
      { label: "Attempt 2", value: /* @__PURE__ */ jsx(FactChip, { named: "refused", children: "refused · same criterion" }) },
      { label: "Attempt 3", value: /* @__PURE__ */ jsx(FactChip, { named: "refused", children: "refused · same criterion" }) },
      { label: "Held", value: /* @__PURE__ */ jsx(FactChip, { children: "retries spent · waiting on you" }) }
    ]
  }
};
const WaitingOnYou$1 = {
  args: {
    label: "Regression check",
    activity: "awaiting_human",
    status: "waiting on you",
    elapsed: "2m 04s",
    selected: true,
    open: true,
    factsId: "facts-waiting",
    onToggle: () => {
    },
    onSelect: () => {
    },
    facts: [
      { label: "Checks", value: /* @__PURE__ */ jsx(FactChip, { named: "passed", children: "2 of 2 passed" }) },
      { label: "Judge", value: /* @__PURE__ */ jsx(FactChip, { named: "met", children: "2 of 2 met" }) },
      { label: "Waiting", value: /* @__PURE__ */ jsx(FactChip, { named: "waiting", children: "on you · 2m 04s" }) }
    ]
  }
};
const Locked = {
  args: {
    label: "Land",
    activity: "not_started",
    status: "not started",
    locked: true,
    factsId: "facts-locked",
    onSelect: () => {
    }
  }
};
const __vite_glob_0_31 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Advanced,
  Failed: Failed$3,
  Held,
  Hover: Hover$1,
  Locked,
  Running: Running$4,
  Selected,
  Unreached,
  WaitingOnYou: WaitingOnYou$1,
  default: meta$E
}, Symbol.toStringTag, { value: "Module" }));
function StepStory({ chapters, openId, openChapter, onOpen }) {
  const [held, setHeld] = useState(openId ?? null);
  const bodies = useId();
  const controlled = openChapter !== void 0;
  const open2 = controlled ? openChapter : held;
  function toggle(chapterId) {
    const next = open2 === chapterId ? null : chapterId;
    if (!controlled) setHeld(next);
    onOpen?.(next);
  }
  return /* @__PURE__ */ jsx("ol", { className: "armada-story", children: chapters.map((chapter) => {
    const opens = chapter.content !== void 0;
    const shown = open2 === chapter.id;
    const collapsed = open2 !== null && !shown;
    return /* @__PURE__ */ jsx("li", { className: "armada-story__chapter", "data-open": shown || void 0, children: /* @__PURE__ */ jsx(
      Chapter,
      {
        ordinal: chapter.ordinal,
        name: chapter.title,
        meta: chapter.summary,
        live: chapter.live,
        tone: chapter.tone,
        open: !collapsed,
        onToggle: opens ? () => toggle(chapter.id) : void 0,
        bodyId: `${bodies}-${chapter.id}`,
        moreLabel: !opens ? void 0 : shown ? chapter.closeLabel ?? "Close" : chapter.openLabel ?? "Open",
        onMore: opens ? () => toggle(chapter.id) : void 0,
        moreCloses: shown,
        children: shown ? chapter.content : chapter.preview
      }
    ) }, chapter.id);
  }) });
}
const meta$D = {
  title: "Compositions/Step story",
  component: StepStory
};
const PREVIEW$1 = [
  { id: "1", at: "14:22:07", actor: "armada", summary: "Go on to Implement." },
  { id: "2", at: "14:26:31", actor: "drone", summary: "Edit", subject: "packages/settings/src/selectors.ts" },
  {
    id: "3",
    at: "14:29:40",
    actor: "drone",
    summary: "Bash",
    subject: "cargo build --workspace --locked",
    output: "$ cargo build --workspace --locked\n    Finished `dev` profile [unoptimized] in 47.61s",
    ran: "exit 0 · 47.61s · in .armada/worktrees/job_2d90bb"
  },
  { id: "4", at: "14:30:28", actor: "fleet", summary: "Heartbeat — the Drone has been quiet for 48 seconds" },
  { id: "5", at: "14:31:58", actor: "drone", summary: "thinking" }
];
const WHOLE$1 = [
  ...PREVIEW$1.slice(0, 1),
  { id: "1b", at: "14:22:44", actor: "drone", summary: "Splitting the selector block into its own module so the tests can import it without the store." },
  { id: "1c", at: "14:23:11", actor: "drone", summary: "Read", subject: "packages/settings/src/reducer.ts" },
  ...PREVIEW$1.slice(1)
];
const FILES = /* @__PURE__ */ jsx(
  ChangedFiles,
  {
    emptyNote: "This drone has not changed anything yet.",
    files: [
      { path: "packages/settings/src/selectors.ts", change: "modified" },
      { path: "packages/settings/src/reducer.ts", change: "modified" },
      { path: "packages/settings/src/index.ts", change: "added" }
    ]
  }
);
const CHAPTERS$1 = [
  {
    id: "instructions",
    ordinal: 1,
    title: "Drone instructions",
    summary: "14:22:07",
    preview: "Move the selector block into its own module so the tests can import it without constructing the store. Do not change reducer behaviour."
  },
  {
    id: "log",
    ordinal: 2,
    title: "Activity log",
    live: true,
    summary: "47 entries · every line opens",
    preview: /* @__PURE__ */ jsx(ActivityLog, { entries: PREVIEW$1 }),
    content: /* @__PURE__ */ jsx(ActivityLog, { entries: WHOLE$1 }),
    openLabel: "Open the log — all 47 entries"
  },
  {
    id: "produced",
    ordinal: 3,
    title: "Produced",
    summary: "3 files · +94 −31 · all inside the plan",
    preview: FILES,
    content: FILES,
    openLabel: "Open the diff — 3 files"
  }
];
const TheStory = {
  args: { chapters: CHAPTERS$1 }
};
const TheLogOpen = {
  args: { chapters: CHAPTERS$1, openId: "log" }
};
const TheDiffOpen = {
  args: { chapters: CHAPTERS$1, openId: "produced" }
};
const WithADecision = {
  args: {
    chapters: [
      ...CHAPTERS$1,
      {
        id: "decision",
        ordinal: 4,
        title: "Your decision",
        summary: "nothing advances until you answer",
        preview: "Approve, send back with a note, or reject. Send back returns it to this step; reject ends the Job.",
        tone: "waiting"
      }
    ]
  }
};
const HeldByTheCaller = {
  args: { chapters: CHAPTERS$1, openChapter: "log" }
};
const __vite_glob_0_32 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  HeldByTheCaller,
  TheDiffOpen,
  TheLogOpen,
  TheStory,
  WithADecision,
  default: meta$D
}, Symbol.toStringTag, { value: "Module" }));
function TransitionHistory({ moves, emptyNote, note }) {
  if (moves.length === 0) {
    return /* @__PURE__ */ jsx("p", { className: "armada-history__empty", role: "note", children: emptyNote });
  }
  return /* @__PURE__ */ jsxs("div", { className: "armada-history", children: [
    /* @__PURE__ */ jsx("ol", { className: "armada-history__moves", children: moves.map((move) => /* @__PURE__ */ jsxs("li", { className: "armada-history__move", children: [
      /* @__PURE__ */ jsx("span", { className: "armada-history__seq", children: move.seq }),
      /* @__PURE__ */ jsx("span", { className: "armada-history__at", children: move.at }),
      /* @__PURE__ */ jsx("span", { className: "armada-history__kind", children: move.kind }),
      /* @__PURE__ */ jsxs("span", { className: "armada-history__body", children: [
        /* @__PURE__ */ jsxs("span", { className: "armada-history__head", children: [
          move.subject === void 0 ? null : /* @__PURE__ */ jsx("span", { className: "armada-history__subject", children: move.subject }),
          /* @__PURE__ */ jsx("span", { className: "armada-history__moved", children: move.moved })
        ] }),
        move.why === void 0 ? null : /* @__PURE__ */ jsx("span", { className: "armada-history__why", children: move.why })
      ] }),
      /* @__PURE__ */ jsx("span", { className: "armada-history__actor", children: move.actor })
    ] }, move.seq)) }),
    note === void 0 ? null : /* @__PURE__ */ jsx("p", { className: "armada-history__note", children: note })
  ] });
}
const meta$C = {
  title: "Compositions/Transition history",
  component: TransitionHistory
};
const NOTE = "What Armada did. What the drone said is in its turns.";
const NOTHING_YET = "This job has not moved yet. Creation is not a transition, so no row describes it.";
const AJobThatRanClean = {
  args: {
    note: NOTE,
    emptyNote: NOTHING_YET,
    moves: [
      { seq: 1, at: "09:14:02", kind: "status", moved: "awaiting_approval → queued", actor: "human" },
      { seq: 2, at: "09:14:02", kind: "status", moved: "queued → running", actor: "fleet" },
      {
        seq: 3,
        at: "09:14:03",
        kind: "drone",
        subject: "drn_01M13",
        moved: "drone_spawned on plan",
        actor: "fleet"
      },
      { seq: 4, at: "09:14:03", kind: "step", subject: "plan", moved: "not_started → running", actor: "fleet" },
      { seq: 5, at: "09:21:41", kind: "step", subject: "plan", moved: "running → advanced", actor: "fleet" },
      { seq: 6, at: "09:21:41", kind: "step", subject: "implement", moved: "not_started → running", actor: "fleet" },
      { seq: 7, at: "10:02:18", kind: "step", subject: "implement", moved: "running → advanced", actor: "fleet" },
      {
        seq: 8,
        at: "10:02:19",
        kind: "drone",
        subject: "drn_01M13",
        moved: "drone_exited on plan",
        actor: "fleet"
      },
      { seq: 9, at: "10:02:19", kind: "status", moved: "running → completed_success", actor: "fleet" }
    ]
  }
};
const AJobThatEndedSomewhereSurprising = {
  args: {
    note: NOTE,
    emptyNote: NOTHING_YET,
    moves: [
      { seq: 1, at: "13:40:11", kind: "status", moved: "awaiting_approval → queued", actor: "human" },
      { seq: 2, at: "13:40:11", kind: "status", moved: "queued → running", actor: "fleet" },
      {
        seq: 3,
        at: "13:40:12",
        kind: "drone",
        subject: "drn_01M2A",
        moved: "drone_spawned on implement",
        actor: "fleet"
      },
      { seq: 4, at: "13:40:12", kind: "step", subject: "implement", moved: "not_started → running", actor: "fleet" },
      {
        seq: 5,
        at: "13:58:04",
        kind: "step",
        subject: "implement",
        moved: "running → stopped",
        why: "refused by the judge",
        actor: "fleet"
      },
      {
        seq: 6,
        at: "13:58:04",
        kind: "status",
        moved: "running → escalated",
        why: "refused by the judge · owes c-2, c-4",
        actor: "fleet"
      },
      {
        seq: 7,
        at: "14:06:33",
        kind: "drone",
        subject: "drn_01M2A",
        moved: "drone_exited on implement",
        actor: "human"
      },
      { seq: 8, at: "14:11:50", kind: "step", subject: "implement", moved: "stopped → retrying", actor: "human" },
      {
        seq: 9,
        at: "14:11:51",
        kind: "drone",
        subject: "drn_01M2F",
        moved: "drone_spawned on implement",
        actor: "fleet"
      },
      { seq: 10, at: "14:11:51", kind: "status", moved: "escalated → running", actor: "human" },
      {
        seq: 11,
        at: "14:44:09",
        kind: "status",
        moved: "running → escalated",
        why: "hit the iteration cap",
        actor: "fleet"
      },
      {
        seq: 12,
        at: "14:52:00",
        kind: "drone",
        subject: "drn_01M2F",
        moved: "drone_exited on implement",
        actor: "fleet"
      },
      { seq: 13, at: "14:52:00", kind: "status", moved: "escalated → killed", actor: "human" }
    ]
  }
};
const TwoMovesInOneInstant = {
  args: {
    note: NOTE,
    emptyNote: NOTHING_YET,
    moves: [
      { seq: 41, at: "16:02:57", kind: "step", subject: "verify", moved: "running → advanced", actor: "fleet" },
      { seq: 42, at: "16:02:57", kind: "status", moved: "running → awaiting_review", actor: "fleet" }
    ]
  }
};
const NothingRecordedYet = {
  args: { moves: [], emptyNote: NOTHING_YET }
};
const __vite_glob_0_33 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AJobThatEndedSomewhereSurprising,
  AJobThatRanClean,
  NothingRecordedYet,
  TwoMovesInOneInstant,
  default: meta$C
}, Symbol.toStringTag, { value: "Module" }));
const OUTSIDE_PLAN = "outside plan";
function UnifiedDiff({ files, emptyNote, cut, note, onCopied }) {
  const copy = useCallback(
    (event, value) => {
      event.stopPropagation();
      void navigator.clipboard.writeText(value).then(() => onCopied?.(value));
    },
    [onCopied]
  );
  if (files.length === 0) {
    return /* @__PURE__ */ jsx("p", { className: "armada-diff__empty", role: "note", children: emptyNote });
  }
  return /* @__PURE__ */ jsxs("div", { className: "armada-diff", children: [
    cut === void 0 ? null : /* @__PURE__ */ jsx("p", { className: "armada-diff__cut", role: "note", children: cut }),
    files.map((file) => /* @__PURE__ */ jsxs("section", { className: "armada-diff__file", children: [
      /* @__PURE__ */ jsxs("header", { className: "armada-diff__head", "data-outside": file.outsidePlan === true || void 0, children: [
        /* @__PURE__ */ jsx(
          "span",
          {
            className: "armada-diff__path",
            title: file.path,
            onClick: (event) => copy(event, file.path),
            children: file.path
          }
        ),
        file.meta === void 0 ? null : /* @__PURE__ */ jsx("span", { className: "armada-diff__meta", children: file.meta }),
        /* @__PURE__ */ jsx("span", { className: "armada-diff__mark", children: file.outsidePlan === true ? OUTSIDE_PLAN : null })
      ] }),
      /* @__PURE__ */ jsx("ol", { className: "armada-diff__lines", children: file.lines.map((line, i) => /* @__PURE__ */ jsx("li", { className: "armada-diff__line", "data-kind": line.kind, children: /* @__PURE__ */ jsx("pre", { className: "armada-diff__text", children: line.text }) }, i)) })
    ] }, file.path)),
    note === void 0 ? null : /* @__PURE__ */ jsx("p", { className: "armada-diff__note", children: note })
  ] });
}
const meta$B = {
  title: "Compositions/Unified diff",
  component: UnifiedDiff
};
const READ$1 = "Read from this job's worktree against the branch it was cut from.";
const APatchToDecideOn = {
  args: {
    emptyNote: "",
    note: `${READ$1} Every path is inside the plan this step declared.`,
    files: [
      {
        path: "crates/fleet/src/gate.rs",
        lines: [
          { kind: "hunk", text: "@@ -41,7 +41,11 @@ impl Gate {" },
          { kind: "context", text: "     fn decide(&self, step: &Step) -> Advance {" },
          { kind: "context", text: "         let carried = step.advance_gate();" },
          { kind: "removed", text: "-        if carried == AdvanceGate::Auto {" },
          { kind: "removed", text: "-            return Advance::Now;" },
          { kind: "added", text: "+        match carried {" },
          { kind: "added", text: "+            AdvanceGate::Auto => Advance::Now," },
          { kind: "added", text: "+            AdvanceGate::HumanAlways => Advance::Wait," },
          { kind: "added", text: "+            AdvanceGate::AutoIfJudgePasses => self.ask_the_judge(step)," },
          { kind: "context", text: "         }" },
          { kind: "context", text: "     }" }
        ]
      },
      {
        path: "crates/fleet/src/reviewing.rs",
        meta: "new file mode 100644",
        lines: [
          { kind: "hunk", text: "@@ -0,0 +1,4 @@" },
          { kind: "added", text: "+//! The three acts a person takes on finished work." },
          { kind: "added", text: "+//!" },
          { kind: "added", text: "+//! All three refuse anywhere but `awaiting_review`." },
          { kind: "added", text: "+" }
        ]
      }
    ]
  }
};
const AFileOutsideTheDeclaredPlan = {
  args: {
    emptyNote: "",
    note: `${READ$1} 1 of 2 paths are outside the plan this step declared.`,
    files: [
      {
        path: "crates/fleet/src/gate.rs",
        lines: [
          { kind: "hunk", text: "@@ -41,3 +41,3 @@" },
          { kind: "context", text: "     fn decide(&self, step: &Step) -> Advance {" },
          { kind: "added", text: "+        // The gate the workflow carried, honoured." },
          { kind: "removed", text: "-        // TODO: read the gate." }
        ]
      },
      {
        path: "crates/config/settings.toml",
        outsidePlan: true,
        lines: [
          { kind: "hunk", text: "@@ -12,2 +12,3 @@ [judge]" },
          { kind: "context", text: ' model = "haiku"' },
          { kind: "added", text: " timeout_seconds = 90" }
        ]
      }
    ]
  }
};
const APatchTooLongToDraw = {
  args: {
    emptyNote: "",
    note: READ$1,
    cut: "This is the first 2,000 lines of a 14,318-line patch. The rest is not on screen. Read the whole diff in the worktree named under Where the work is before deciding.",
    files: [
      {
        path: "crates/store/src/schema.rs",
        lines: [
          { kind: "hunk", text: "@@ -199,4 +199,5 @@ const MIGRATIONS: &[Migration] = &[" },
          { kind: "context", text: '    Migration { id: 11, sql: include_str!("sql/011_steps.sql") },' },
          { kind: "added", text: '+    Migration { id: 12, sql: include_str!("sql/012_review.sql") },' },
          { kind: "context", text: "];" }
        ]
      }
    ]
  }
};
const ADroneThatChangedNothing = {
  args: {
    files: [],
    emptyNote: "This job's worktree opened and holds no change against the branch it was cut from. That is what a diff_nonempty check refuses."
  }
};
const AJobWithNoWorktree = {
  args: {
    files: [],
    emptyNote: "This job has no worktree, so there is nothing to read. A job at the approval gate has not been given one."
  }
};
const __vite_glob_0_34 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  ADroneThatChangedNothing,
  AFileOutsideTheDeclaredPlan,
  AJobWithNoWorktree,
  APatchToDecideOn,
  APatchTooLongToDraw,
  default: meta$B
}, Symbol.toStringTag, { value: "Module" }));
const GLYPH = 12;
const STROKE = 2;
const SAYS = {
  open: "Open where this lives",
  copy: "Copy this value",
  into: "Open this"
};
const MARK = { open: ExternalLink, copy: Copy, into: ChevronRight };
function WhereRow({
  label: label2,
  value,
  note,
  act,
  copyValue,
  onCopied,
  onAct,
  actLabel
}) {
  const writes = copyValue ?? (typeof value === "string" ? value : void 0);
  const press = useCallback(() => {
    if (act !== "copy") {
      onAct?.();
      return;
    }
    if (writes === void 0) return;
    void navigator.clipboard.writeText(writes).then(
      () => onCopied?.(writes),
      // A failed clipboard write is otherwise indistinguishable from a dead
      // control, and the row has already said it copied.
      () => onCopied?.(writes)
    );
  }, [act, onAct, onCopied, writes]);
  const actable = act === "copy" ? writes !== void 0 : onAct !== void 0;
  const Mark = MARK[act];
  const says = actLabel ?? SAYS[act];
  const body2 = /* @__PURE__ */ jsxs(Fragment, { children: [
    /* @__PURE__ */ jsx("span", { className: "armada-wrow__k", children: label2 }),
    /* @__PURE__ */ jsxs("span", { className: "armada-wrow__v", children: [
      value,
      note === void 0 ? null : /* @__PURE__ */ jsx("span", { className: "armada-wrow__note", children: note })
    ] }),
    /* @__PURE__ */ jsx(Mark, { size: GLYPH, strokeWidth: STROKE, className: "armada-wrow__mark", "aria-hidden": true })
  ] });
  if (!actable) {
    return /* @__PURE__ */ jsx("div", { className: "armada-wrow", "data-act": act, children: body2 });
  }
  return /* @__PURE__ */ jsxs("button", { type: "button", className: "armada-wrow", "data-act": act, onClick: press, title: says, children: [
    body2,
    /* @__PURE__ */ jsx("span", { className: "armada-wrow__sr", children: says })
  ] });
}
const meta$A = {
  title: "Compositions/Where row",
  component: WhereRow,
  decorators: [
    (Story) => /* @__PURE__ */ jsx("div", { style: { width: "calc(var(--space-12) * 8)" }, children: /* @__PURE__ */ jsx(Story, {}) })
  ]
};
const APathThatOpens = {
  args: {
    label: "Worktree",
    value: ".armada/worktrees/job_2d90bb",
    act: "open",
    onAct: () => {
    }
  }
};
const AnIdentifierThatCopies = {
  args: {
    label: "Drone",
    value: "01M10B1V2A0011VRS6RA2SKPQ7",
    act: "copy",
    onCopied: () => {
    }
  }
};
const TheWholeRegion = {
  render: () => /* @__PURE__ */ jsxs("div", { style: { display: "flex", flexDirection: "column" }, children: [
    /* @__PURE__ */ jsx(
      WhereRow,
      {
        label: "Worktree",
        value: ".armada/worktrees/job_2d90bb",
        act: "open",
        onAct: () => {
        }
      }
    ),
    /* @__PURE__ */ jsx(
      WhereRow,
      {
        label: "Branch",
        value: "fix/settings-split-selectors",
        act: "copy",
        onCopied: () => {
        }
      }
    ),
    /* @__PURE__ */ jsx(WhereRow, { label: "Manifest", value: "armada.yml", act: "open", onAct: () => {
    } }),
    /* @__PURE__ */ jsx(
      WhereRow,
      {
        label: "Workflow",
        value: "bug",
        note: "as it was at 14:20",
        act: "into",
        onAct: () => {
        }
      }
    ),
    /* @__PURE__ */ jsx(
      WhereRow,
      {
        label: "Job log",
        value: ".armada/logs/job_2d90bb.jsonl",
        act: "open",
        onAct: () => {
        }
      }
    ),
    /* @__PURE__ */ jsx(
      WhereRow,
      {
        label: "Drone",
        value: "01M10B1V2A0011VRS6RA2SKPQ7",
        act: "copy",
        onCopied: () => {
        }
      }
    )
  ] })
};
const WiderThanTheColumn = {
  render: () => /* @__PURE__ */ jsx("div", { style: { width: "calc(var(--space-12) * 5)" }, children: /* @__PURE__ */ jsx(
    WhereRow,
    {
      label: "Transcript",
      value: ".armada/transcripts/01M10B1V2A0011VRS6RA2SKPQ7.jsonl",
      act: "open",
      onAct: () => {
      }
    }
  ) })
};
const NothingToDo = {
  args: {
    label: "Manifest",
    value: "armada.yml",
    act: "open"
  }
};
const __vite_glob_0_35 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  APathThatOpens,
  AnIdentifierThatCopies,
  NothingToDo,
  TheWholeRegion,
  WiderThanTheColumn,
  default: meta$A
}, Symbol.toStringTag, { value: "Module" }));
const meta$z = {
  title: "Compositions/Workflow rail",
  component: WorkflowRail
};
const EVIDENCE = FileCheck;
const running$1 = [
  {
    id: "plan",
    label: "Plan the change",
    activity: "advanced",
    status: "advanced",
    evidence: { icon: EVIDENCE, iconLabel: "Evidence", label: "evidence · 09:14" }
  },
  {
    id: "implement",
    label: "Implement",
    activity: "running",
    status: "running · 6m 12s",
    current: true,
    gates: [
      { command: "build · cargo build --workspace", result: "not reached", icon: ShieldMinus, iconLabel: "Not reached" },
      { command: "diff_nonempty", result: "not reached", icon: ShieldMinus, iconLabel: "Not reached" }
    ],
    declarations: [{ label: "judge · 2 criteria", result: "not reached" }, { label: "advance_gate · auto_if_judge_passes" }]
  },
  {
    id: "verify",
    label: "Run tests",
    activity: "not_started",
    status: "not started",
    gates: [{ command: "test · cargo test --workspace", result: "not reached", icon: ShieldMinus, iconLabel: "Not reached" }],
    declarations: [{ label: "judge · 1 criterion · gaming check", result: "not reached" }, { label: "advance_gate · auto_if_judge_passes" }]
  },
  {
    id: "handoff",
    label: "Summarise",
    activity: "not_started",
    status: "not started",
    declarations: [{ label: "advance_gate · human_always" }]
  }
];
const Running$3 = {
  args: { steps: running$1, pulsing: true }
};
const RunningPulseElsewhere = {
  args: { steps: running$1, pulsing: false }
};
const AWorkflowBeforeItRuns = {
  args: {
    steps: running$1.map(({ id, label: label2, gates, declarations }) => ({
      id,
      label: label2,
      activity: "not_started",
      gates: gates?.map(({ command }) => ({ command })),
      declarations: declarations?.map(({ label: what }) => ({ label: what }))
    }))
  }
};
const Failed$2 = {
  args: {
    steps: [
      { id: "plan", label: "Plan the change", activity: "advanced", status: "advanced", evidence: { icon: EVIDENCE, label: "evidence · 13:58" } },
      {
        id: "implement",
        label: "Implement",
        activity: "advanced",
        status: "advanced",
        gates: [
          { command: "build · cargo build --workspace", result: "exit 0", icon: ShieldCheck, iconLabel: "Passed" },
          { command: "diff_nonempty", result: "passed", icon: ShieldCheck, iconLabel: "Passed" }
        ]
      },
      {
        id: "verify",
        label: "Run tests",
        activity: "failed",
        status: "failed a check",
        gates: [{ command: "test · cargo test --workspace", result: "exit 1", icon: ShieldX, iconLabel: "Failed" }]
      },
      { id: "handoff", label: "Summarise", activity: "not_started", status: "not started" }
    ]
  }
};
const Stopped = {
  args: {
    steps: [
      { id: "plan", label: "Plan the change", activity: "advanced", status: "advanced" },
      {
        id: "implement",
        label: "Implement",
        activity: "stopped",
        status: "retries spent",
        gates: [{ command: "build · cargo build --workspace", result: "exit 101", icon: ShieldX, iconLabel: "Failed" }]
      },
      { id: "verify", label: "Run tests", activity: "not_started", status: "not started" }
    ]
  }
};
const EvidenceFlaggedByTheGamingCheck = {
  args: {
    steps: [
      {
        id: "implement",
        label: "Implement",
        activity: "advanced",
        status: "advanced",
        elapsed: "6m 48s",
        verdict: "passed",
        verdictNamed: "passed",
        gates: [{ command: "build · cargo build --workspace", result: "passed", icon: ShieldCheck, iconLabel: "passed" }]
      },
      {
        id: "verify",
        label: "Run tests",
        activity: "stopped",
        status: "stopped",
        current: true,
        elapsed: "3m 07s",
        verdict: "failed · evidence disputed",
        verdictNamed: "failed",
        gates: [{ command: "test · cargo test --workspace", result: "passed", icon: ShieldCheck, iconLabel: "passed" }],
        declarations: [{ label: "judge · 2 criteria · gaming check" }, { label: "advance_gate · auto_if_judge_passes" }],
        flags: [
          { pattern: "check_config_edited", cited: "package.json · scripts.test now runs vitest run src/unit" },
          { pattern: "assertion_weakened", cited: "src/parse.test.ts:88 · toEqual replaced by toBeDefined" }
        ]
      },
      { id: "handoff", label: "Summarise", activity: "not_started", status: "not started" }
    ]
  }
};
const AFlaggedStepIsNeverUngated = {
  args: {
    steps: [
      {
        id: "root_cause",
        label: "root_cause",
        labelIsAnIdentifier: true,
        activity: "stopped",
        status: "stopped",
        current: true,
        elapsed: "8m 22s",
        evidence: { label: "" },
        flags: [{ pattern: "findings_generic", cited: "the report names no file and no line" }]
      }
    ]
  }
};
const WaitingAndRetrying = {
  args: {
    steps: [
      { id: "plan", label: "Plan the change", activity: "advanced", status: "advanced" },
      { id: "review", label: "Review the diff", activity: "awaiting_human", status: "waiting on you", current: true },
      { id: "fix", label: "Fix", activity: "retrying", status: "retrying · attempt 2" }
    ]
  }
};
const Killed$3 = {
  args: {
    steps: [
      { id: "plan", label: "Plan the change", activity: "advanced", status: "advanced" },
      { id: "implement", label: "Implement", activity: "killed", status: "killed · 4m 09s" },
      { id: "verify", label: "Run tests", activity: "not_started", status: "not started" }
    ]
  }
};
const HardPrerequisite = {
  args: {
    steps: [
      { id: "plan", label: "Plan the change", activity: "advanced", status: "advanced" },
      {
        id: "verify",
        label: "Run tests",
        activity: "not_started",
        status: "not started",
        trailing: /* @__PURE__ */ jsxs(
          "span",
          {
            style: {
              display: "inline-flex",
              alignItems: "center",
              gap: "var(--space-1)",
              flex: "none",
              color: "var(--fg-muted)",
              fontSize: "var(--text-2xs)"
            },
            children: [
              /* @__PURE__ */ jsx(Lock, { size: 12, strokeWidth: 2, "aria-hidden": true }),
              "Cannot be skipped"
            ]
          }
        )
      }
    ]
  }
};
const LabelsMissing = {
  args: {
    steps: [
      { id: "plan", label: "plan", labelIsAnIdentifier: true, activity: "advanced", status: "advanced" },
      { id: "implement", label: "implement", labelIsAnIdentifier: true, activity: "running", status: "running", current: true },
      { id: "verify", label: "verify", labelIsAnIdentifier: true, activity: "not_started", status: "not started" },
      { id: "handoff", label: "handoff", labelIsAnIdentifier: true, activity: "not_started", status: "not started" }
    ],
    pulsing: true
  }
};
const OneStep = {
  args: {
    steps: [{ id: "fix", label: "Fix", activity: "running", status: "running · 1m 40s", current: true }],
    pulsing: true
  }
};
const ServedSteps = {
  args: {
    steps: [
      { id: "plan", label: "plan", labelIsAnIdentifier: true, activity: "advanced", status: "advanced", elapsed: "2m 14s", verdict: "passed", verdictNamed: "passed" },
      { id: "implement", label: "implement", labelIsAnIdentifier: true, activity: "running", status: "running", current: true, elapsed: "11m 03s" },
      { id: "verify", label: "verify", labelIsAnIdentifier: true, activity: "not_started", status: "not_started" },
      { id: "handoff", label: "handoff", labelIsAnIdentifier: true, activity: "not_started", status: "not_started" }
    ],
    pulsing: true
  }
};
const EveryStepState = {
  args: {
    steps: [
      {
        id: "not_started",
        label: "not_started",
        labelIsAnIdentifier: true,
        activity: "not_started",
        status: "not_started",
        gates: [{ command: "build", result: "not reached", icon: ShieldMinus, iconLabel: "not reached" }]
      },
      {
        id: "running",
        label: "running",
        labelIsAnIdentifier: true,
        activity: "running",
        status: "running",
        current: true,
        elapsed: "6m 12s",
        gates: [{ command: "test", result: "not reached", icon: ShieldMinus, iconLabel: "not reached" }]
      },
      { id: "awaiting_human", label: "awaiting_human", labelIsAnIdentifier: true, activity: "awaiting_human", status: "awaiting_human", elapsed: "1m 04s" },
      { id: "retrying", label: "retrying", labelIsAnIdentifier: true, activity: "retrying", status: "retrying", elapsed: "3m 41s", verdict: "failed · failed a check", verdictNamed: "failed" },
      {
        id: "advanced",
        label: "advanced",
        labelIsAnIdentifier: true,
        activity: "advanced",
        status: "advanced",
        elapsed: "2m 14s",
        verdict: "passed",
        verdictNamed: "passed",
        gates: [{ command: "fmt", result: "passed", icon: ShieldCheck, iconLabel: "passed" }]
      },
      {
        id: "stopped",
        label: "stopped",
        labelIsAnIdentifier: true,
        activity: "stopped",
        status: "stopped",
        elapsed: "12m 30s",
        gates: [{ command: "build", result: "failed · exit 0 → exit 101", icon: ShieldX, iconLabel: "failed" }]
      }
    ],
    pulsing: true
  }
};
const AVerdictBesideTheState = {
  args: {
    steps: [
      {
        id: "plan",
        label: "plan",
        labelIsAnIdentifier: true,
        activity: "advanced",
        elapsed: "2m 14s",
        verdict: "passed",
        verdictNamed: "passed"
      },
      {
        id: "verify",
        label: "verify",
        labelIsAnIdentifier: true,
        activity: "retrying",
        current: true,
        elapsed: "6m 51s",
        verdict: "failed · failed a check",
        verdictNamed: "failed"
      },
      { id: "handoff", label: "handoff", labelIsAnIdentifier: true, activity: "not_started" }
    ]
  }
};
const OverruledByAPerson = {
  args: {
    steps: [
      {
        id: "implement",
        label: "Implement",
        activity: "advanced",
        status: "advanced",
        elapsed: "6m 48s",
        verdict: "passed",
        verdictNamed: "passed",
        gates: [{ command: "build · cargo build --workspace", result: "passed", icon: ShieldCheck, iconLabel: "passed" }]
      },
      {
        id: "verify",
        label: "Run tests",
        activity: "advanced",
        status: "advanced",
        elapsed: "12m 30s",
        verdict: "failed · refused by the judge",
        verdictNamed: "failed",
        overridden: "overruled by a person",
        gates: [{ command: "test · cargo test --workspace", result: "passed", icon: ShieldCheck, iconLabel: "passed" }],
        declarations: [{ label: "judge · 2 criteria" }, { label: "advance_gate · auto_if_judge_passes" }],
        verdicts: [{
          ordinal: 2,
          criterionId: "crit_7f2a",
          named: "not_met",
          verdict: "refused",
          text: "The regression is covered by a test that fails without the fix.",
          expected: "a test that fails on the parent commit",
          produced: "a test asserting the new behaviour only",
          consequence: "a reader cannot tell the fix from the assertion"
        }]
      },
      {
        id: "handoff",
        label: "Summarise",
        activity: "running",
        status: "running · 0m 04s",
        current: true,
        declarations: [{ label: "advance_gate · human_always" }]
      }
    ],
    pulsing: true
  }
};
const UngatedAndUnanswerable = {
  args: {
    steps: [
      {
        id: "reproduce",
        label: "reproduce",
        labelIsAnIdentifier: true,
        activity: "advanced",
        elapsed: "1m 40s",
        evidence: { label: "" }
      },
      {
        id: "root_cause",
        label: "root_cause",
        labelIsAnIdentifier: true,
        activity: "running",
        current: true,
        elapsed: "4m 02s",
        // What Bridge passes when `checks` is absent from the served step.
        ungatedLabel: "Fleet cannot say what this step checks",
        evidence: { label: "" }
      }
    ],
    pulsing: true
  }
};
const TheSixCheckOutcomes = {
  args: {
    steps: [
      {
        id: "verify",
        label: "verify",
        labelIsAnIdentifier: true,
        activity: "stopped",
        current: true,
        elapsed: "3m 18s",
        verdict: "failed · failed a check",
        verdictNamed: "failed",
        gates: [
          { command: "fmt", result: "passed" },
          { command: "build", result: "failed · exit 0 → exit 101" },
          { command: "test", result: "signalled · SIGKILL" },
          { command: "audit", result: "timed_out · 120s budget" },
          { command: "typecheck", result: "never_ran · tsc is not installed" },
          { command: "storybook", result: "skipped · no changed file is under packages/**" }
        ]
      }
    ]
  }
};
const AChecksPathsAreDrawnBeforeItRuns = {
  args: {
    steps: [
      {
        id: "implement",
        label: "Implement",
        labelIsAnIdentifier: true,
        activity: "not_started",
        status: "not_started",
        gates: [
          { command: "build · cargo build --workspace --locked" },
          { command: "storybook · pnpm -C packages/components build-storybook", covers: "when packages/**, apps/desktop/**" }
        ]
      },
      {
        id: "verify",
        label: "Verify",
        labelIsAnIdentifier: true,
        activity: "advanced",
        status: "advanced",
        elapsed: "1m 22s",
        verdict: "passed",
        verdictNamed: "passed",
        gates: [
          { command: "build · cargo build --workspace --locked", result: "passed", icon: ShieldCheck, iconLabel: "passed" },
          { command: "storybook · pnpm -C packages/components build-storybook", covers: "when packages/**, apps/desktop/**", result: "not run · no changed file is under packages/**, apps/desktop/**", icon: ShieldOff, iconLabel: "not run" }
        ]
      }
    ]
  }
};
const AFailedCheckNamesItsOutput = {
  args: {
    steps: [
      {
        id: "verify",
        label: "Run tests",
        activity: "failed",
        status: "failed a check",
        current: true,
        gates: [
          { command: "test · cargo test --workspace", result: "failed · exit 0 → exit 101", icon: ShieldX, iconLabel: "Failed", outputPath: ".armada/jobs/job_2d90bb/checks/test.log" },
          { command: "diff_nonempty", result: "never started", icon: ShieldMinus, iconLabel: "Never started" }
        ]
      }
    ]
  }
};
const FourJobsOverAndOneResumable = {
  render: () => {
    const done2 = { id: "implement", label: "Implement", activity: "advanced", status: "advanced", elapsed: "6m 48s" };
    const failing = { command: "test · cargo test --workspace", result: "failed · exit 0 → exit 101", icon: ShieldX, iconLabel: "failed", outputPath: ".armada/jobs/job_91ab/checks/test.log" };
    const rails = [
      [
        "escalated",
        "over, and resumable — a redirect resumes exactly this step",
        { id: "verify", label: "Run tests", activity: "stopped", status: "stopped", current: true, elapsed: "12m 30s", gates: [failing] }
      ],
      [
        "completed_failed",
        "over, and a dead end. Nothing resumes it",
        { id: "verify", label: "Run tests", activity: "failed", status: "failed", current: true, elapsed: "12m 30s", gates: [failing] }
      ],
      [
        "killed",
        "frozen where it stood, and unhued — an operator act carries no verdict",
        {
          id: "verify",
          label: "Run tests",
          activity: "killed",
          status: "killed",
          current: true,
          elapsed: "4m 09s",
          gates: [{ command: "test · cargo test --workspace", result: "not reached", icon: ShieldMinus, iconLabel: "not reached" }]
        }
      ],
      [
        "completed_success",
        "frozen, every step advanced. The Job's status says so",
        {
          id: "verify",
          label: "Run tests",
          activity: "advanced",
          status: "done",
          current: true,
          elapsed: "9m 51s",
          gates: [{ command: "test · cargo test --workspace", result: "passed", icon: ShieldCheck, iconLabel: "passed" }]
        }
      ]
    ];
    return /* @__PURE__ */ jsx("div", { style: { display: "flex", flexDirection: "column", gap: "var(--space-6)" }, children: rails.map(([status, reads, step]) => /* @__PURE__ */ jsxs("div", { style: { display: "flex", flexDirection: "column", gap: "var(--space-2)" }, children: [
      /* @__PURE__ */ jsxs("span", { style: { fontSize: "var(--text-2xs)", color: "var(--fg-subtle)" }, children: [
        /* @__PURE__ */ jsx("span", { style: { fontFamily: "var(--font-mono)" }, children: status }),
        ` — ${reads}`
      ] }),
      /* @__PURE__ */ jsx(WorkflowRail, { steps: [done2, step] })
    ] }, status)) });
  }
};
const WhatAStepDeclares = {
  args: {
    steps: [
      { id: "scope", label: "scope", labelIsAnIdentifier: true, activity: "advanced", status: "advanced", elapsed: "1m 12s", evidence: { label: "" } },
      {
        id: "implement",
        label: "implement",
        labelIsAnIdentifier: true,
        activity: "not_started",
        status: "not_started",
        declarations: [{ label: "judge · 2 criteria · panel of 3", result: "not reached" }, { label: "advance_gate · auto_if_judge_passes" }]
      },
      {
        id: "tests",
        label: "tests",
        labelIsAnIdentifier: true,
        activity: "not_started",
        status: "not_started",
        declarations: [{ label: "judge · gaming check", result: "not reached" }]
      },
      {
        id: "handoff",
        label: "handoff",
        labelIsAnIdentifier: true,
        activity: "not_started",
        status: "not_started",
        declarations: [{ label: "advance_gate · human_always" }]
      }
    ]
  }
};
const __vite_glob_0_36 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AChecksPathsAreDrawnBeforeItRuns,
  AFailedCheckNamesItsOutput,
  AFlaggedStepIsNeverUngated,
  AVerdictBesideTheState,
  AWorkflowBeforeItRuns,
  EveryStepState,
  EvidenceFlaggedByTheGamingCheck,
  Failed: Failed$2,
  FourJobsOverAndOneResumable,
  HardPrerequisite,
  Killed: Killed$3,
  LabelsMissing,
  OneStep,
  OverruledByAPerson,
  Running: Running$3,
  RunningPulseElsewhere,
  ServedSteps,
  Stopped,
  TheSixCheckOutcomes,
  UngatedAndUnanswerable,
  WaitingAndRetrying,
  WhatAStepDeclares,
  default: meta$z
}, Symbol.toStringTag, { value: "Module" }));
const meta$y = {
  title: "Errors/Error code",
  component: ErrorCode
};
const Fault = {
  args: { kind: "fault", code: "fleet.approve.refused" }
};
const Degraded = {
  args: { kind: "degraded", code: "bridge.stream.dropped" }
};
const AgainstAStatusBadge = {
  render: () => /* @__PURE__ */ jsxs("div", { style: { display: "flex", alignItems: "center", gap: "var(--space-3)" }, children: [
    /* @__PURE__ */ jsx(Badge, { status: "completed-failed", icon: X, children: "Failed" }),
    /* @__PURE__ */ jsx(ErrorCode, { kind: "fault", code: "fleet.approve.refused" })
  ] })
};
const __vite_glob_0_37 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AgainstAStatusBadge,
  Degraded,
  Fault,
  default: meta$y
}, Symbol.toStringTag, { value: "Module" }));
function Checkbox({ children, ...rest }) {
  return /* @__PURE__ */ jsxs("label", { className: "armada-checkbox", children: [
    /* @__PURE__ */ jsx("input", { ...rest, type: "checkbox", className: "armada-checkbox__input" }),
    /* @__PURE__ */ jsx("span", { className: "armada-checkbox__box", "aria-hidden": "true", children: /* @__PURE__ */ jsx(Check, { size: 12, strokeWidth: 2 }) }),
    /* @__PURE__ */ jsx("span", { children })
  ] });
}
const GAP = 3;
const INDENT = "  ";
const NO_CODE = "none";
function aligned(rows, indent) {
  const width = Math.max(...rows.map(([label2]) => label2.length)) + GAP;
  return rows.map(([label2, value]) => {
    const pad = " ".repeat(indent.length + width);
    const [head, ...rest] = value.split("\n");
    const first = `${indent}${label2}${" ".repeat(width - label2.length)}${head ?? ""}`;
    return [first, ...rest.map((line) => `${pad}${line}`)].join("\n");
  });
}
function ordered(chain) {
  const width = String(chain.length).length;
  return chain.map((entry, at) => `${INDENT}${String(at + 1).padStart(width)}  ${entry}`);
}
function debugInfo(payload) {
  const guaranteed = [
    ["code", payload.code ?? NO_CODE],
    ["message", payload.message]
  ];
  if (payload.run_id !== void 0) guaranteed.push(["run_id", payload.run_id]);
  if (payload.job_id !== void 0) guaranteed.push(["job_id", payload.job_id]);
  if (payload.drone_id !== void 0) guaranteed.push(["drone_id", payload.drone_id]);
  if (payload.step_id !== void 0) guaranteed.push(["step_id", payload.step_id]);
  const blocks = [["armada error"], aligned(guaranteed, "")];
  const fields = payload.fields ?? [];
  if (fields.length > 0) {
    blocks.push([
      "fields",
      ...aligned(
        fields.map((field) => [field.key, field.value]),
        INDENT
      )
    ]);
  }
  const chain = payload.chain ?? [];
  if (chain.length > 0) blocks.push(["chain", ...ordered(chain)]);
  const tail = [`bridge protocol ${payload.bridgeProtocol}`];
  if (payload.fleetProtocol !== void 0) tail.push(`fleet protocol ${payload.fleetProtocol}`);
  tail.push(`taken ${payload.at}`);
  blocks.push([tail.join("  ")]);
  return blocks.map((block) => block.join("\n")).join("\n\n");
}
function copyDebugInfo(payload, onCopied) {
  const said = () => onCopied?.(COPIED);
  void navigator.clipboard.writeText(debugInfo(payload)).then(said, said);
}
const COPIED = "The debug info";
const SAFETY = "Structured fields carry primitives only, and a credential does not compile into one. Nothing bounds the message or the chain, which are prose an error wrote — read them before you send this.";
const COPY_DEBUG_INFO = "Copy debug info";
const MIN_FENCE = 3;
function fence(body2) {
  let longest = 0;
  let run = 0;
  for (const character of body2) {
    run = character === "`" ? run + 1 : 0;
    if (run > longest) longest = run;
  }
  return "`".repeat(Math.max(MIN_FENCE, longest + 1));
}
const NO_SCRUB = "Nothing here was scrubbed. Armada removes nothing on the way out, so what is below is what left the machine.";
function issueBody(filing) {
  const lines = [`# ${filing.title}`, "", NO_SCRUB];
  for (const item of filing.attached) {
    const wrap = fence(item.body);
    lines.push("", `## ${item.label}`, "", item.warning, "", wrap, item.body, wrap);
  }
  const withheld = filing.withheld ?? [];
  if (withheld.length > 0) {
    lines.push("", "## Not attached", "");
    for (const item of withheld) lines.push(`- **${item.label}** — ${item.why}`);
  }
  return lines.join("\n");
}
function copyIssue(filing, onCopied) {
  const said = () => onCopied?.(COPIED_ISSUE);
  void navigator.clipboard.writeText(issueBody(filing)).then(said, said);
}
const COPIED_ISSUE = "The issue";
const FILE_AN_ISSUE = "File an issue";
const COPY_THE_ISSUE = "Copy the issue";
const OPENS_NOTHING = "Armada does not open anything in the tracker. Copying puts the issue on your clipboard; the last step is yours.";
const ENVELOPE$1 = "The error's own record";
function envelopeOf(payload) {
  return {
    id: "envelope",
    label: ENVELOPE$1,
    warning: SAFETY,
    body: debugInfo(payload),
    required: true
  };
}
const NOT_OFFERED = [
  {
    label: "The drone's turns",
    why: "Whether a transcript may leave this machine is not decided. It carries every command the drone ran and every path it touched."
  }
];
const FOLD_ICON = 16;
const FOLD_STROKE = 2;
const ALWAYS = "Always sent";
const READ = "Read";
function FileAnIssue({ compose, onCopied }) {
  const [offered, setOffered] = useState(null);
  const [removed, setRemoved] = useState(/* @__PURE__ */ new Set());
  function close() {
    setOffered(null);
    setRemoved(/* @__PURE__ */ new Set());
  }
  return /* @__PURE__ */ jsxs(Fragment, { children: [
    /* @__PURE__ */ jsx(Button, { variant: "ghost", size: "sm", ground: "sunken", onClick: () => setOffered(compose()), children: FILE_AN_ISSUE }),
    /* @__PURE__ */ jsx(
      FilingReview,
      {
        offered,
        removed,
        onToggle: (id) => setRemoved((was) => {
          const next = new Set(was);
          if (next.has(id)) next.delete(id);
          else next.add(id);
          return next;
        }),
        onCancel: close,
        onCopy: (filing) => {
          copyIssue(filing, onCopied);
          close();
        }
      }
    )
  ] });
}
const REVIEW_TITLE = "Send this failure to an issue tracker?";
function FilingReview({ offered, removed, onToggle, onCancel, onCopy }) {
  const filing = offered === null ? null : {
    ...offered,
    attached: offered.attached.filter(
      (item) => item.required === true || !removed.has(item.id)
    )
  };
  return /* @__PURE__ */ jsx(
    Dialog,
    {
      open: filing !== null,
      tone: "neutral",
      width: "wide",
      title: REVIEW_TITLE,
      confirmLabel: COPY_THE_ISSUE,
      confirmDisabled: filing === null || filing.attached.length === 0,
      onCancel,
      onConfirm: () => {
        if (filing !== null) onCopy(filing);
      },
      children: offered === null || filing === null ? null : /* @__PURE__ */ jsxs("div", { className: "armada-filing", children: [
        /* @__PURE__ */ jsx("p", { className: "armada-filing__says", children: NO_SCRUB }),
        /* @__PURE__ */ jsx("p", { className: "armada-filing__says", children: OPENS_NOTHING }),
        /* @__PURE__ */ jsx("ul", { className: "armada-filing__rows", children: offered.attached.map((item) => /* @__PURE__ */ jsxs("li", { className: "armada-filing__row", children: [
          /* @__PURE__ */ jsx("div", { className: "armada-filing__control", children: item.required === true ? (
            // Not a checked, disabled checkbox: a control that cannot
            // be operated reads as one that is broken, and this row is
            // not a decision made for somebody. It is a fact about the
            // artifact, stated after the name it is a fact about.
            /* @__PURE__ */ jsxs(Fragment, { children: [
              /* @__PURE__ */ jsx("span", { className: "armada-filing__label", children: item.label }),
              /* @__PURE__ */ jsx("span", { className: "armada-filing__always", children: ALWAYS })
            ] })
          ) : /* @__PURE__ */ jsx(
            Checkbox,
            {
              checked: !removed.has(item.id),
              onChange: () => onToggle(item.id),
              children: item.label
            }
          ) }),
          /* @__PURE__ */ jsx("p", { className: "armada-filing__warning", children: item.warning }),
          /* @__PURE__ */ jsxs("details", { className: "armada-filing__read", children: [
            /* @__PURE__ */ jsxs("summary", { className: "armada-filing__summary", children: [
              /* @__PURE__ */ jsx(Fold, {}),
              READ
            ] }),
            /* @__PURE__ */ jsx("pre", { className: "armada-filing__text", children: item.body })
          ] })
        ] }, item.id)) }),
        offered.withheld === void 0 || offered.withheld.length === 0 ? null : /* @__PURE__ */ jsxs("div", { className: "armada-filing__withheld", children: [
          /* @__PURE__ */ jsx("p", { className: "armada-filing__says", children: "Not offered, and the issue body says so too — a reader who finds none of this cannot otherwise tell it was left out on purpose." }),
          /* @__PURE__ */ jsx("ul", { className: "armada-filing__rows", children: offered.withheld.map((item) => /* @__PURE__ */ jsxs("li", { className: "armada-filing__row", children: [
            /* @__PURE__ */ jsx("span", { className: "armada-filing__label", children: item.label }),
            /* @__PURE__ */ jsx("p", { className: "armada-filing__warning", children: item.why })
          ] }, item.label)) })
        ] }),
        /* @__PURE__ */ jsxs("details", { className: "armada-filing__read", children: [
          /* @__PURE__ */ jsxs("summary", { className: "armada-filing__summary", children: [
            /* @__PURE__ */ jsx(Fold, {}),
            `${READ} the whole body`
          ] }),
          /* @__PURE__ */ jsx("pre", { className: "armada-filing__text", children: issueBody(filing) })
        ] })
      ] })
    }
  );
}
function Fold() {
  return /* @__PURE__ */ jsxs(Fragment, { children: [
    /* @__PURE__ */ jsx(
      ChevronRight,
      {
        className: "armada-filing__caret armada-filing__caret--shut",
        size: FOLD_ICON,
        strokeWidth: FOLD_STROKE,
        "aria-hidden": true
      }
    ),
    /* @__PURE__ */ jsx(
      ChevronDown,
      {
        className: "armada-filing__caret armada-filing__caret--open",
        size: FOLD_ICON,
        strokeWidth: FOLD_STROKE,
        "aria-hidden": true
      }
    )
  ] });
}
const EXPANDABLE = {
  inline: "never",
  toast: "never",
  banner: "disclosed",
  surface: "always"
};
function ErrorNotice(props) {
  const { kind, code, message, fields, actions, placement, payload, onCopied } = props;
  const act = props.act;
  const dismiss = props.placement === "toast" ? props.onDismiss : void 0;
  const [open2, setOpen] = useState(false);
  const disclosure = EXPANDABLE[placement];
  const expanded = payload !== void 0 && (disclosure === "always" || open2);
  const text = payload === void 0 ? null : debugInfo(payload);
  const copy = useCallback(() => {
    if (payload === void 0) return;
    copyDebugInfo(payload, onCopied);
    dismiss?.();
  }, [payload, onCopied, dismiss]);
  return /* @__PURE__ */ jsxs(
    "section",
    {
      className: `armada-error armada-error--${kind} armada-error--${placement}`,
      "data-error-class": kind,
      "data-placement": placement,
      role: kind === "fault" ? "alert" : "status",
      children: [
        /* @__PURE__ */ jsxs("div", { className: "armada-error__head", children: [
          kind === "degraded" ? /* @__PURE__ */ jsx("span", { className: "armada-error__dot", "aria-hidden": "true" }) : null,
          /* @__PURE__ */ jsx("span", { className: "armada-error__message", children: message })
        ] }),
        act !== void 0 ? /* @__PURE__ */ jsx("span", { className: "armada-error__act", children: act }) : null,
        /* @__PURE__ */ jsxs("div", { className: "armada-error__facts", children: [
          /* @__PURE__ */ jsx(ErrorCode, { kind, code }),
          fields !== void 0 && fields.length > 0 ? /* @__PURE__ */ jsx("dl", { className: "armada-error__fields", children: fields.map((field, at) => /* @__PURE__ */ jsxs("div", { className: "armada-error__field", children: [
            /* @__PURE__ */ jsx("dt", { className: "armada-error__label", children: field.label }),
            /* @__PURE__ */ jsx("dd", { className: "armada-error__value", children: field.value })
          ] }, at)) }) : null
        ] }),
        expanded && text !== null && payload !== void 0 ? /* @__PURE__ */ jsxs("div", { className: "armada-error__payload", children: [
          /* @__PURE__ */ jsx("pre", { className: "armada-error__debug", children: text }),
          /* @__PURE__ */ jsx("p", { className: "armada-error__safety", children: SAFETY }),
          /* @__PURE__ */ jsx(
            FileAnIssue,
            {
              compose: () => ({
                title: payload.message,
                attached: [envelopeOf(payload)],
                withheld: NOT_OFFERED
              }),
              onCopied
            }
          )
        ] }) : null,
        payload !== void 0 || actions !== void 0 ? /* @__PURE__ */ jsxs("div", { className: "armada-error__actions", children: [
          payload === void 0 ? null : /* @__PURE__ */ jsx(Button, { variant: "ghost", size: "sm", onClick: copy, children: COPY_DEBUG_INFO }),
          payload !== void 0 && disclosure === "disclosed" ? /* @__PURE__ */ jsx(Button, { variant: "ghost", size: "sm", "aria-expanded": open2, onClick: () => setOpen((was) => !was), children: "Details" }) : null,
          actions
        ] }) : null
      ]
    }
  );
}
const REFUSED$1 = {
  code: "judge.undecided",
  message: "judge returned prose for criterion 2",
  run_id: "01JQ8ZC4M2WYVK7T3RQN8H",
  job_id: "job_31c7",
  drone_id: "drn_4c8",
  step_id: "verify",
  fields: [
    { key: "criterion", value: "2" },
    // The tier, not the vendor's id for it. Every other story in this package
    // says `sonnet` for the same reason the vendor-literal rule refuses the
    // other spelling: a model name typed into a sample is a second roster.
    { key: "judge_model", value: "sonnet" },
    { key: "response_bytes", value: "1184" }
  ],
  chain: [
    "judge: no verdict parsed from response",
    "gate verify: undecided",
    "job_31c7: escalated"
  ],
  bridgeProtocol: "5.2",
  fleetProtocol: "5.2",
  at: "2026-08-30T09:16:40Z"
};
const meta$x = {
  title: "Errors/Error notice",
  component: ErrorNotice
};
const Inline = {
  args: {
    kind: "fault",
    placement: "inline",
    code: "fleet.approve.refused",
    message: "Job 12 was not approved. The gate had already closed on step 3.",
    act: "Redispatch to start a fresh attempt from the same brief.",
    fields: [
      { label: "Job", value: "job_2d90bb" },
      { label: "Step", value: "3 of 5" },
      { label: "Fleet run", value: "01J9Z4K7QW" }
    ]
  }
};
const Toast$1 = {
  render: (args) => /* @__PURE__ */ jsx("div", { className: "armada-error-toast-region", children: /* @__PURE__ */ jsx(ErrorNotice, { ...args }) }),
  args: {
    kind: "fault",
    placement: "toast",
    code: "bridge.clipboard.denied",
    message: "The branch name did not copy. The system refused the clipboard write.",
    fields: [{ label: "Value", value: "auth/session-expiry" }]
  }
};
const Banner = {
  args: {
    kind: "degraded",
    placement: "banner",
    code: "bridge.fleet.unreachable",
    message: "The Job Board stopped updating 4 minutes ago. Fleet is alive on its port and has not answered.",
    act: "Nothing to do yet. Bridge reconnects on its own, and re-reads rather than patching what it has.",
    fields: [
      { label: "Pid", value: "48213" },
      { label: "Port", value: "7681" },
      { label: "Last read", value: "16:42:08" }
    ],
    actions: /* @__PURE__ */ jsx(Button, { variant: "secondary", size: "sm", children: "Retry now" })
  }
};
const BannerFault = {
  args: {
    kind: "fault",
    placement: "banner",
    code: "fleet.protocol.mismatch",
    message: "No Job dispatched since 14:02. Fleet speaks protocol 3 and Bridge speaks 4.",
    act: "Update Fleet, or run an older Bridge. The v0 routes still list and kill Jobs meanwhile.",
    fields: [
      { label: "Fleet", value: "0.4.1" },
      { label: "Bridge", value: "0.5.0" }
    ],
    actions: /* @__PURE__ */ jsx(Button, { variant: "secondary", size: "sm", children: "Open Doctor" })
  }
};
const FullSurface = {
  render: (args) => /* @__PURE__ */ jsx("div", { className: "armada-error-surface-region", children: /* @__PURE__ */ jsx(ErrorNotice, { ...args }) }),
  args: {
    kind: "fault",
    placement: "surface",
    code: "bridge.fleet.absent",
    message: "Fleet is not running, so there are no Jobs to show.",
    act: "Start Fleet from a terminal. Bridge cannot start it, and Jobs keep progressing once it is up.",
    fields: [
      { label: "Runtime file", value: "no file at its path" },
      { label: "Bridge run", value: "01J9Z4K7QW" }
    ],
    actions: /* @__PURE__ */ jsx(Button, { variant: "secondary", size: "sm", children: "Check again" })
  }
};
const InlineWithPayload = {
  args: { ...Inline.args, payload: REFUSED$1 }
};
const ToastWithPayload = {
  render: (args) => /* @__PURE__ */ jsx("div", { className: "armada-error-toast-region", children: /* @__PURE__ */ jsx(ErrorNotice, { ...args }) }),
  args: {
    kind: "fault",
    placement: "toast",
    code: "judge.undecided",
    message: "Job 31c7 escalated. The judge returned prose for criterion 2.",
    payload: REFUSED$1
  }
};
const BannerWithPayload = {
  args: { ...Banner.args, payload: REFUSED$1 }
};
const FullSurfaceWithPayload = {
  render: (args) => /* @__PURE__ */ jsx("div", { className: "armada-error-surface-region", children: /* @__PURE__ */ jsx(ErrorNotice, { ...args }) }),
  args: {
    kind: "fault",
    placement: "surface",
    code: "judge.undecided",
    message: "Job 31c7 escalated. The judge returned prose for criterion 2.",
    act: "Read the judge's response, then overrule the verdict or redispatch the job.",
    payload: REFUSED$1
  }
};
const CodelessPayload = {
  render: (args) => /* @__PURE__ */ jsx("div", { className: "armada-error-surface-region", children: /* @__PURE__ */ jsx(ErrorNotice, { ...args }) }),
  args: {
    kind: "fault",
    placement: "surface",
    code: "bridge.render.threw",
    message: "Bridge could not draw the job board.",
    act: "Reload Bridge. Fleet keeps running and jobs keep progressing.",
    payload: {
      message: "Cannot read properties of undefined (reading 'status')",
      fields: [
        { key: "region", value: "the job board" },
        { key: "component", value: "JobRowStacked" }
      ],
      chain: [
        "at JobRowStacked (JobRowStacked.tsx:88:19)",
        "at JobBoard (JobBoard.tsx:142:7)",
        "at Shell (Shell.tsx:61:5)"
      ],
      bridgeProtocol: "5.2",
      at: "2026-08-30T09:16:40Z"
    }
  }
};
const __vite_glob_0_38 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Banner,
  BannerFault,
  BannerWithPayload,
  CodelessPayload,
  FullSurface,
  FullSurfaceWithPayload,
  Inline,
  InlineWithPayload,
  Toast: Toast$1,
  ToastWithPayload,
  default: meta$x
}, Symbol.toStringTag, { value: "Module" }));
const REFUSED = {
  code: "judge.undecided",
  message: "judge returned prose for criterion 2",
  run_id: "01JQ8ZC4M2WYVK7T3RQN8H",
  job_id: "job_31c7",
  drone_id: "drn_4c8",
  step_id: "verify",
  fields: [
    { key: "criterion", value: "2" },
    { key: "judge_model", value: "sonnet" },
    { key: "response_bytes", value: "1184" }
  ],
  chain: [
    "judge: no verdict parsed from response",
    "gate verify: undecided",
    "job_31c7: escalated"
  ],
  bridgeProtocol: "5.2",
  fleetProtocol: "5.2",
  at: "2026-08-30T09:16:40Z"
};
const ENVELOPE = envelopeOf(REFUSED);
const meta$w = {
  title: "Errors/File an issue",
  component: FileAnIssue
};
const DIFF = {
  id: "worktree-diff",
  label: "What the drone changed",
  warning: "A patch is the contents of files on this machine, including any the drone read a secret out of.",
  body: [
    "diff --git a/crates/judge/src/parse.rs b/crates/judge/src/parse.rs",
    "@@ -18,7 +18,7 @@",
    "-    let verdict = line.split_once(':')?.1;",
    "+    let verdict = line.split_once(':').map(|pair| pair.1)?;"
  ].join("\n")
};
const TheControl = {
  args: {
    compose: () => ({ title: REFUSED.message, attached: [ENVELOPE], withheld: NOT_OFFERED })
  }
};
const OneItemAndItCannotBeRemoved = {
  render: () => /* @__PURE__ */ jsx(
    FilingReview,
    {
      offered: { title: REFUSED.message, attached: [ENVELOPE], withheld: NOT_OFFERED },
      removed: /* @__PURE__ */ new Set(),
      onToggle: () => void 0,
      onCancel: () => void 0,
      onCopy: () => void 0
    }
  )
};
const AnItemThatCanBeRemoved = {
  render: () => /* @__PURE__ */ jsx(
    FilingReview,
    {
      offered: { title: REFUSED.message, attached: [ENVELOPE, DIFF], withheld: NOT_OFFERED },
      removed: /* @__PURE__ */ new Set(),
      onToggle: () => void 0,
      onCancel: () => void 0,
      onCopy: () => void 0
    }
  )
};
const AnItemRemoved = {
  render: () => /* @__PURE__ */ jsx(
    FilingReview,
    {
      offered: { title: REFUSED.message, attached: [ENVELOPE, DIFF], withheld: NOT_OFFERED },
      removed: /* @__PURE__ */ new Set([DIFF.id]),
      onToggle: () => void 0,
      onCancel: () => void 0,
      onCopy: () => void 0
    }
  )
};
const __vite_glob_0_39 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AnItemRemoved,
  AnItemThatCanBeRemoved,
  OneItemAndItCannotBeRemoved,
  TheControl,
  default: meta$w
}, Symbol.toStringTag, { value: "Module" }));
function Alert({ tone = "escalated", title, children, icon, action }) {
  return /* @__PURE__ */ jsxs(
    "div",
    {
      className: [
        "armada-alert",
        tone === "neutral" ? "armada-alert--neutral" : "armada-alert--escalated",
        // A headline makes the copy a block, and the glyph belongs beside its
        // first line. Without one there is a single line to sit against, and
        // aligning to its top reads as a mistake rather than as alignment.
        title ? "armada-alert--stacked" : "armada-alert--single"
      ].join(" "),
      role: "status",
      children: [
        icon ? /* @__PURE__ */ jsx("span", { className: "armada-alert__glyph", children: icon }) : null,
        /* @__PURE__ */ jsxs("div", { className: "armada-alert__copy", children: [
          title ? /* @__PURE__ */ jsx("span", { className: "armada-alert__title", children: title }) : null,
          /* @__PURE__ */ jsx("span", { className: "armada-alert__body", children })
        ] }),
        action ? /* @__PURE__ */ jsx("span", { className: "armada-alert__action", children: action }) : null
      ]
    }
  );
}
const meta$v = {
  title: "Primitives/Alert",
  component: Alert
};
const Escalated$1 = {
  args: {
    tone: "escalated",
    icon: /* @__PURE__ */ jsx(TriangleAlert, { size: 16, strokeWidth: 2, "aria-hidden": "true" }),
    title: "Fleet lost the api/auth worktree",
    children: "The branch was deleted outside Armada. Two jobs are blocked."
  }
};
const Neutral = {
  args: {
    tone: "neutral",
    icon: /* @__PURE__ */ jsx(Stethoscope, { size: 16, strokeWidth: 2, "aria-hidden": "true" }),
    children: /* @__PURE__ */ jsxs("span", { children: [
      "Doctor: 1 of 6 modules failing — ",
      /* @__PURE__ */ jsx("span", { className: "mono", children: "toolchain" }),
      ", node 20 missing on this machine. Jobs still dispatch."
    ] }),
    action: /* @__PURE__ */ jsx("button", { type: "button", className: "armada-alert__button", children: "Open Doctor" })
  }
};
const __vite_glob_0_40 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Escalated: Escalated$1,
  Neutral,
  default: meta$v
}, Symbol.toStringTag, { value: "Module" }));
function AttachmentChip({ filename, onRemove }) {
  return /* @__PURE__ */ jsxs("span", { className: "armada-attachment-chip", children: [
    /* @__PURE__ */ jsx("span", { className: "armada-attachment-chip__name", children: filename }),
    onRemove !== void 0 && /* @__PURE__ */ jsx(
      "button",
      {
        type: "button",
        className: "armada-attachment-chip__remove",
        onClick: onRemove,
        "aria-label": `Remove ${filename}`,
        children: "×"
      }
    )
  ] });
}
const meta$u = {
  title: "Primitives/AttachmentChip",
  component: AttachmentChip
};
const Default$7 = {
  args: { filename: "screenshot.png", onRemove: () => {
  } }
};
const LongFilename = {
  args: {
    filename: "a-very-long-filename-that-should-truncate-rather-than-widen-the-row.png",
    onRemove: () => {
    }
  }
};
const ReadOnly = {
  args: { filename: "evidence.log" }
};
const __vite_glob_0_41 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Default: Default$7,
  LongFilename,
  ReadOnly,
  default: meta$u
}, Symbol.toStringTag, { value: "Module" }));
const meta$t = {
  title: "Primitives/Badge",
  component: Badge
};
const NO_GLYPH_IN_REGISTRY = void 0;
const NotStarted = {
  args: { status: "not-started", icon: NO_GLYPH_IN_REGISTRY, children: "Not started" }
};
const Queued$1 = {
  args: { status: "not-started", icon: Clock, children: "Queued" }
};
const QueuedOutOfHeadroom = {
  args: { status: "not-started", icon: Cpu, children: "Waiting on resources" }
};
const QueuedBlockedByDependency = {
  args: { status: "not-started", icon: Link, children: "Blocked by a dependency" }
};
const AwaitingApproval$1 = {
  args: { status: "awaiting-approval", icon: UserCheck, children: "Awaiting approval" }
};
const Running$2 = {
  args: { status: "running", icon: CircleDot, children: "Running" }
};
const RunningPulsing = {
  args: { status: "running", icon: CircleDot, children: "Running", pulsing: true }
};
const Piloted = {
  args: { status: "piloted", icon: Terminal, children: "Piloted" }
};
const AwaitingReview = {
  args: { status: "awaiting-review", icon: Eye, children: "Awaiting review" }
};
const AwaitingAttestation = {
  args: { status: "awaiting-attestation", icon: Stamp, children: "Awaiting attestation" }
};
const Escalated = {
  args: { status: "escalated", icon: NO_GLYPH_IN_REGISTRY, children: "Escalated" }
};
const CompletedSuccess = {
  args: { status: "completed-success", icon: Check, children: "Completed" }
};
const CompletedFailed = {
  args: { status: "completed-failed", icon: X, children: "Failed" }
};
const Rejected = {
  args: { status: "rejected", icon: Ban, children: "Rejected" }
};
const Killed$2 = {
  args: { status: "killed", icon: Power, children: "Killed" }
};
const Superseded = {
  args: { status: "superseded", icon: Archive, children: "Superseded" }
};
const EscalationReasons = {
  render: () => /* @__PURE__ */ jsxs("div", { style: { display: "flex", flexDirection: "column", alignItems: "flex-start", gap: "var(--space-2)" }, children: [
    /* @__PURE__ */ jsx(Badge, { status: "escalated", icon: OctagonAlert, children: "Stalled" }),
    /* @__PURE__ */ jsx(Badge, { status: "escalated", icon: RefreshCw, children: "Churning" }),
    /* @__PURE__ */ jsx(Badge, { status: "escalated", icon: FileQuestionMark, children: "Evidence disputed" }),
    /* @__PURE__ */ jsx(Badge, { status: "escalated", icon: ShieldX, children: "Check failed" }),
    /* @__PURE__ */ jsx(Badge, { status: "escalated", icon: Split, children: "Fanned out" }),
    /* @__PURE__ */ jsx(Badge, { status: "escalated", icon: Unplug, children: "Connection lost" }),
    /* @__PURE__ */ jsx(Badge, { status: "escalated", icon: ArrowUpToLine, children: "Reached its ceiling" })
  ] })
};
const __vite_glob_0_42 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AwaitingApproval: AwaitingApproval$1,
  AwaitingAttestation,
  AwaitingReview,
  CompletedFailed,
  CompletedSuccess,
  Escalated,
  EscalationReasons,
  Killed: Killed$2,
  NotStarted,
  Piloted,
  Queued: Queued$1,
  QueuedBlockedByDependency,
  QueuedOutOfHeadroom,
  Rejected,
  Running: Running$2,
  RunningPulsing,
  Superseded,
  default: meta$t
}, Symbol.toStringTag, { value: "Module" }));
const meta$s = {
  title: "Primitives/Button",
  component: Button
};
function Card$6({ children }) {
  return /* @__PURE__ */ jsx(
    "div",
    {
      style: {
        display: "flex",
        alignItems: "center",
        gap: "var(--space-3)",
        flexWrap: "wrap",
        padding: "var(--pad-card)",
        borderRadius: "var(--radius-md)",
        background: "var(--bg-raised)"
      },
      children
    }
  );
}
const Primary = {
  args: { variant: "primary", children: "Dispatch job" },
  render: (args) => /* @__PURE__ */ jsx(Card$6, { children: /* @__PURE__ */ jsx(Button, { ...args }) })
};
const Secondary = {
  args: { variant: "secondary", children: "Cancel" },
  render: (args) => /* @__PURE__ */ jsx(Card$6, { children: /* @__PURE__ */ jsx(Button, { ...args }) })
};
const Ghost = {
  args: { variant: "ghost", children: "Ghost" },
  render: (args) => /* @__PURE__ */ jsx(Card$6, { children: /* @__PURE__ */ jsx(Button, { ...args }) })
};
const Destructive = {
  args: { variant: "destructive", children: "Kill job" },
  render: (args) => /* @__PURE__ */ jsx(Card$6, { children: /* @__PURE__ */ jsx(Button, { ...args }) })
};
const Hover = {
  render: () => /* @__PURE__ */ jsxs(Card$6, { children: [
    /* @__PURE__ */ jsx(Button, { variant: "primary", "data-preview-hover": "", children: "Dispatch job" }),
    /* @__PURE__ */ jsx(Button, { variant: "secondary", "data-preview-hover": "", children: "Cancel" }),
    /* @__PURE__ */ jsx(Button, { variant: "ghost", "data-preview-hover": "", children: "Ghost" }),
    /* @__PURE__ */ jsx(Button, { variant: "destructive", "data-preview-hover": "", children: "Kill job" })
  ] })
};
const Focused$7 = {
  render: () => /* @__PURE__ */ jsx(Card$6, { children: /* @__PURE__ */ jsx(Button, { variant: "secondary", "data-preview-focus": "", children: "Focused" }) })
};
const Disabled$7 = {
  render: () => /* @__PURE__ */ jsxs(Card$6, { children: [
    /* @__PURE__ */ jsx(Button, { variant: "primary", disabled: true, children: "Dispatch job" }),
    /* @__PURE__ */ jsx(Button, { variant: "secondary", disabled: true, children: "Cancel" }),
    /* @__PURE__ */ jsx(Button, { variant: "destructive", disabled: true, children: "Kill job" })
  ] })
};
const Small = {
  render: () => /* @__PURE__ */ jsxs(Card$6, { children: [
    /* @__PURE__ */ jsx(Button, { size: "sm", children: "Review" }),
    /* @__PURE__ */ jsx(Button, { size: "sm", children: "Open diff" }),
    /* @__PURE__ */ jsx(Button, { size: "sm", variant: "ghost", iconOnly: true, "aria-label": "Retry", children: /* @__PURE__ */ jsx(RotateCw, { size: 16, strokeWidth: 2, "aria-hidden": "true" }) })
  ] })
};
const Group = {
  render: () => /* @__PURE__ */ jsxs(Card$6, { children: [
    /* @__PURE__ */ jsx(Button, { variant: "primary", children: "Approve" }),
    /* @__PURE__ */ jsx(Button, { variant: "secondary", children: "Open diff" }),
    /* @__PURE__ */ jsx(Button, { variant: "ghost", children: "Redirect" }),
    /* @__PURE__ */ jsx(Button, { variant: "destructive", children: "Kill" })
  ] })
};
const SecondaryOnASunkenGround = {
  render: () => /* @__PURE__ */ jsxs("div", { style: { display: "flex", flexDirection: "column", gap: "var(--space-4)" }, children: [
    /* @__PURE__ */ jsx(Card$6, { children: /* @__PURE__ */ jsx(Button, { variant: "secondary", ground: "card", children: "On a card" }) }),
    /* @__PURE__ */ jsxs(
      "div",
      {
        style: {
          display: "flex",
          gap: "var(--space-3)",
          padding: "var(--pad-card)",
          borderRadius: "var(--radius-md)",
          background: "var(--bg-sunken)"
        },
        children: [
          /* @__PURE__ */ jsx(Button, { variant: "secondary", ground: "sunken", children: "On a sunken row" }),
          /* @__PURE__ */ jsx(Button, { variant: "secondary", ground: "card", children: "Wrong ground" })
        ]
      }
    )
  ] })
};
const Light$7 = {
  render: () => /* @__PURE__ */ jsx("div", { "data-theme": "light", children: /* @__PURE__ */ jsxs(Card$6, { children: [
    /* @__PURE__ */ jsx(Button, { variant: "primary", children: "Dispatch job" }),
    /* @__PURE__ */ jsx(Button, { variant: "secondary", children: "Cancel" })
  ] }) })
};
const __vite_glob_0_43 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Destructive,
  Disabled: Disabled$7,
  Focused: Focused$7,
  Ghost,
  Group,
  Hover,
  Light: Light$7,
  Primary,
  Secondary,
  SecondaryOnASunkenGround,
  Small,
  default: meta$s
}, Symbol.toStringTag, { value: "Module" }));
const meta$r = {
  title: "Primitives/Card",
  component: Card$7,
  decorators: [
    (Story) => /* @__PURE__ */ jsx("div", { style: { maxWidth: "56ch" }, children: /* @__PURE__ */ jsx(Story, {}) })
  ]
};
const Default$6 = {
  render: () => /* @__PURE__ */ jsxs(Card$7, { children: [
    /* @__PURE__ */ jsx(CardTitle, { children: "Evidence accepted at step 4 of 5" }),
    /* @__PURE__ */ jsx(CardDescription, { children: "Four criteria resolved. One test added, none removed." })
  ] })
};
const WithHeader = {
  render: () => /* @__PURE__ */ jsxs(Card$7, { children: [
    /* @__PURE__ */ jsxs(CardHeader, { children: [
      /* @__PURE__ */ jsx("span", { className: "caps", children: "Evidence" }),
      /* @__PURE__ */ jsx(Badge, { status: "awaiting-review", icon: Eye, children: "Awaiting review" })
    ] }),
    /* @__PURE__ */ jsx(CardTitle, { children: "Evidence accepted at step 4 of 5" }),
    /* @__PURE__ */ jsx(CardDescription, { children: "Four criteria resolved. One test added, none removed." }),
    /* @__PURE__ */ jsx(CardContent, { children: /* @__PURE__ */ jsxs("span", { style: { color: "var(--fg-muted)", fontSize: "var(--text-xs)" }, children: [
      "Verification source ",
      /* @__PURE__ */ jsx("span", { className: "mono", children: "Judge" })
    ] }) }),
    /* @__PURE__ */ jsx(CardFooter, { children: /* @__PURE__ */ jsxs("span", { style: { color: "var(--fg-subtle)", fontSize: "var(--text-2xs)" }, children: [
      "Read at ",
      /* @__PURE__ */ jsx("span", { className: "mono", children: "14:22" })
    ] }) })
  ] })
};
const Dimmed$1 = {
  render: () => /* @__PURE__ */ jsxs(Card$7, { "data-dimmed": true, children: [
    /* @__PURE__ */ jsx(CardTitle, { children: "Superseded at step 2 of 5" }),
    /* @__PURE__ */ jsx(CardDescription, { children: "The work landed outside this job." })
  ] })
};
const __vite_glob_0_44 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Default: Default$6,
  Dimmed: Dimmed$1,
  WithHeader,
  default: meta$r
}, Symbol.toStringTag, { value: "Module" }));
const meta$q = {
  title: "Primitives/Checkbox",
  component: Checkbox
};
function Card$5({ children }) {
  return /* @__PURE__ */ jsx(
    "div",
    {
      style: {
        display: "flex",
        flexDirection: "column",
        gap: "var(--space-2)",
        width: "var(--sidebar-max)",
        padding: "var(--pad-card)",
        borderRadius: "var(--radius-md)",
        background: "var(--bg-raised)"
      },
      children
    }
  );
}
const Unchecked = {
  render: () => /* @__PURE__ */ jsx(Card$5, { children: /* @__PURE__ */ jsx(Checkbox, { children: "Land as a convoy" }) })
};
const Checked = {
  render: () => /* @__PURE__ */ jsx(Card$5, { children: /* @__PURE__ */ jsx(Checkbox, { defaultChecked: true, children: "Run Doctor before dispatch" }) })
};
const Focused$6 = {
  render: () => /* @__PURE__ */ jsx(Card$5, { children: /* @__PURE__ */ jsx(Checkbox, { defaultChecked: true, "data-preview-focus": "", children: "Run Doctor before dispatch" }) })
};
const Disabled$6 = {
  render: () => /* @__PURE__ */ jsxs(Card$5, { children: [
    /* @__PURE__ */ jsx(Checkbox, { disabled: true, children: "Land as a convoy" }),
    /* @__PURE__ */ jsx(Checkbox, { disabled: true, defaultChecked: true, children: "Run Doctor before dispatch" })
  ] })
};
const Light$6 = {
  render: () => /* @__PURE__ */ jsx("div", { "data-theme": "light", children: /* @__PURE__ */ jsx(Card$5, { children: /* @__PURE__ */ jsx(Checkbox, { defaultChecked: true, children: "Run Doctor before dispatch" }) }) })
};
const __vite_glob_0_45 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Checked,
  Disabled: Disabled$6,
  Focused: Focused$6,
  Light: Light$6,
  Unchecked,
  default: meta$q
}, Symbol.toStringTag, { value: "Module" }));
const DEFAULT_SECTIONS = ["Actions", "Navigation", "Jobs", "Settings"];
function matches(entry, query) {
  if (!query) return true;
  const q = query.toLowerCase();
  if (entry.label.toLowerCase().includes(q)) return true;
  return (entry.aliases ?? []).some((alias) => alias.toLowerCase().includes(q));
}
function CommandPalette({
  open: open2,
  entries: entries2,
  sectionOrder = DEFAULT_SECTIONS,
  placeholder = "Search actions, jobs and settings",
  defaultQuery = "",
  onSelect,
  onConfirm,
  onClose
}) {
  const [query, setQuery] = useState(defaultQuery);
  const [at, setAt] = useState(0);
  const input = useRef(null);
  const results = useMemo(() => {
    const kept = entries2.filter((entry) => matches(entry, query));
    const rank = (section) => {
      const found = sectionOrder.indexOf(section);
      return found === -1 ? sectionOrder.length : found;
    };
    return kept.map((entry, i) => ({ entry, i })).sort((a, b) => rank(a.entry.section) - rank(b.entry.section) || a.i - b.i).map((held) => held.entry);
  }, [entries2, query, sectionOrder]);
  useEffect(() => setAt(0), [query]);
  useEffect(() => {
    if (open2) input.current?.focus();
  }, [open2]);
  if (!open2) return null;
  function choose(entry) {
    if (!entry) return;
    if (entry.destructive) {
      onConfirm?.(entry);
      return;
    }
    onSelect?.(entry);
    onClose?.();
  }
  function onKey(event) {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setAt((n) => results.length ? (n + 1) % results.length : 0);
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      setAt((n) => results.length ? (n - 1 + results.length) % results.length : 0);
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      choose(results[at]);
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      onClose?.();
    }
  }
  let lastSection = "";
  return /* @__PURE__ */ jsx("div", { className: "armada-palette-layer", children: /* @__PURE__ */ jsxs("div", { className: "armada-palette", role: "dialog", "aria-modal": "true", "aria-label": "Command palette", children: [
    /* @__PURE__ */ jsx(
      "input",
      {
        ref: input,
        className: "armada-palette__input",
        type: "text",
        role: "combobox",
        "aria-expanded": "true",
        "aria-controls": "armada-palette-list",
        placeholder,
        value: query,
        onChange: (event) => setQuery(event.target.value),
        onKeyDown: onKey
      }
    ),
    /* @__PURE__ */ jsxs("div", { className: "armada-palette__list", id: "armada-palette-list", role: "listbox", children: [
      results.length === 0 ? /* @__PURE__ */ jsx("p", { className: "armada-palette__empty", children: "No match. Every action Armada has is in this list, under the name Armada uses for it." }) : null,
      results.map((entry, n) => {
        const heading2 = entry.section !== lastSection ? entry.section : "";
        lastSection = entry.section;
        return /* @__PURE__ */ jsxs("div", { children: [
          heading2 ? /* @__PURE__ */ jsx("div", { className: "armada-palette__section", children: heading2 }) : null,
          /* @__PURE__ */ jsxs(
            "button",
            {
              type: "button",
              role: "option",
              "aria-selected": n === at,
              className: n === at ? "armada-palette__row armada-palette__row--active" : "armada-palette__row",
              onMouseEnter: () => setAt(n),
              onClick: () => choose(entry),
              children: [
                /* @__PURE__ */ jsx("span", { className: "armada-palette__glyph", children: entry.icon }),
                /* @__PURE__ */ jsx("span", { className: "armada-palette__label", children: entry.label }),
                /* @__PURE__ */ jsx("kbd", { className: "armada-palette__kbd", children: entry.shortcut })
              ]
            }
          )
        ] }, entry.id);
      })
    ] })
  ] }) });
}
const meta$p = {
  title: "Primitives/CommandPalette",
  component: CommandPalette
};
const glyph = { size: 12, strokeWidth: 2, "aria-hidden": true };
const entries = [
  {
    id: "dispatch",
    section: "Actions",
    label: "Dispatch a job",
    aliases: ["start", "launch", "new"],
    shortcut: "⌘D",
    icon: /* @__PURE__ */ jsx(Send, { ...glyph })
  },
  {
    id: "approve",
    section: "Actions",
    label: "Approve dispatch",
    aliases: ["accept", "ok"],
    shortcut: "a",
    icon: /* @__PURE__ */ jsx(Eye, { ...glyph })
  },
  {
    id: "redirect",
    section: "Actions",
    label: "Redirect the drone",
    aliases: ["steer", "correct"],
    shortcut: "r",
    icon: /* @__PURE__ */ jsx(CornerUpRight, { ...glyph })
  },
  {
    id: "kill",
    section: "Actions",
    label: "Kill the drone",
    aliases: ["terminate", "stop", "abort"],
    shortcut: "x",
    icon: /* @__PURE__ */ jsx(Power, { ...glyph }),
    destructive: true
  },
  {
    id: "board",
    section: "Navigation",
    label: "Job Board",
    shortcut: "⌘1",
    icon: /* @__PURE__ */ jsx(ClipboardList, { ...glyph })
  },
  {
    id: "active",
    section: "Navigation",
    label: "Active jobs",
    shortcut: "⌘2",
    icon: /* @__PURE__ */ jsx(Activity, { ...glyph })
  },
  {
    id: "alerts",
    section: "Navigation",
    label: "Alerts",
    shortcut: "⌘3",
    icon: /* @__PURE__ */ jsx(Bell, { ...glyph })
  },
  {
    id: "doctor",
    section: "Navigation",
    label: "Doctor",
    shortcut: "⌘6",
    icon: /* @__PURE__ */ jsx(Stethoscope, { ...glyph })
  },
  {
    id: "helm",
    section: "Navigation",
    label: "Helm",
    shortcut: "⌘8",
    icon: /* @__PURE__ */ jsx(MessageSquare, { ...glyph })
  },
  {
    id: "job-8f2a1c",
    section: "Jobs",
    label: "job_8f2a1c — split the settings reducer",
    shortcut: "↵",
    icon: /* @__PURE__ */ jsx(CircleDot, { ...glyph })
  },
  {
    id: "job-2d90bb",
    section: "Jobs",
    label: "job_2d90bb — coalesce the session refresh",
    shortcut: "↵",
    icon: /* @__PURE__ */ jsx(Eye, { ...glyph })
  },
  {
    id: "kit",
    section: "Settings",
    label: "Kit",
    aliases: ["skills", "allowlist", "mcp"],
    shortcut: "⌘,",
    icon: /* @__PURE__ */ jsx(Settings, { ...glyph })
  }
];
const Resting$1 = {
  args: { open: true, entries }
};
const AliasFindsTheLexiconTerm = {
  args: { open: true, entries, defaultQuery: "terminate" }
};
const NoMatch = {
  args: { open: true, entries, defaultQuery: "zzz" }
};
const DestructiveEntryConfirms = {
  render: () => {
    const [pending, setPending] = useState(void 0);
    return /* @__PURE__ */ jsxs(Fragment, { children: [
      /* @__PURE__ */ jsx(
        CommandPalette,
        {
          open: true,
          entries,
          defaultQuery: "kill",
          onConfirm: (entry) => setPending(entry)
        }
      ),
      /* @__PURE__ */ jsx(
        Dialog,
        {
          open: pending !== void 0,
          tone: "destructive",
          title: "Kill the drone on job 12?",
          confirmLabel: "Kill job",
          onCancel: () => setPending(void 0),
          onConfirm: () => setPending(void 0),
          children: "Step 3 of 5, 18 minutes in. The worktree is left in place and evidence carries forward if you redispatch."
        }
      )
    ] });
  }
};
const __vite_glob_0_46 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AliasFindsTheLexiconTerm,
  DestructiveEntryConfirms,
  NoMatch,
  Resting: Resting$1,
  default: meta$p
}, Symbol.toStringTag, { value: "Module" }));
const meta$o = {
  title: "Primitives/Dialog",
  component: Dialog
};
const Confirmation = {
  args: {
    open: true,
    tone: "destructive",
    title: "Kill the drone on job 12?",
    confirmLabel: "Kill job",
    cancelLabel: "Cancel",
    children: "Step 3 of 5, 18 minutes in. The worktree is left in place and evidence carries forward if you redispatch."
  }
};
const NeutralConfirm = {
  args: {
    open: true,
    tone: "neutral",
    title: "Kill drone 4 and dispatch a replacement",
    confirmLabel: "Kill and redispatch",
    cancelLabel: "Cancel",
    children: "Ends job 12 and opens a new job on the same workspace and branch. The worktree is kept."
  }
};
const KillTheDrone = {
  args: {
    open: true,
    tone: "destructive",
    title: "Kill the drone on this job?",
    confirmLabel: "Kill drone",
    children: "The process stops and the job stays open. Its worktree is held as the drone left it, so the job can be redispatched from where it got to."
  }
};
const KillTheJob = {
  args: {
    open: true,
    tone: "destructive",
    title: "Kill this job?",
    confirmLabel: "Kill job",
    children: "The job ends at killed. That is terminal and carries no verdict — nothing resumes it, and anything the drone wrote stays on its branch."
  }
};
const RedispatchAsANewJob = {
  args: {
    open: true,
    tone: "destructive",
    title: "Redispatch this job as a new one?",
    confirmLabel: "Redispatch as a new job",
    children: "This job is killed and a replacement is created carrying a reference back to it. Nothing resumes: the new job starts at the approval gate and needs releasing. The failed job's worktree and branch are left as its drone left them."
  }
};
const RestartTheStep = {
  args: {
    open: true,
    tone: "neutral",
    title: "Restart this step?",
    confirmLabel: "Restart step",
    children: "A fresh drone takes over on the same worktree, at the step the last one stopped at. The toolset, model and environment are resolved again from scratch, so a widened scope can only narrow — and where the worktree itself is gone, Fleet refuses this and names a redispatch instead."
  }
};
const MoreThanFitsWithAFieldToReach = {
  args: {
    open: true,
    tone: "neutral",
    width: "wide",
    title: "Overrule the gaming flag on this step?",
    confirmLabel: "Overrule the flag",
    field: /* @__PURE__ */ jsx(Textarea, { label: "Why the flag is wrong", rows: 3 }),
    children: /* @__PURE__ */ jsxs(Fragment, { children: [
      /* @__PURE__ */ jsx("p", { children: "The gaming check flagged the evidence for Regression check. It did not refuse the work — it says the evidence for it is not to be trusted. Overruling says a person has read that evidence and takes responsibility for it; the step advances still recorded as failed against the flag." }),
      /* @__PURE__ */ jsx("p", { children: "It is not the last step, so the job carries on at the next one. Your reason is written to this job's log and stays there — the log is append-only, and nothing takes an override back. It is not sent to the drone, which did nothing wrong and is told only that the step was accepted." }),
      /* @__PURE__ */ jsx("p", { children: "The check reads the diff and the evidence together and infers intent, which is why it is its own escalation rather than a gate failure: resubmitting under the same instructions would likely reproduce whatever it found, so the job stops and asks rather than retrying." }),
      /* @__PURE__ */ jsx("p", { children: "A person overruling it is on the record as having read the finding. That is the whole reason this is a dialog and not a button." })
    ] })
  }
};
const __vite_glob_0_47 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Confirmation,
  KillTheDrone,
  KillTheJob,
  MoreThanFitsWithAFieldToReach,
  NeutralConfirm,
  RedispatchAsANewJob,
  RestartTheStep,
  default: meta$o
}, Symbol.toStringTag, { value: "Module" }));
function DropdownMenu({
  triggerLabel,
  entries: entries2,
  defaultOpen = false,
  onSelect
}) {
  const [open2, setOpen] = useState(defaultOpen);
  const root = useRef(null);
  useEffect(() => {
    if (!open2) return;
    function onKey(event) {
      if (event.key === "Escape") setOpen(false);
    }
    function onDown(event) {
      if (!root.current?.contains(event.target)) setOpen(false);
    }
    window.addEventListener("keydown", onKey);
    window.addEventListener("mousedown", onDown);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("mousedown", onDown);
    };
  }, [open2]);
  return /* @__PURE__ */ jsxs("div", { className: "armada-dropdown-menu", ref: root, children: [
    /* @__PURE__ */ jsx(
      "button",
      {
        type: "button",
        className: "armada-dropdown-menu__trigger",
        "aria-haspopup": "menu",
        "aria-expanded": open2,
        onClick: () => setOpen((v) => !v),
        children: triggerLabel
      }
    ),
    open2 ? /* @__PURE__ */ jsx("div", { className: "armada-dropdown-menu__panel", role: "menu", children: entries2.map((entry) => {
      if (entry.kind === "separator") {
        return /* @__PURE__ */ jsx("div", { className: "armada-dropdown-menu__separator", role: "separator" }, entry.id);
      }
      if (entry.kind === "label") {
        return /* @__PURE__ */ jsx("div", { className: "armada-dropdown-menu__label", children: entry.label }, entry.id);
      }
      return /* @__PURE__ */ jsxs(
        "button",
        {
          type: "button",
          role: "menuitem",
          className: entry.danger ? "armada-dropdown-menu__item armada-dropdown-menu__item--danger" : "armada-dropdown-menu__item",
          onClick: () => {
            setOpen(false);
            onSelect?.(entry.id);
          },
          children: [
            /* @__PURE__ */ jsx("span", { className: "armada-dropdown-menu__text", children: entry.label }),
            entry.shortcut ? /* @__PURE__ */ jsx("kbd", { className: "armada-dropdown-menu__kbd", children: entry.shortcut }) : null
          ]
        },
        entry.id
      );
    }) }) : null
  ] });
}
const meta$n = {
  title: "Primitives/DropdownMenu",
  component: DropdownMenu
};
const RowMenu = {
  args: {
    defaultOpen: true,
    triggerLabel: "More",
    entries: [
      { kind: "item", id: "worktree", label: "Open the worktree" },
      { kind: "item", id: "copy", label: "Copy job ID", shortcut: "⌘C" },
      { kind: "item", id: "board", label: "Send back to the Job Board" },
      { kind: "separator", id: "rule" },
      { kind: "item", id: "kill", label: "Kill job", shortcut: "x", danger: true }
    ]
  }
};
const WithSectionLabels = {
  args: {
    defaultOpen: true,
    triggerLabel: "More",
    entries: [
      { kind: "label", id: "l-job", label: "This job" },
      { kind: "item", id: "worktree", label: "Open the worktree" },
      { kind: "item", id: "copy", label: "Copy job ID", shortcut: "⌘C" },
      { kind: "separator", id: "rule" },
      { kind: "label", id: "l-fleet", label: "Fleet" },
      { kind: "item", id: "pause", label: "Freeze dispatch" },
      { kind: "item", id: "kill", label: "Kill job", shortcut: "x", danger: true }
    ]
  }
};
const rowActions = [
  { kind: "item", id: "worktree", label: "Open the worktree" },
  { kind: "item", id: "copy", label: "Copy job ID", shortcut: "⌘C" },
  { kind: "item", id: "board", label: "Send back to the Job Board" },
  { kind: "separator", id: "rule" },
  { kind: "item", id: "kill", label: "Kill job", shortcut: "x", danger: true }
];
function Frame$2({ edge, children }) {
  return /* @__PURE__ */ jsx("div", { className: `armada-dropdown-menu-frame armada-dropdown-menu-frame--${edge}`, children });
}
const AtTheLeftEdge$2 = {
  render: () => /* @__PURE__ */ jsx(Frame$2, { edge: "left", children: /* @__PURE__ */ jsx(DropdownMenu, { defaultOpen: true, triggerLabel: "More", entries: rowActions }) })
};
const AtTheRightEdge$2 = {
  render: () => /* @__PURE__ */ jsx(Frame$2, { edge: "right", children: /* @__PURE__ */ jsx(DropdownMenu, { defaultOpen: true, triggerLabel: "More", entries: rowActions }) })
};
const WithNoRoomBelow$2 = {
  render: () => /* @__PURE__ */ jsx(Frame$2, { edge: "bottom", children: /* @__PURE__ */ jsx(DropdownMenu, { defaultOpen: true, triggerLabel: "More", entries: rowActions }) })
};
const __vite_glob_0_48 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AtTheLeftEdge: AtTheLeftEdge$2,
  AtTheRightEdge: AtTheRightEdge$2,
  RowMenu,
  WithNoRoomBelow: WithNoRoomBelow$2,
  WithSectionLabels,
  default: meta$n
}, Symbol.toStringTag, { value: "Module" }));
const meta$m = {
  title: "Primitives/Input",
  component: Input
};
function Card$4({ children }) {
  return /* @__PURE__ */ jsx(
    "div",
    {
      style: {
        display: "flex",
        flexDirection: "column",
        gap: "var(--space-4)",
        width: "var(--sidebar-max)",
        padding: "var(--pad-card)",
        borderRadius: "var(--radius-md)",
        background: "var(--bg-raised)"
      },
      children
    }
  );
}
const Default$5 = {
  args: { label: "Job title", defaultValue: "Refresh the auth token flow" },
  render: (args) => /* @__PURE__ */ jsx(Card$4, { children: /* @__PURE__ */ jsx(Input, { ...args }) })
};
const Placeholder$1 = {
  args: { label: "Job title", placeholder: "Refresh the auth token flow" },
  render: (args) => /* @__PURE__ */ jsx(Card$4, { children: /* @__PURE__ */ jsx(Input, { ...args }) })
};
const Mono = {
  args: { label: "Project location", defaultValue: "~/code/armada", mono: true },
  render: (args) => /* @__PURE__ */ jsx(Card$4, { children: /* @__PURE__ */ jsx(Input, { ...args }) })
};
const Focused$5 = {
  render: () => /* @__PURE__ */ jsx(Card$4, { children: /* @__PURE__ */ jsx(Input, { label: "Job title", defaultValue: "Refresh the auth token flow", "data-preview-focus": "" }) })
};
const Invalid$2 = {
  render: () => /* @__PURE__ */ jsx(Card$4, { children: /* @__PURE__ */ jsx(
    Input,
    {
      label: "Branch",
      defaultValue: "feat/auth",
      mono: true,
      invalid: true,
      message: "Branch already exists in the workspace."
    }
  ) })
};
const Disabled$5 = {
  render: () => /* @__PURE__ */ jsx(Card$4, { children: /* @__PURE__ */ jsx(Input, { label: "Job title", defaultValue: "Refresh the auth token flow", disabled: true }) })
};
const Light$5 = {
  render: () => /* @__PURE__ */ jsx("div", { "data-theme": "light", children: /* @__PURE__ */ jsx(Card$4, { children: /* @__PURE__ */ jsx(Input, { label: "Job title", defaultValue: "Refresh the auth token flow" }) }) })
};
const __vite_glob_0_49 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Default: Default$5,
  Disabled: Disabled$5,
  Focused: Focused$5,
  Invalid: Invalid$2,
  Light: Light$5,
  Mono,
  Placeholder: Placeholder$1,
  default: meta$m
}, Symbol.toStringTag, { value: "Module" }));
const meta$l = {
  title: "Primitives/kbd",
  component: Kbd
};
const Default$4 = {
  args: { children: "Esc" }
};
const Chord = {
  render: () => /* @__PURE__ */ jsxs(KbdChord, { children: [
    /* @__PURE__ */ jsx(Kbd, { children: "⌘" }),
    /* @__PURE__ */ jsx(Kbd, { children: "K" })
  ] })
};
const ContextualKeys = {
  render: () => /* @__PURE__ */ jsxs("div", { style: { display: "flex", gap: "var(--space-2)", alignItems: "center" }, children: [
    /* @__PURE__ */ jsx(Kbd, { children: "j" }),
    /* @__PURE__ */ jsx(Kbd, { children: "k" }),
    /* @__PURE__ */ jsx(Kbd, { children: "Enter" }),
    /* @__PURE__ */ jsx(Kbd, { children: "a" }),
    /* @__PURE__ */ jsx(Kbd, { children: "r" }),
    /* @__PURE__ */ jsx(Kbd, { children: "x" }),
    /* @__PURE__ */ jsx(Kbd, { children: "/" })
  ] })
};
const __vite_glob_0_50 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Chord,
  ContextualKeys,
  Default: Default$4,
  default: meta$l
}, Symbol.toStringTag, { value: "Module" }));
function Popover({ trigger: trigger2, children, align = "start", defaultOpen = false }) {
  const [open2, setOpen] = useState(defaultOpen);
  const root = useRef(null);
  useEffect(() => {
    if (!open2) return;
    function onKey(event) {
      if (event.key === "Escape") setOpen(false);
    }
    function onDown(event) {
      if (!root.current?.contains(event.target)) setOpen(false);
    }
    window.addEventListener("keydown", onKey);
    window.addEventListener("mousedown", onDown);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("mousedown", onDown);
    };
  }, [open2]);
  return /* @__PURE__ */ jsxs("div", { className: "armada-popover", ref: root, children: [
    /* @__PURE__ */ jsx("span", { className: "armada-popover__trigger", onClick: () => setOpen((v) => !v), children: trigger2 }),
    open2 ? /* @__PURE__ */ jsx(
      "div",
      {
        className: align === "end" ? "armada-popover__panel armada-popover__panel--end" : "armada-popover__panel armada-popover__panel--start",
        role: "dialog",
        children
      }
    ) : null
  ] });
}
const meta$k = {
  title: "Primitives/Popover",
  component: Popover
};
const trigger = /* @__PURE__ */ jsx("button", { type: "button", className: "armada-popover__button", children: "Spend" });
const Open$1 = {
  args: {
    defaultOpen: true,
    trigger,
    children: /* @__PURE__ */ jsxs("span", { children: [
      "Spend follows the active billing mode. This machine gates on the quota floor, so the row shows ",
      /* @__PURE__ */ jsx("span", { className: "mono", children: "68% quota" }),
      " left."
    ] })
  }
};
const AlignedToTheEnd = {
  args: {
    defaultOpen: true,
    align: "end",
    trigger,
    children: /* @__PURE__ */ jsxs("span", { children: [
      "Spend follows the active billing mode. This machine gates on the quota floor, so the row shows ",
      /* @__PURE__ */ jsx("span", { className: "mono", children: "68% quota" }),
      " left."
    ] })
  }
};
const body = /* @__PURE__ */ jsxs("span", { children: [
  "Spend follows the active billing mode. This machine gates on the quota floor, so the row shows ",
  /* @__PURE__ */ jsx("span", { className: "mono", children: "68% quota" }),
  " left."
] });
function Frame$1({ edge, children }) {
  return /* @__PURE__ */ jsx("div", { className: `armada-popover-frame armada-popover-frame--${edge}`, children });
}
const AtTheLeftEdge$1 = {
  render: () => /* @__PURE__ */ jsx(Frame$1, { edge: "left", children: /* @__PURE__ */ jsx(Popover, { defaultOpen: true, align: "end", trigger, children: body }) })
};
const AtTheRightEdge$1 = {
  render: () => /* @__PURE__ */ jsx(Frame$1, { edge: "right", children: /* @__PURE__ */ jsx(Popover, { defaultOpen: true, align: "start", trigger, children: body }) })
};
const WithNoRoomBelow$1 = {
  render: () => /* @__PURE__ */ jsx(Frame$1, { edge: "bottom", children: /* @__PURE__ */ jsx(Popover, { defaultOpen: true, trigger, children: body }) })
};
const __vite_glob_0_51 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AlignedToTheEnd,
  AtTheLeftEdge: AtTheLeftEdge$1,
  AtTheRightEdge: AtTheRightEdge$1,
  Open: Open$1,
  WithNoRoomBelow: WithNoRoomBelow$1,
  default: meta$k
}, Symbol.toStringTag, { value: "Module" }));
const meta$j = {
  title: "Primitives/Prose",
  component: Prose
};
const AJudgesConsequence = {
  args: {
    text: 'The route table is walked once per operation and the loop skips `forget_job`, so the assertion that every operation is served passes without ever reading the one operation the step was about.\n\nThe skip is in `crates/api/src/tests/served.rs` and reads:\n\n```\nif route.operation == "forget_job" { continue; }\n```\n\nNothing else in the file narrows the set, so the count the assertion compares against was lowered by the same edit that made it pass.'
  }
};
const AFlagsCitation = {
  args: {
    text: '`crates/api/src/tests/served.rs:214` — the `served_every_operation` assertion counts `ROUTES.len()` after the filter rather than before it:\n\n```\nlet routes: Vec<&Route> = ROUTES.iter().filter(|r| r.operation != "forget_job").collect();\nassert_eq!(routes.len(), served.len());\n```'
  }
};
const AListAndAHeading = {
  args: {
    text: "# What the check read\n\nThree things, and the third is the one that matters:\n\n- the diff touches `armada.yml`\n- the touched key is `checks.tests.command`\n- the command it was changed **to** exits 0 on an empty test set\n\nThe first two alone are ordinary. Together with the third they are the pattern."
  }
};
const WhatItWillNotDraw = {
  args: {
    text: "A link is written [like this](https://example.invalid/x) and stays written that way.\n\n> A blockquote is a paragraph that opens with a caret.\n\n| so | is | a table |"
  }
};
const __vite_glob_0_52 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AFlagsCitation,
  AJudgesConsequence,
  AListAndAHeading,
  WhatItWillNotDraw,
  default: meta$j
}, Symbol.toStringTag, { value: "Module" }));
const meta$i = {
  title: "Primitives/Radio",
  component: Radio
};
function Card$3({ children }) {
  return /* @__PURE__ */ jsx(
    "div",
    {
      style: {
        display: "flex",
        flexDirection: "column",
        gap: "var(--space-4)",
        width: "var(--sidebar-max)",
        padding: "var(--pad-card)",
        borderRadius: "var(--radius-md)",
        background: "var(--bg-raised)"
      },
      children
    }
  );
}
const Default$3 = {
  render: () => /* @__PURE__ */ jsx(Card$3, { children: /* @__PURE__ */ jsxs(RadioGroup, { label: "Kit source", children: [
    /* @__PURE__ */ jsx(Radio, { name: "kit", defaultChecked: true, children: "Import an existing Kit" }),
    /* @__PURE__ */ jsx(Radio, { name: "kit", children: "Start fresh" })
  ] }) })
};
const Focused$4 = {
  render: () => /* @__PURE__ */ jsx(Card$3, { children: /* @__PURE__ */ jsxs(RadioGroup, { label: "Kit source", children: [
    /* @__PURE__ */ jsx(Radio, { name: "kit-focus", defaultChecked: true, "data-preview-focus": "", children: "Import an existing Kit" }),
    /* @__PURE__ */ jsx(Radio, { name: "kit-focus", children: "Start fresh" })
  ] }) })
};
const Disabled$4 = {
  render: () => /* @__PURE__ */ jsx(Card$3, { children: /* @__PURE__ */ jsxs(RadioGroup, { label: "Kit source", children: [
    /* @__PURE__ */ jsx(Radio, { name: "kit-disabled", defaultChecked: true, disabled: true, children: "Import an existing Kit" }),
    /* @__PURE__ */ jsx(Radio, { name: "kit-disabled", disabled: true, children: "Start fresh" })
  ] }) })
};
const Light$4 = {
  render: () => /* @__PURE__ */ jsx("div", { "data-theme": "light", children: /* @__PURE__ */ jsx(Card$3, { children: /* @__PURE__ */ jsxs(RadioGroup, { label: "Kit source", children: [
    /* @__PURE__ */ jsx(Radio, { name: "kit-light", defaultChecked: true, children: "Import an existing Kit" }),
    /* @__PURE__ */ jsx(Radio, { name: "kit-light", children: "Start fresh" })
  ] }) }) })
};
const __vite_glob_0_53 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Default: Default$3,
  Disabled: Disabled$4,
  Focused: Focused$4,
  Light: Light$4,
  default: meta$i
}, Symbol.toStringTag, { value: "Module" }));
function joined(base, extra) {
  return extra ? `${base} ${extra}` : base;
}
function Table({ className, ...rest }) {
  return /* @__PURE__ */ jsx("table", { className: joined("armada-table", className), ...rest });
}
function TableHead({ className, ...rest }) {
  return /* @__PURE__ */ jsx("thead", { className: joined("armada-table-head", className), ...rest });
}
function TableBody({ className, ...rest }) {
  return /* @__PURE__ */ jsx("tbody", { className: joined("armada-table-body", className), ...rest });
}
function TableRow({ className, selected, focused, dimmed, ...rest }) {
  return /* @__PURE__ */ jsx(
    "tr",
    {
      className: joined("armada-table-row", className),
      "data-selected": selected || void 0,
      "data-focused": focused || void 0,
      "data-dimmed": dimmed || void 0,
      ...rest
    }
  );
}
function TableHeaderCell({ className, ...rest }) {
  return /* @__PURE__ */ jsx("th", { scope: "col", className: joined("armada-table-header-cell", className), ...rest });
}
function TableCell({
  className,
  variant = "primary",
  copyValue,
  onCopied,
  truncates,
  onClick,
  ...rest
}) {
  const copies = copyValue !== void 0;
  const handleClick = useCallback(
    (event) => {
      onClick?.(event);
      if (copyValue === void 0) return;
      void navigator.clipboard.writeText(copyValue).then(
        () => onCopied?.(copyValue),
        // A failed clipboard write is otherwise indistinguishable from a dead
        // element, so the surface is told either way and says which happened.
        () => onCopied?.(copyValue)
      );
    },
    [copyValue, onCopied, onClick]
  );
  return /* @__PURE__ */ jsx(
    "td",
    {
      className: joined("armada-table-cell", className),
      "data-variant": variant,
      "data-copies": copies || void 0,
      "data-truncates": truncates || void 0,
      onClick: handleClick,
      ...rest
    }
  );
}
const meta$h = {
  title: "Primitives/ScrollArea",
  component: ScrollArea
};
const jobs = Array.from({ length: 18 }, (_, i) => ({
  id: `job_${(i + 1).toString().padStart(4, "0")}`,
  branch: `feat/step-${i + 1}`,
  elapsed: `${i + 2}m`
}));
const Scrolling = {
  render: () => /* @__PURE__ */ jsx(
    "div",
    {
      style: {
        width: "var(--palette-width)",
        border: "var(--border-width) solid var(--border-default)",
        borderRadius: "var(--radius-lg)",
        background: "var(--bg-overlay)",
        overflow: "hidden"
      },
      children: /* @__PURE__ */ jsx(ScrollArea, { maxHeight: "var(--palette-max-height)", children: /* @__PURE__ */ jsx(Table, { children: /* @__PURE__ */ jsx(TableBody, { children: jobs.map((j) => /* @__PURE__ */ jsxs(TableRow, { children: [
        /* @__PURE__ */ jsx(TableCell, { variant: "mono", copyValue: j.id, children: j.id }),
        /* @__PURE__ */ jsx(TableCell, { variant: "mono", children: j.branch }),
        /* @__PURE__ */ jsx(TableCell, { variant: "metadata", children: j.elapsed })
      ] }, j.id)) }) }) })
    }
  )
};
const WithinBounds = {
  render: () => /* @__PURE__ */ jsx(
    "div",
    {
      style: {
        width: "var(--palette-width)",
        border: "var(--border-width) solid var(--border-default)",
        borderRadius: "var(--radius-lg)",
        background: "var(--bg-overlay)",
        overflow: "hidden"
      },
      children: /* @__PURE__ */ jsx(ScrollArea, { maxHeight: "var(--palette-max-height)", children: /* @__PURE__ */ jsx(Table, { children: /* @__PURE__ */ jsx(TableBody, { children: jobs.slice(0, 3).map((j) => /* @__PURE__ */ jsxs(TableRow, { children: [
        /* @__PURE__ */ jsx(TableCell, { variant: "mono", copyValue: j.id, children: j.id }),
        /* @__PURE__ */ jsx(TableCell, { variant: "mono", children: j.branch }),
        /* @__PURE__ */ jsx(TableCell, { variant: "metadata", children: j.elapsed })
      ] }, j.id)) }) }) })
    }
  )
};
const __vite_glob_0_54 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Scrolling,
  WithinBounds,
  default: meta$h
}, Symbol.toStringTag, { value: "Module" }));
const meta$g = {
  title: "Primitives/Select",
  component: Select
};
function Card$2({ children }) {
  return /* @__PURE__ */ jsx(
    "div",
    {
      style: {
        display: "flex",
        flexDirection: "column",
        gap: "var(--space-4)",
        width: "var(--sidebar-max)",
        padding: "var(--pad-card)",
        borderRadius: "var(--radius-md)",
        background: "var(--bg-raised)"
      },
      children
    }
  );
}
const ceilings = /* @__PURE__ */ jsxs(Fragment, { children: [
  /* @__PURE__ */ jsx("option", { children: "4 drones" }),
  /* @__PURE__ */ jsx("option", { children: "8 drones" })
] });
const Default$2 = {
  render: () => /* @__PURE__ */ jsx(Card$2, { children: /* @__PURE__ */ jsx(Select, { label: "Concurrency ceiling", children: ceilings }) })
};
const Focused$3 = {
  render: () => /* @__PURE__ */ jsx(Card$2, { children: /* @__PURE__ */ jsx(Select, { label: "Concurrency ceiling", "data-preview-focus": "", children: ceilings }) })
};
const Invalid$1 = {
  render: () => /* @__PURE__ */ jsx(Card$2, { children: /* @__PURE__ */ jsx(
    Select,
    {
      label: "Concurrency ceiling",
      invalid: true,
      message: "8 drones exceeds the machine's headroom.",
      defaultValue: "8 drones",
      children: ceilings
    }
  ) })
};
const Disabled$3 = {
  render: () => /* @__PURE__ */ jsx(Card$2, { children: /* @__PURE__ */ jsx(Select, { label: "Concurrency ceiling", disabled: true, children: ceilings }) })
};
const Light$3 = {
  render: () => /* @__PURE__ */ jsx("div", { "data-theme": "light", children: /* @__PURE__ */ jsx(Card$2, { children: /* @__PURE__ */ jsx(Select, { label: "Concurrency ceiling", children: ceilings }) }) })
};
const __vite_glob_0_55 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Default: Default$2,
  Disabled: Disabled$3,
  Focused: Focused$3,
  Invalid: Invalid$1,
  Light: Light$3,
  default: meta$g
}, Symbol.toStringTag, { value: "Module" }));
const meta$f = {
  title: "Primitives/Separator",
  component: Separator,
  decorators: [
    (Story) => /* @__PURE__ */ jsx("div", { style: { maxWidth: "56ch" }, children: /* @__PURE__ */ jsx(Story, {}) })
  ]
};
const Horizontal = {
  render: () => /* @__PURE__ */ jsxs(
    "div",
    {
      style: {
        display: "flex",
        flexDirection: "column",
        padding: "var(--space-1) 0",
        borderRadius: "var(--radius-lg)",
        background: "var(--bg-overlay)",
        fontSize: "var(--text-sm)"
      },
      children: [
        /* @__PURE__ */ jsx("span", { style: { height: "var(--h-menu-item)", padding: "0 var(--space-3)", lineHeight: "var(--h-menu-item)" }, children: "Open the worktree" }),
        /* @__PURE__ */ jsx("span", { style: { height: "var(--h-menu-item)", padding: "0 var(--space-3)", lineHeight: "var(--h-menu-item)" }, children: "Send back to the Job Board" }),
        /* @__PURE__ */ jsx("div", { style: { margin: "var(--space-1) 0" }, children: /* @__PURE__ */ jsx(Separator, {}) }),
        /* @__PURE__ */ jsx(
          "span",
          {
            style: {
              height: "var(--h-menu-item)",
              padding: "0 var(--space-3)",
              lineHeight: "var(--h-menu-item)",
              color: "var(--status-completed-failed)"
            },
            children: "Kill job"
          }
        )
      ]
    }
  )
};
const Vertical = {
  render: () => /* @__PURE__ */ jsxs(
    "div",
    {
      style: {
        display: "flex",
        alignItems: "center",
        gap: "var(--space-3)",
        height: "var(--h-control)",
        color: "var(--fg-muted)",
        fontSize: "var(--text-sm)"
      },
      children: [
        /* @__PURE__ */ jsx("span", { children: "Filter" }),
        /* @__PURE__ */ jsx(Separator, { orientation: "vertical" }),
        /* @__PURE__ */ jsx("span", { children: "Sort" }),
        /* @__PURE__ */ jsx(Separator, { orientation: "vertical" }),
        /* @__PURE__ */ jsx("span", { children: "Group" })
      ]
    }
  )
};
const Announced = {
  args: { decorative: false }
};
const __vite_glob_0_56 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Announced,
  Horizontal,
  Vertical,
  default: meta$f
}, Symbol.toStringTag, { value: "Module" }));
function Sheet({ open: open2, title, children, side = "right", footer, onClose }) {
  const closeRef = useRef(null);
  useEffect(() => {
    if (open2) closeRef.current?.focus();
  }, [open2]);
  useEffect(() => {
    if (!open2) return;
    function onKey(event) {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose?.();
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open2, onClose]);
  if (!open2) return null;
  return /* @__PURE__ */ jsx("div", { className: "armada-sheet-scrim", children: /* @__PURE__ */ jsxs(
    "div",
    {
      className: side === "right" ? "armada-sheet armada-sheet--right" : "armada-sheet armada-sheet--left",
      role: "dialog",
      "aria-modal": "true",
      "aria-label": title,
      children: [
        /* @__PURE__ */ jsxs("div", { className: "armada-sheet__head", children: [
          /* @__PURE__ */ jsx("h2", { className: "armada-sheet__title", children: title }),
          /* @__PURE__ */ jsx(
            "button",
            {
              ref: closeRef,
              type: "button",
              className: "armada-sheet__close",
              "aria-label": "Close",
              onClick: onClose,
              children: /* @__PURE__ */ jsx(X, { size: 16, strokeWidth: 2, "aria-hidden": "true" })
            }
          )
        ] }),
        /* @__PURE__ */ jsx("div", { className: "armada-sheet__body", children }),
        footer ? /* @__PURE__ */ jsx("div", { className: "armada-sheet__foot", children: footer }) : null
      ]
    }
  ) });
}
const meta$e = {
  title: "Primitives/Sheet",
  component: Sheet
};
const Right = {
  args: {
    open: true,
    side: "right",
    title: "Kit allowlist",
    children: "The command tripped the allowlist 5 times and was approved every time. Adding it here stops the prompt."
  }
};
const Left = {
  args: {
    open: true,
    side: "left",
    title: "Kit allowlist",
    children: "The command tripped the allowlist 5 times and was approved every time. Adding it here stops the prompt."
  }
};
const __vite_glob_0_57 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Left,
  Right,
  default: meta$e
}, Symbol.toStringTag, { value: "Module" }));
function Skeleton({ className, width, style, ...rest }) {
  return /* @__PURE__ */ jsx(
    "div",
    {
      className: className ? `armada-skeleton ${className}` : "armada-skeleton",
      "aria-hidden": true,
      style: width ? { ...style, width } : style,
      ...rest
    }
  );
}
const DEFAULT_WIDTHS = ["60%", "85%", "40%"];
function SkeletonText({
  className,
  widths = DEFAULT_WIDTHS,
  label: label2 = "Loading",
  ...rest
}) {
  return /* @__PURE__ */ jsx(
    "div",
    {
      className: className ? `armada-skeleton-text ${className}` : "armada-skeleton-text",
      role: "status",
      "aria-label": label2,
      "aria-busy": true,
      ...rest,
      children: widths.map((w, i) => /* @__PURE__ */ jsx(Skeleton, { width: w }, `${w}-${i}`))
    }
  );
}
const meta$d = {
  title: "Primitives/Skeleton",
  component: Skeleton,
  decorators: [
    (Story) => /* @__PURE__ */ jsx("div", { style: { maxWidth: "56ch" }, children: /* @__PURE__ */ jsx(Story, {}) })
  ]
};
const Single = {
  render: () => /* @__PURE__ */ jsx(Skeleton, { width: "60%" })
};
const Text = {
  render: () => /* @__PURE__ */ jsx(SkeletonText, {})
};
const InACard = {
  render: () => /* @__PURE__ */ jsxs(Card$7, { children: [
    /* @__PURE__ */ jsx(CardTitle, { children: "Evidence" }),
    /* @__PURE__ */ jsx(SkeletonText, { label: "Loading evidence" })
  ] })
};
const __vite_glob_0_58 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  InACard,
  Single,
  Text,
  default: meta$d
}, Symbol.toStringTag, { value: "Module" }));
const meta$c = {
  title: "Primitives/Split button",
  component: SplitButton
};
function Row({ children }) {
  return /* @__PURE__ */ jsx(
    "div",
    {
      style: {
        display: "flex",
        alignItems: "flex-start",
        gap: "var(--space-2)",
        padding: "var(--space-3)",
        borderRadius: "var(--radius-sm)",
        background: "var(--bg-sunken)"
      },
      children
    }
  );
}
const reviewActions = [
  { label: "Reject" },
  { label: "Redispatch with notes" },
  { label: "Open diff", shortcut: "d" }
];
const Closed = {
  render: () => /* @__PURE__ */ jsx(Row, { children: /* @__PURE__ */ jsx(SplitButton, { items: reviewActions, ground: "sunken", children: "Approve" }) })
};
const Open = {
  render: () => /* @__PURE__ */ jsx(Row, { children: /* @__PURE__ */ jsx(SplitButton, { items: reviewActions, ground: "sunken", defaultOpen: true, children: "Approve" }) })
};
const EscalatedRow = {
  render: () => /* @__PURE__ */ jsx(Row, { children: /* @__PURE__ */ jsx(
    SplitButton,
    {
      ground: "sunken",
      defaultOpen: true,
      items: [{ label: "Kill & Redispatch" }, { label: "Kill", danger: true, shortcut: "x" }],
      children: "Pilot"
    }
  ) })
};
const Focused$2 = {
  render: () => /* @__PURE__ */ jsxs("div", { style: { display: "flex", gap: "var(--space-4)" }, children: [
    /* @__PURE__ */ jsx("div", { "data-preview-focus": "action", children: /* @__PURE__ */ jsx(Row, { children: /* @__PURE__ */ jsx(SplitButton, { items: reviewActions, ground: "sunken", children: "Approve" }) }) }),
    /* @__PURE__ */ jsx("div", { "data-preview-focus": "caret", children: /* @__PURE__ */ jsx(Row, { children: /* @__PURE__ */ jsx(SplitButton, { items: reviewActions, ground: "sunken", children: "Approve" }) }) })
  ] })
};
const Disabled$2 = {
  render: () => /* @__PURE__ */ jsx(Row, { children: /* @__PURE__ */ jsx(SplitButton, { items: reviewActions, ground: "sunken", disabled: true, children: "Approve" }) })
};
const PrimaryOnJobDetail = {
  render: () => /* @__PURE__ */ jsx(
    "div",
    {
      style: {
        display: "flex",
        gap: "var(--space-3)",
        padding: "var(--pad-card)",
        borderRadius: "var(--radius-md)",
        background: "var(--bg-raised)"
      },
      children: /* @__PURE__ */ jsx(SplitButton, { items: reviewActions, variant: "primary", children: "Approve" })
    }
  )
};
const Light$2 = {
  render: () => /* @__PURE__ */ jsx("div", { "data-theme": "light", children: /* @__PURE__ */ jsx(Row, { children: /* @__PURE__ */ jsx(SplitButton, { items: reviewActions, ground: "sunken", children: "Approve" }) }) })
};
const FocusedOnPrimary = {
  render: () => /* @__PURE__ */ jsxs("div", { className: "armada-split-button-focus-row", children: [
    /* @__PURE__ */ jsx("div", { "data-preview-focus": "action", children: /* @__PURE__ */ jsx(SplitButton, { items: reviewActions, variant: "primary", children: "Approve" }) }),
    /* @__PURE__ */ jsx("div", { "data-preview-focus": "caret", children: /* @__PURE__ */ jsx(SplitButton, { items: reviewActions, variant: "primary", children: "Approve" }) })
  ] })
};
const __vite_glob_0_59 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Closed,
  Disabled: Disabled$2,
  EscalatedRow,
  Focused: Focused$2,
  FocusedOnPrimary,
  Light: Light$2,
  Open,
  PrimaryOnJobDetail,
  default: meta$c
}, Symbol.toStringTag, { value: "Module" }));
function Switch({ children, description, ...rest }) {
  return /* @__PURE__ */ jsxs("label", { className: "armada-switch", "data-described": description !== void 0 || void 0, children: [
    /* @__PURE__ */ jsxs("span", { className: "armada-switch__text", children: [
      /* @__PURE__ */ jsx("span", { children }),
      description !== void 0 && /* @__PURE__ */ jsx("span", { className: "armada-switch__description", children: description })
    ] }),
    /* @__PURE__ */ jsx("input", { ...rest, type: "checkbox", role: "switch", className: "armada-switch__input" }),
    /* @__PURE__ */ jsx("span", { className: "armada-switch__track", "aria-hidden": "true", children: /* @__PURE__ */ jsx("span", { className: "armada-switch__thumb" }) })
  ] });
}
const meta$b = {
  title: "Primitives/Switch",
  component: Switch
};
function Card$1({ children }) {
  return /* @__PURE__ */ jsx(
    "div",
    {
      style: {
        display: "flex",
        flexDirection: "column",
        gap: "var(--space-2)",
        width: "var(--sidebar-max)",
        padding: "var(--pad-card)",
        borderRadius: "var(--radius-md)",
        background: "var(--bg-raised)"
      },
      children
    }
  );
}
const On = {
  render: () => /* @__PURE__ */ jsx(Card$1, { children: /* @__PURE__ */ jsx(Switch, { defaultChecked: true, children: "Escalate on stall" }) })
};
const Off = {
  render: () => /* @__PURE__ */ jsx(Card$1, { children: /* @__PURE__ */ jsx(Switch, { children: "Auto-approve small diffs" }) })
};
const Focused$1 = {
  render: () => /* @__PURE__ */ jsx(Card$1, { children: /* @__PURE__ */ jsx(Switch, { defaultChecked: true, "data-preview-focus": "", children: "Escalate on stall" }) })
};
const Disabled$1 = {
  render: () => /* @__PURE__ */ jsxs(Card$1, { children: [
    /* @__PURE__ */ jsx(Switch, { defaultChecked: true, disabled: true, children: "Escalate on stall" }),
    /* @__PURE__ */ jsx(Switch, { disabled: true, children: "Auto-approve small diffs" })
  ] })
};
const WithADescription = {
  render: () => /* @__PURE__ */ jsxs(Card$1, { children: [
    /* @__PURE__ */ jsx(
      Switch,
      {
        defaultChecked: true,
        description: "A drone that stops reporting for 12 minutes reaches your phone. Off, it waits on the Alerts queue.",
        children: "Escalate on stall"
      }
    ),
    /* @__PURE__ */ jsx(Switch, { description: "Armada reads the config repo each launch. Local edits made since the last push are kept.", children: "Pull the Kit on startup" })
  ] })
};
const Light$1 = {
  render: () => /* @__PURE__ */ jsx("div", { "data-theme": "light", children: /* @__PURE__ */ jsx(Card$1, { children: /* @__PURE__ */ jsx(Switch, { defaultChecked: true, children: "Escalate on stall" }) }) })
};
const __vite_glob_0_60 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Disabled: Disabled$1,
  Focused: Focused$1,
  Light: Light$1,
  Off,
  On,
  WithADescription,
  default: meta$b
}, Symbol.toStringTag, { value: "Module" }));
const meta$a = {
  title: "Primitives/Table",
  component: Table
};
const conditions = [
  { module: "Fleet daemon", result: "ok", detail: "pid 4417, uptime 6d", checked: "2m ago" },
  { module: "Git", result: "ok", detail: "worktrees clean", checked: "2m ago" },
  { module: "Disk", result: "low", detail: "4% free where Fleet writes", checked: "2m ago" }
];
function Grid({ children }) {
  return /* @__PURE__ */ jsx("div", { style: { maxWidth: "88ch" }, children });
}
function Header() {
  return /* @__PURE__ */ jsx(TableHead, { children: /* @__PURE__ */ jsxs(TableRow, { children: [
    /* @__PURE__ */ jsx(TableHeaderCell, { children: "Module" }),
    /* @__PURE__ */ jsx(TableHeaderCell, { children: "Result" }),
    /* @__PURE__ */ jsx(TableHeaderCell, { children: "Detail" }),
    /* @__PURE__ */ jsx(TableHeaderCell, { children: "Checked" })
  ] }) });
}
const Default$1 = {
  render: () => /* @__PURE__ */ jsx(Grid, { children: /* @__PURE__ */ jsxs(Table, { children: [
    /* @__PURE__ */ jsx(Header, {}),
    /* @__PURE__ */ jsx(TableBody, { children: conditions.map((c) => /* @__PURE__ */ jsxs(TableRow, { children: [
      /* @__PURE__ */ jsx(TableCell, { children: c.module }),
      /* @__PURE__ */ jsx(TableCell, { variant: "mono", children: c.result }),
      /* @__PURE__ */ jsx(TableCell, { variant: "mono", truncates: true, children: c.detail }),
      /* @__PURE__ */ jsx(TableCell, { variant: "metadata", children: c.checked })
    ] }, c.module)) })
  ] }) })
};
const FocusedAndSelected = {
  render: () => /* @__PURE__ */ jsx(Grid, { children: /* @__PURE__ */ jsxs(Table, { children: [
    /* @__PURE__ */ jsx(Header, {}),
    /* @__PURE__ */ jsxs(TableBody, { children: [
      /* @__PURE__ */ jsxs(TableRow, { focused: true, children: [
        /* @__PURE__ */ jsx(TableCell, { children: "Fleet daemon" }),
        /* @__PURE__ */ jsx(TableCell, { variant: "mono", children: "ok" }),
        /* @__PURE__ */ jsx(TableCell, { variant: "secondary", children: "Focused — the keyboard cursor" }),
        /* @__PURE__ */ jsx(TableCell, { variant: "metadata", children: "2m ago" })
      ] }),
      /* @__PURE__ */ jsxs(TableRow, { selected: true, children: [
        /* @__PURE__ */ jsx(TableCell, { children: "Git" }),
        /* @__PURE__ */ jsx(TableCell, { variant: "mono", children: "ok" }),
        /* @__PURE__ */ jsx(TableCell, { variant: "secondary", children: "Selected" }),
        /* @__PURE__ */ jsx(TableCell, { variant: "metadata", children: "2m ago" })
      ] }),
      /* @__PURE__ */ jsxs(TableRow, { focused: true, selected: true, children: [
        /* @__PURE__ */ jsx(TableCell, { children: "Disk" }),
        /* @__PURE__ */ jsx(TableCell, { variant: "mono", children: "low" }),
        /* @__PURE__ */ jsx(TableCell, { variant: "secondary", children: "Both at once" }),
        /* @__PURE__ */ jsx(TableCell, { variant: "metadata", children: "2m ago" })
      ] })
    ] })
  ] }) })
};
const MonoValuesCopy = {
  render: () => /* @__PURE__ */ jsx(Grid, { children: /* @__PURE__ */ jsx(Table, { children: /* @__PURE__ */ jsxs(TableBody, { children: [
    /* @__PURE__ */ jsxs(TableRow, { children: [
      /* @__PURE__ */ jsx(TableCell, { variant: "secondary", children: "Job" }),
      /* @__PURE__ */ jsx(TableCell, { variant: "mono", copyValue: "job_8f2a1c", children: "job_8f2a1c" })
    ] }),
    /* @__PURE__ */ jsxs(TableRow, { children: [
      /* @__PURE__ */ jsx(TableCell, { variant: "secondary", children: "Branch" }),
      /* @__PURE__ */ jsx(TableCell, { variant: "mono", copyValue: "feat/auth-refresh", children: "feat/auth-refresh" })
    ] }),
    /* @__PURE__ */ jsxs(TableRow, { children: [
      /* @__PURE__ */ jsx(TableCell, { variant: "secondary", children: "Path" }),
      /* @__PURE__ */ jsx(TableCell, { variant: "mono", copyValue: "auth/session.rs", children: "auth/session.rs" })
    ] })
  ] }) }) })
};
const Dimmed = {
  render: () => /* @__PURE__ */ jsx(Grid, { children: /* @__PURE__ */ jsxs(Table, { children: [
    /* @__PURE__ */ jsx(Header, {}),
    /* @__PURE__ */ jsxs(TableBody, { children: [
      /* @__PURE__ */ jsxs(TableRow, { children: [
        /* @__PURE__ */ jsx(TableCell, { children: "Fleet daemon" }),
        /* @__PURE__ */ jsx(TableCell, { variant: "mono", children: "ok" }),
        /* @__PURE__ */ jsx(TableCell, { variant: "secondary", children: "Reads at full weight" }),
        /* @__PURE__ */ jsx(TableCell, { variant: "metadata", children: "2m ago" })
      ] }),
      /* @__PURE__ */ jsxs(TableRow, { dimmed: true, children: [
        /* @__PURE__ */ jsx(TableCell, { children: "Disk" }),
        /* @__PURE__ */ jsx(TableCell, { variant: "mono", children: "not checked" }),
        /* @__PURE__ */ jsx(TableCell, { variant: "secondary", children: "Stepped down a token" }),
        /* @__PURE__ */ jsx(TableCell, { variant: "metadata", children: "2m ago" })
      ] })
    ] })
  ] }) })
};
const RowsGrowWithContent = {
  render: () => /* @__PURE__ */ jsx(Grid, { children: /* @__PURE__ */ jsxs(Table, { children: [
    /* @__PURE__ */ jsx(Header, {}),
    /* @__PURE__ */ jsxs(TableBody, { children: [
      /* @__PURE__ */ jsxs(TableRow, { children: [
        /* @__PURE__ */ jsx(TableCell, { children: "Git" }),
        /* @__PURE__ */ jsx(TableCell, { variant: "mono", children: "ok" }),
        /* @__PURE__ */ jsx(TableCell, { variant: "secondary", children: "The branch was deleted outside Armada, so the worktree Fleet cut for this job no longer resolves and two jobs are blocked behind it." }),
        /* @__PURE__ */ jsx(TableCell, { variant: "metadata", children: "2m ago" })
      ] }),
      /* @__PURE__ */ jsxs(TableRow, { children: [
        /* @__PURE__ */ jsx(TableCell, { children: "Disk" }),
        /* @__PURE__ */ jsx(TableCell, { variant: "mono", children: "ok" }),
        /* @__PURE__ */ jsx(TableCell, { variant: "secondary", children: "Short." }),
        /* @__PURE__ */ jsx(TableCell, { variant: "metadata", children: "2m ago" })
      ] })
    ] })
  ] }) })
};
const __vite_glob_0_61 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Default: Default$1,
  Dimmed,
  FocusedAndSelected,
  MonoValuesCopy,
  RowsGrowWithContent,
  default: meta$a
}, Symbol.toStringTag, { value: "Module" }));
const meta$9 = {
  title: "Primitives/Tabs",
  component: Tabs
};
const SectionsOfOneObject = {
  args: {
    defaultValue: "diff",
    items: [
      { id: "diff", label: "Diff" },
      { id: "evidence", label: "Evidence" },
      { id: "log", label: "Log" }
    ]
  }
};
const LastActive = {
  args: {
    defaultValue: "log",
    items: [
      { id: "diff", label: "Diff" },
      { id: "evidence", label: "Evidence" },
      { id: "log", label: "Log" }
    ]
  }
};
const __vite_glob_0_62 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  LastActive,
  SectionsOfOneObject,
  default: meta$9
}, Symbol.toStringTag, { value: "Module" }));
const meta$8 = {
  title: "Primitives/Tabs with counts",
  component: TabsWithCounts
};
const Queues = {
  args: {
    defaultValue: "alerts",
    items: [
      { id: "alerts", label: "Alerts", count: 4 },
      { id: "reviews", label: "Reviews", count: 3 },
      { id: "activity", label: "Activity" }
    ]
  }
};
const Zero = {
  args: {
    defaultValue: "alerts",
    items: [
      { id: "alerts", label: "Alerts", count: 0 },
      { id: "reviews", label: "Reviews", count: 3 },
      { id: "activity", label: "Activity" }
    ]
  }
};
const TheBoard$1 = {
  args: {
    defaultValue: "all",
    items: [
      { id: "all", label: "All", count: 15, shortcut: "1" },
      { id: "needs-you", label: "Needs you", count: 4, shortcut: "2" },
      { id: "running", label: "Running", count: 6, shortcut: "3" },
      { id: "queued", label: "Queued", count: 2, shortcut: "4" },
      { id: "finished", label: "Finished", count: 3, shortcut: "5" }
    ]
  }
};
const Suspended = {
  args: {
    defaultValue: "running",
    suspended: true,
    items: [
      { id: "all", label: "All", count: 3, shortcut: "1" },
      { id: "needs-you", label: "Needs you", count: 1, shortcut: "2" },
      { id: "running", label: "Running", count: 2, shortcut: "3" },
      { id: "queued", label: "Queued", shortcut: "4" },
      { id: "finished", label: "Finished", shortcut: "5" }
    ]
  }
};
const __vite_glob_0_63 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Queues,
  Suspended,
  TheBoard: TheBoard$1,
  Zero,
  default: meta$8
}, Symbol.toStringTag, { value: "Module" }));
const meta$7 = {
  title: "Primitives/Textarea",
  component: Textarea
};
function Card({ children }) {
  return /* @__PURE__ */ jsx(
    "div",
    {
      style: {
        display: "flex",
        flexDirection: "column",
        gap: "var(--space-4)",
        width: "var(--sidebar-max)",
        padding: "var(--pad-card)",
        borderRadius: "var(--radius-md)",
        background: "var(--bg-raised)"
      },
      children
    }
  );
}
const BRIEF$1 = "A burst of 401s should produce one refresh call, not one per request. Keep the retry ceiling where it is.";
const Default = {
  args: { label: "Brief", defaultValue: BRIEF$1 },
  render: (args) => /* @__PURE__ */ jsx(Card, { children: /* @__PURE__ */ jsx(Textarea, { ...args }) })
};
const Placeholder = {
  args: { label: "Brief", placeholder: BRIEF$1 },
  render: (args) => /* @__PURE__ */ jsx(Card, { children: /* @__PURE__ */ jsx(Textarea, { ...args }) })
};
const Focused = {
  render: () => /* @__PURE__ */ jsx(Card, { children: /* @__PURE__ */ jsx(Textarea, { label: "Brief", defaultValue: BRIEF$1, "data-preview-focus": "" }) })
};
const Invalid = {
  render: () => /* @__PURE__ */ jsx(Card, { children: /* @__PURE__ */ jsx(Textarea, { label: "Brief", invalid: true, message: "A job needs a brief. Write what the work is." }) })
};
const Disabled = {
  render: () => /* @__PURE__ */ jsx(Card, { children: /* @__PURE__ */ jsx(Textarea, { label: "Brief", defaultValue: BRIEF$1, disabled: true }) })
};
const Rows = {
  render: () => /* @__PURE__ */ jsx(Card, { children: /* @__PURE__ */ jsx(Textarea, { label: "Brief", rows: 6, defaultValue: BRIEF$1 }) })
};
const Overflowing = {
  render: () => /* @__PURE__ */ jsx(Card, { children: /* @__PURE__ */ jsx(
    Textarea,
    {
      label: "Brief",
      defaultValue: `${BRIEF$1} The retry ceiling is three, set where the transport is configured rather than at the call site, and moving it is a separate job. What is in scope is the coalescing: one refresh in flight, every waiter parked on it.`
    }
  ) })
};
const Light = {
  render: () => /* @__PURE__ */ jsx("div", { "data-theme": "light", children: /* @__PURE__ */ jsx(Card, { children: /* @__PURE__ */ jsx(Textarea, { label: "Brief", defaultValue: BRIEF$1 }) }) })
};
const __vite_glob_0_64 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Default,
  Disabled,
  Focused,
  Invalid,
  Light,
  Overflowing,
  Placeholder,
  Rows,
  default: meta$7
}, Symbol.toStringTag, { value: "Module" }));
function Toast({ status, children, actionLabel, onAction }) {
  return /* @__PURE__ */ jsxs("div", { className: "armada-toast", role: "status", children: [
    status ? /* @__PURE__ */ jsx(
      "span",
      {
        className: "armada-toast__dot",
        style: { background: `var(--status-${status})` },
        "aria-hidden": "true"
      }
    ) : null,
    /* @__PURE__ */ jsx("span", { className: "armada-toast__text", children }),
    actionLabel ? /* @__PURE__ */ jsx("button", { type: "button", className: "armada-toast__action", onClick: onAction, children: actionLabel }) : null
  ] });
}
const meta$6 = {
  title: "Primitives/Toast",
  component: Toast
};
const Copied = {
  args: {
    children: "Copied job_8f2a1c."
  }
};
const Killed$1 = {
  args: {
    status: "killed",
    children: "Killed job_8f2a1c. The worktree is left in place."
  }
};
const Landed = {
  args: {
    status: "completed-success",
    children: "Convoy landed as one PR.",
    actionLabel: "View"
  }
};
const __vite_glob_0_65 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Copied,
  Killed: Killed$1,
  Landed,
  default: meta$6
}, Symbol.toStringTag, { value: "Module" }));
const meta$5 = {
  title: "Primitives/Tooltip",
  component: Tooltip
};
const TruncatedValue = {
  args: {
    defaultOpen: true,
    label: "crates/api/src/session/refresh_coalescing.rs",
    children: /* @__PURE__ */ jsx("span", { className: "armada-tooltip__truncated", children: "crates/api/src/session/refresh_coalescing.rs" })
  }
};
const WithShortcut = {
  args: {
    defaultOpen: true,
    label: "Approve dispatch",
    shortcut: "a",
    children: /* @__PURE__ */ jsx("button", { type: "button", className: "armada-tooltip__action", children: "Approve" })
  }
};
const Resting = {
  args: {
    label: "crates/api/src/session/refresh_coalescing.rs",
    children: /* @__PURE__ */ jsx("span", { className: "armada-tooltip__truncated", children: "crates/api/src/session/refresh_coalescing.rs" })
  }
};
const longPath = "crates/api/src/session/refresh_coalescing.rs";
function Frame({ edge, children }) {
  return /* @__PURE__ */ jsx("div", { className: `armada-tooltip-frame armada-tooltip-frame--${edge}`, children });
}
const AtTheLeftEdge = {
  render: () => /* @__PURE__ */ jsx(Frame, { edge: "left", children: /* @__PURE__ */ jsx(Tooltip, { defaultOpen: true, label: longPath, children: /* @__PURE__ */ jsx("span", { className: "armada-tooltip__truncated", children: longPath }) }) })
};
const AtTheRightEdge = {
  render: () => /* @__PURE__ */ jsx(Frame, { edge: "right", children: /* @__PURE__ */ jsx(Tooltip, { defaultOpen: true, label: longPath, children: /* @__PURE__ */ jsx("span", { className: "armada-tooltip__truncated", children: longPath }) }) })
};
const WithNoRoomBelow = {
  render: () => /* @__PURE__ */ jsx(Frame, { edge: "bottom", children: /* @__PURE__ */ jsx(Tooltip, { defaultOpen: true, label: longPath, children: /* @__PURE__ */ jsx("span", { className: "armada-tooltip__truncated", children: longPath }) }) })
};
const __vite_glob_0_66 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AtTheLeftEdge,
  AtTheRightEdge,
  Resting,
  TruncatedValue,
  WithNoRoomBelow,
  WithShortcut,
  default: meta$5
}, Symbol.toStringTag, { value: "Module" }));
function DispatchAJobFullWithTheM1SubsetMarked({
  composer
}) {
  return /* @__PURE__ */ jsx("div", { className: "armada-screen__row", children: /* @__PURE__ */ jsx("div", { className: "armada-screen__col", "data-width": "card", children: /* @__PURE__ */ jsx(JobComposer, { ...composer }) }) });
}
const meta$4 = {
  title: "Screens/Dispatch a job — full, with the M1 subset marked",
  component: DispatchAJobFullWithTheM1SubsetMarked
};
const Dispatch = {
  render: () => /* @__PURE__ */ jsx("div", { className: "armada-screen", children: /* @__PURE__ */ jsx(
    DispatchAJobFullWithTheM1SubsetMarked,
    {
      composer: {
        title: "Coalesce concurrent token refreshes",
        brief: "A burst of 401s should produce one refresh call, not one per request. Keep the retry ceiling where it is.",
        workflows: /* @__PURE__ */ jsx("option", { children: "bug — 4 steps" }),
        project: "armada",
        glance: [
          { label: "Steps", value: "4 · 2 gated" },
          { label: "Checks", value: "build, test" }
        ],
        provenance: "Dispatched by you"
      }
    }
  ) })
};
const __vite_glob_0_67 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Dispatch,
  default: meta$4
}, Symbol.toStringTag, { value: "Module" }));
function FirstLaunch$1({ running: running2, notRunning, onCopied }) {
  return /* @__PURE__ */ jsxs("div", { className: "armada-screen__row", children: [
    /* @__PURE__ */ jsx(Reading, { ...running2, onCopied: running2.onCopied ?? onCopied }),
    /* @__PURE__ */ jsx(Reading, { ...notRunning, onCopied: notRunning.onCopied ?? onCopied })
  ] });
}
function Reading({ caption, ...state }) {
  return /* @__PURE__ */ jsxs("div", { className: "armada-screen__card", "data-width": "half", children: [
    /* @__PURE__ */ jsx("span", { className: "armada-screen__caption", children: caption }),
    /* @__PURE__ */ jsx(BoardEmptyState, { ...state })
  ] });
}
const meta$3 = {
  title: "Screens/First launch",
  component: FirstLaunch$1
};
const FirstLaunch = {
  render: () => /* @__PURE__ */ jsx("div", { className: "armada-screen", children: /* @__PURE__ */ jsx(
    FirstLaunch$1,
    {
      running: {
        caption: "Fleet running, no jobs",
        quiet: true,
        action: /* @__PURE__ */ jsx(Button, { variant: "primary", children: "New job" }),
        children: "No jobs. Fleet has been up 6 days."
      },
      notRunning: {
        caption: "Fleet is not running",
        command: "armada-fleet start",
        note: "Run that in a terminal. Bridge connects on its own once the runtime file appears.",
        children: "Fleet is not running. Bridge has nothing to read."
      }
    }
  ) })
};
const __vite_glob_0_68 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  FirstLaunch,
  default: meta$3
}, Symbol.toStringTag, { value: "Module" }));
function Absent({ name, note }) {
  return /* @__PURE__ */ jsxs("div", { className: "armada-screen-absent", role: "note", children: [
    /* @__PURE__ */ jsx("span", { className: "armada-screen-absent__name", children: name }),
    /* @__PURE__ */ jsx("span", { className: "armada-screen-absent__why", children: note })
  ] });
}
function InsideAJob({
  heading: heading2,
  run,
  runLabel = "The run",
  runElapsed,
  runAbsent = "Nothing serves this Job's workflow, so its steps are unknown.",
  pulsing = true,
  onSelectStep,
  openSteps,
  onOpenStep,
  where,
  whereLabel = "Where things are",
  whereNote,
  whereAbsent = "Nothing serves this Job's paths or its branch.",
  record,
  recordLabel = "What it left behind",
  brief,
  briefAbsent = "Nothing serves this Job's brief or its acceptance criteria.",
  step,
  stepAbsent = "No step is open. Select one in the run.",
  onCopied
}) {
  return /* @__PURE__ */ jsxs("div", { className: "armada-screen__detail", children: [
    /* @__PURE__ */ jsx(JobDetailHeaderActions, { ...heading2, onCopied }),
    /* @__PURE__ */ jsxs("div", { className: "armada-inside", children: [
      /* @__PURE__ */ jsxs("div", { className: "armada-inside__run", children: [
        /* @__PURE__ */ jsxs("div", { className: "armada-inside__region-head", children: [
          /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", children: runLabel }),
          runElapsed === void 0 ? null : /* @__PURE__ */ jsx("span", { className: "armada-inside__elapsed", children: runElapsed })
        ] }),
        run.length === 0 ? /* @__PURE__ */ jsx("div", { className: "armada-screen__slot", children: /* @__PURE__ */ jsx(Absent, { name: "The run", note: runAbsent }) }) : /* @__PURE__ */ jsx(
          RunTree,
          {
            steps: run,
            pulsing,
            onSelect: onSelectStep,
            openSteps,
            onOpen: onOpenStep,
            onCopied
          }
        ),
        /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", "data-spaced": true, children: whereLabel }),
        where === void 0 || where.length === 0 ? /* @__PURE__ */ jsx("div", { className: "armada-screen__slot", children: /* @__PURE__ */ jsx(Absent, { name: "Where things are", note: whereAbsent }) }) : /* @__PURE__ */ jsx(WhereRegion, { rows: where, note: whereNote, onCopied }),
        record === void 0 ? null : /* @__PURE__ */ jsxs(Fragment, { children: [
          /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", "data-spaced": true, children: recordLabel }),
          record
        ] })
      ] }),
      /* @__PURE__ */ jsxs("div", { className: "armada-inside__panel", children: [
        /* @__PURE__ */ jsxs("div", { className: "armada-inside__brief", children: [
          /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", children: "Brief" }),
          brief === void 0 ? /* @__PURE__ */ jsx("div", { className: "armada-screen__slot", children: /* @__PURE__ */ jsx(Absent, { name: "Brief", note: briefAbsent }) }) : /* @__PURE__ */ jsx(JobBrief, { ...brief })
        ] }),
        step === void 0 ? /* @__PURE__ */ jsx("div", { className: "armada-screen__slot", children: /* @__PURE__ */ jsx(Absent, { name: "The step", note: stepAbsent }) }) : /* @__PURE__ */ jsxs(Fragment, { children: [
          /* @__PURE__ */ jsxs("div", { className: "armada-inside__step-head", children: [
            /* @__PURE__ */ jsxs("div", { className: "armada-inside__step-titles", children: [
              /* @__PURE__ */ jsx(
                "span",
                {
                  className: "armada-inside__step-name",
                  "data-identifier": step.labelIsAnIdentifier || void 0,
                  children: step.label
                }
              ),
              /* @__PURE__ */ jsx("div", { className: "armada-inside__step-fields", children: step.fields.map((field, f) => /* @__PURE__ */ jsxs("span", { className: "armada-inside__field", children: [
                field.label === void 0 ? null : /* @__PURE__ */ jsx("span", { className: "armada-inside__field-label", children: field.label }),
                field.value === void 0 ? null : /* @__PURE__ */ jsx(
                  "span",
                  {
                    className: "armada-inside__field-value",
                    "data-mono": field.mono || void 0,
                    children: field.value
                  }
                )
              ] }, f)) })
            ] }),
            step.acts === void 0 ? null : /* @__PURE__ */ jsx("div", { className: "armada-inside__step-acts", children: step.acts })
          ] }),
          step.notice === void 0 ? null : /* @__PURE__ */ jsxs("div", { className: "armada-inside__notice", "data-tone": step.notice.tone, role: "status", children: [
            step.notice.title === void 0 ? null : /* @__PURE__ */ jsx("span", { className: "armada-inside__notice-title", children: step.notice.title }),
            /* @__PURE__ */ jsx("span", { className: "armada-inside__notice-body", children: step.notice.children })
          ] }),
          step.phases === void 0 ? /* @__PURE__ */ jsx("p", { className: "armada-inside__absent", role: "note", children: step.phasesAbsent ?? "Nothing serves this step's gates, so where it stands is unknown." }) : /* @__PURE__ */ jsx(PhaseStrip, { ...step.phases }),
          step.before === void 0 ? null : /* @__PURE__ */ jsx("div", { className: "armada-inside__before", children: step.before }),
          /* @__PURE__ */ jsx(
            StepStory,
            {
              chapters: step.chapters,
              openId: step.openChapter,
              openChapter: step.openChapterId,
              onOpen: step.onOpenChapter
            }
          ),
          step.after === void 0 ? null : /* @__PURE__ */ jsx("div", { className: "armada-inside__after", children: step.after })
        ] })
      ] })
    ] })
  ] });
}
function WhereRegion({
  rows,
  note,
  onCopied
}) {
  const [unopened, setUnopened] = useState(null);
  const [opening, setOpening] = useState(null);
  const open2 = useCallback((at, go) => {
    setOpening(at);
    setUnopened(null);
    void go().then((why) => {
      if (why !== null) setUnopened({ row: at, because: why.because });
    }).finally(() => setOpening(null));
  }, []);
  return /* @__PURE__ */ jsxs("div", { className: "armada-inside__where", children: [
    rows.map((row, at) => {
      const opens = row.open;
      const failed2 = unopened !== null && unopened.row === at ? unopened.because : null;
      return /* @__PURE__ */ jsxs(Fragment$1, { children: [
        row.separated ? /* @__PURE__ */ jsx("span", { className: "armada-inside__where-rule", "aria-hidden": true }) : null,
        /* @__PURE__ */ jsx(
          WhereRow,
          {
            label: row.iconLabel,
            value: row.value,
            note: row.meta,
            act: opens === void 0 ? "copy" : "open",
            copyValue: row.copyValue,
            onCopied,
            actLabel: opens?.label,
            onAct: opens === void 0 || opening !== null ? void 0 : () => open2(at, opens.go)
          }
        ),
        failed2 === null ? null : /* @__PURE__ */ jsx("p", { className: "armada-inside__where-unopened", role: "status", children: failed2 })
      ] }, at);
    }),
    note === void 0 ? null : /* @__PURE__ */ jsx("p", { className: "armada-inside__where-note", children: note })
  ] });
}
const JOB = "job_2d90bb";
const WORKTREE = `.armada/worktrees/${JOB}`;
const DRONE = "01M10B1V2A0011VRS6RA2SKPQ7";
const WHERE = [
  { iconLabel: "Worktree", value: WORKTREE, copyValue: WORKTREE },
  {
    iconLabel: "Branch",
    value: "fix/settings-split-selectors",
    copyValue: "fix/settings-split-selectors"
  },
  { iconLabel: "Manifest", value: "armada.yml", copyValue: "armada.yml" },
  {
    iconLabel: "Workflow",
    value: "bug",
    copyValue: "bug",
    meta: "as it was at 14:20"
  },
  {
    iconLabel: "Job log",
    value: `.armada/logs/${JOB}.jsonl`,
    copyValue: `.armada/logs/${JOB}.jsonl`,
    separated: true
  },
  {
    iconLabel: "Transcript",
    value: ".armada/transcripts/01M10B1V2A.jsonl",
    copyValue: ".armada/transcripts/01M10B1V2A.jsonl"
  },
  { iconLabel: "Drone", value: DRONE, copyValue: DRONE }
];
const BRIEF = {
  facts: "The selectors cannot be tested without constructing the whole store, which makes every settings test an integration test.",
  criteria: [],
  only: "facts",
  factsLabel: null
};
const HEADING = {
  status: "running",
  statusIcon: CircleDot,
  statusLabel: "Running",
  headline: "Split the settings reducer so the selectors can be tested alone",
  jobId: JOB,
  fields: [
    { label: "Workflow", value: "Bug" },
    {
      label: "Branch",
      value: "fix/settings-split-selectors",
      mono: true,
      copyValue: "fix/settings-split-selectors"
    },
    { label: "Elapsed", value: "11m 03s", mono: true },
    { label: "Spend, estimated", value: "~$1.80", mono: true },
    { label: "Dispatched by you" }
  ]
};
const ESCALATED_HEADING = {
  ...HEADING,
  status: "escalated",
  statusIcon: Eye,
  statusLabel: "Needs you"
};
const FAILED_HEADING = {
  ...HEADING,
  status: "completed_failed",
  statusIcon: X,
  statusLabel: "Failed"
};
const BEHIND = [
  {
    id: "repro",
    label: "Reproduction",
    activity: "advanced",
    status: "advanced",
    elapsed: "1m 12s",
    facts: [
      {
        label: "Produced",
        paths: [{ directory: "packages/settings/test/", basename: "useColumnSelectors.test.ts" }]
      },
      { label: "Cleared", value: "test", named: "passed" }
    ]
  },
  {
    id: "root_cause",
    label: "Root cause",
    activity: "advanced",
    status: "advanced",
    elapsed: "3m 40s",
    facts: [
      { label: "Attempt 1", value: "refused", named: "refused" },
      { label: "Attempt 2", value: "advanced", named: "advanced" },
      {
        label: "Produced",
        paths: [{ directory: `.armada/artifacts/${JOB}/`, basename: "root_cause.md" }]
      }
    ]
  }
];
const AHEAD = [
  {
    id: "consumers",
    label: "Check the consumers still compile",
    activity: "not_started",
    facts: [],
    factsAbsent: "This step has not run, so it has produced nothing."
  },
  {
    id: "land",
    label: "Land",
    activity: "not_started",
    locked: true,
    facts: [],
    factsAbsent: "This step has not run, so it has produced nothing."
  }
];
const RUN_RUNNING = [
  ...BEHIND,
  {
    id: "fix",
    label: "Fix",
    activity: "running",
    status: "running",
    elapsed: "6m 11s",
    current: true,
    factsOpen: true,
    facts: [
      { label: "Produced", value: "3 files · +94 −31" },
      { label: "Checks", value: "not run" },
      { label: "Judge", value: "2 criteria" }
    ]
  },
  {
    id: "regression_verify",
    label: "Regression check",
    activity: "not_started",
    facts: [],
    factsAbsent: "This step has not run, so it has produced nothing."
  },
  ...AHEAD
];
const RUN_WAITING = [
  ...BEHIND,
  { id: "fix", label: "Fix", activity: "advanced", status: "advanced", elapsed: "6m 11s", facts: [] },
  {
    id: "regression_verify",
    label: "Regression check",
    activity: "awaiting_human",
    status: "waiting on you",
    elapsed: "2m 04s",
    current: true,
    factsOpen: true,
    facts: [
      { label: "Checks", value: "2 of 2 passed", named: "passed" },
      { label: "Judge", value: "2 of 2 met", named: "passed" },
      { label: "Waiting", value: "on you · 2m 04s" }
    ]
  },
  ...AHEAD
];
const RUN_REPAIRING = [
  ...BEHIND,
  { id: "fix", label: "Fix", activity: "advanced", status: "advanced", elapsed: "6m 11s", facts: [] },
  {
    id: "regression_verify",
    label: "Regression check",
    activity: "retrying",
    status: "retrying",
    elapsed: "1m 09s",
    current: true,
    factsOpen: true,
    facts: [
      { label: "Attempt 1", value: "test failed · exit 101", named: "failed" },
      { label: "Attempt 2", value: "running" },
      { label: "Checks", value: "1 of 2 failed", named: "failed" }
    ]
  },
  ...AHEAD
];
const RUN_STOPPED = [
  ...BEHIND,
  { id: "fix", label: "Fix", activity: "advanced", status: "advanced", elapsed: "6m 11s", facts: [] },
  {
    id: "regression_verify",
    label: "Regression check",
    activity: "stopped",
    status: "retries spent",
    elapsed: "6m 40s",
    current: true,
    factsOpen: true,
    facts: [
      { label: "Attempt 1", value: "same failure", named: "failed" },
      { label: "Attempt 2", value: "same failure", named: "failed" },
      { label: "Attempt 3", value: "same failure", named: "failed" },
      { label: "Held", value: "retries spent · waiting on you" }
    ]
  },
  ...AHEAD
];
const RUN_FAILED = [
  ...BEHIND,
  { id: "fix", label: "Fix", activity: "advanced", status: "advanced", elapsed: "6m 11s", facts: [] },
  {
    id: "regression_verify",
    label: "Regression check",
    activity: "failed",
    status: "failed",
    elapsed: "2m 51s",
    current: true,
    factsOpen: true,
    facts: [
      { label: "Checks", value: "test failed · exit 101", named: "failed" },
      { label: "Judge", value: "not reached" },
      { label: "Job", value: "completed_failed", named: "failed" }
    ]
  }
];
const PREVIEW = [
  { id: "1", at: "14:22:07", actor: "armada", summary: "Go on to Implement." },
  {
    id: "2",
    at: "14:26:31",
    actor: "drone",
    summary: "Edit",
    subject: "packages/settings/src/selectors.ts"
  },
  {
    id: "3",
    at: "14:29:40",
    actor: "drone",
    summary: "Bash",
    subject: "cargo build --workspace --locked",
    output: [
      "$ cargo build --workspace --locked",
      "   Compiling armada-settings v0.1.0 (packages/settings)",
      "   Compiling armada-fleet v0.1.0 (crates/fleet)",
      "    Finished `dev` profile [unoptimized] in 47.61s"
    ].join("\n"),
    ran: `exit 0 · 47.61s · in ${WORKTREE}`
  },
  {
    id: "4",
    at: "14:30:28",
    actor: "fleet",
    summary: "Heartbeat — the Drone has been quiet for 48 seconds"
  },
  { id: "5", at: "14:31:58", actor: "drone", summary: "thinking" }
];
const WHOLE = [
  PREVIEW[0],
  {
    id: "1b",
    at: "14:22:44",
    actor: "drone",
    summary: "Splitting the selector block into its own module so the tests can import it without the store."
  },
  { id: "1c", at: "14:23:11", actor: "drone", summary: "Read", subject: "packages/settings/src/reducer.ts" },
  ...PREVIEW.slice(1)
];
const PRODUCED_FILES = [
  { path: "packages/settings/src/selectors.ts", change: "modified", added: 61, deleted: 4 },
  { path: "packages/settings/src/reducer.ts", change: "modified", added: 12, deleted: 27 },
  { path: "packages/settings/src/index.ts", change: "added", added: 21 }
];
const PRODUCED = /* @__PURE__ */ jsx(ChangedFiles, { emptyNote: "This drone has not changed anything yet.", files: PRODUCED_FILES });
const CHAPTERS = [
  {
    id: "instructions",
    ordinal: 1,
    title: "Drone instructions",
    summary: "14:22:07 · 2 criteria and what it was given",
    preview: "Move the selector block into its own module so the tests can import it without constructing the store. Do not change reducer behaviour."
  },
  {
    id: "log",
    ordinal: 2,
    title: "Activity log",
    // `live` is the running dot, not the word. A count says how many entries
    // there are and only the dot says they are still arriving.
    live: true,
    summary: "47 entries · every line opens",
    preview: /* @__PURE__ */ jsx(ActivityLog, { entries: PREVIEW }),
    content: /* @__PURE__ */ jsx(ActivityLog, { entries: WHOLE }),
    openLabel: "Open the log — all 47 entries"
  },
  {
    id: "produced",
    ordinal: 3,
    title: "Produced",
    // Built from the list rather than typed beside it, so the header and the
    // rows cannot disagree about what the reading found.
    summary: changedFilesSummary(PRODUCED_FILES, true),
    preview: PRODUCED,
    content: PRODUCED,
    openLabel: "Open the diff — 3 files"
  }
];
const REPAIR_CHAPTERS = [
  {
    id: "instructions",
    ordinal: 1,
    title: "Drone instructions",
    summary: "14:44:20",
    preview: "Run the regression suite and fix anything it turns up."
  },
  {
    id: "log",
    ordinal: 2,
    title: "Activity log",
    summary: "88 entries · ended 14:47:11",
    openLabel: "Open the log — all 88 entries",
    preview: /* @__PURE__ */ jsx(
      ActivityLog,
      {
        entries: [
          {
            id: "r1",
            at: "14:46:02",
            actor: "drone",
            summary: "Bash",
            subject: "cargo nextest run --workspace"
          },
          {
            id: "r2",
            at: "14:47:09",
            actor: "fleet",
            summary: "Check failed — 3 of 2034 tests. Handed back to the Drone, attempt 2 of 3.",
            subject: "test",
            named: "failed",
            output: [
              "FAIL settings::selectors::visible_manifests_memoises",
              "  expected the same reference on repeat calls, got a new object",
              "and 2 more"
            ].join("\n"),
            ran: `exit 101 · 1m 22s · in ${WORKTREE}`
          }
        ],
        openId: "r2"
      }
    ),
    content: /* @__PURE__ */ jsx(ActivityLog, { entries: WHOLE })
  },
  {
    id: "produced",
    ordinal: 3,
    title: "Produced",
    summary: "4 files · being repaired",
    preview: "The work is on fix/settings-split-selectors and the Drone is editing it now. Nothing was thrown away and nothing was rolled back.",
    content: PRODUCED,
    openLabel: "Open the diff — 4 files"
  }
];
const meta$2 = {
  title: "Screens/Inside a job — one arrangement at every state",
  component: InsideAJob
};
const JOB_ACTS = /* @__PURE__ */ jsx(Fragment, { children: /* @__PURE__ */ jsx(Button, { variant: "ghost", children: "Kill" }) });
const Running$1 = {
  render: () => /* @__PURE__ */ jsx("div", { className: "armada-screen", children: /* @__PURE__ */ jsx(
    InsideAJob,
    {
      heading: { ...HEADING, actions: JOB_ACTS },
      run: RUN_RUNNING,
      runElapsed: "11m 03s",
      where: WHERE,
      whereNote: "A path opens where it lives; an identifier copies. This milestone is about never needing these — they are here for when you want them anyway.",
      brief: BRIEF,
      step: {
        label: "Fix",
        fields: [
          { label: "Running for", value: "6m 11s", mono: true },
          { label: "Attempt", value: "1", mono: true }
        ],
        acts: /* @__PURE__ */ jsxs(Fragment, { children: [
          /* @__PURE__ */ jsx(Button, { variant: "secondary", children: "Restart step" }),
          /* @__PURE__ */ jsx(Button, { variant: "primary", children: "Redirect" })
        ] }),
        phases: {
          note: "The Drone is working. Nothing has been submitted, so no gate has been asked anything yet.",
          stages: [
            { id: "instructed", label: "Instructed", state: "cleared" },
            { id: "working", label: "Working", state: "current" },
            { id: "submitted", label: "Submitted", state: "ahead" },
            {
              id: "checks",
              label: "build, test",
              kind: "checks",
              state: "ahead",
              stands: "not run",
              rows: [
                { label: "cargo build --workspace --locked", mono: true, result: "not run" },
                { label: "cargo nextest run --workspace", mono: true, result: "not run" }
              ]
            },
            {
              id: "judge",
              label: "Judge · 2 criteria",
              kind: "judge",
              state: "ahead",
              stands: "not reached",
              rows: [
                { label: "Selectors import without the store", result: "not reached" },
                { label: "No behaviour change in the reducer", result: "not reached" }
              ]
            },
            { id: "you", label: "You", kind: "human", state: "ahead" }
          ]
        },
        chapters: CHAPTERS
      }
    }
  ) })
};
const WaitingOnYou = {
  render: () => /* @__PURE__ */ jsx("div", { className: "armada-screen", children: /* @__PURE__ */ jsx(
    InsideAJob,
    {
      heading: {
        ...HEADING,
        status: "awaiting_review",
        statusLabel: "Waiting on you",
        actions: JOB_ACTS
      },
      run: RUN_WAITING,
      runElapsed: "13m 47s",
      where: WHERE,
      brief: BRIEF,
      step: {
        label: "Regression check",
        fields: [
          { label: "Waiting", value: "2m 04s", mono: true },
          { label: "Took", value: "4m 18s", mono: true },
          { label: "Attempt", value: "1", mono: true }
        ],
        acts: /* @__PURE__ */ jsx(Button, { variant: "secondary", children: "Restart step" }),
        notice: {
          tone: "waiting",
          title: "Nothing is wrong. The workflow asks for a person here.",
          children: "The suite passed and the Judge met both criteria. Nothing advances until you answer."
        },
        phases: {
          note: "The suite passed and the Judge met both criteria. Nothing is wrong; the workflow asks for a person here.",
          stages: [
            { id: "instructed", label: "Instructed", state: "cleared" },
            { id: "working", label: "Working", state: "cleared" },
            { id: "submitted", label: "Submitted", state: "cleared" },
            {
              id: "checks",
              label: "build, test",
              kind: "checks",
              state: "cleared",
              stands: "2 of 2 passed",
              rows: [
                { label: "cargo build --workspace --locked", mono: true, result: "exit 0 · 47s", named: "passed" },
                { label: "cargo nextest run --workspace", mono: true, result: "exit 0 · 1m 22s", named: "passed" }
              ]
            },
            {
              id: "judge",
              label: "Judge · 2 of 2 met",
              kind: "judge",
              state: "cleared",
              stands: "2 of 2 met",
              rows: [
                { label: "Selectors import without the store", result: "met", named: "met" },
                { label: "No behaviour change in the reducer", result: "met", named: "met" }
              ]
            },
            { id: "you", label: "You", kind: "human", state: "waiting", stands: "waiting · 2m 04s" }
          ]
        },
        chapters: [
          ...CHAPTERS,
          {
            id: "decision",
            ordinal: 4,
            title: "Your decision",
            summary: "nothing advances until you answer",
            preview: "Approve, or send it back with a note. Send back returns it to this step; reject ends the Job. A note is optional on approve."
          }
        ],
        after: /* @__PURE__ */ jsxs("div", { className: "armada-screen__actions", children: [
          /* @__PURE__ */ jsx(Button, { variant: "primary", children: "Approve" }),
          /* @__PURE__ */ jsx(Button, { variant: "secondary", children: "Send back" }),
          /* @__PURE__ */ jsx(Button, { variant: "destructive", children: "Reject" })
        ] })
      }
    }
  ) })
};
const ACheckFailed = {
  render: () => /* @__PURE__ */ jsx("div", { className: "armada-screen", children: /* @__PURE__ */ jsx(
    InsideAJob,
    {
      heading: { ...HEADING, actions: JOB_ACTS },
      run: RUN_REPAIRING,
      runElapsed: "15m 20s",
      where: WHERE,
      brief: BRIEF,
      step: {
        label: "Regression check",
        fields: [
          { label: "Running for", value: "1m 09s", mono: true },
          { label: "Attempt", value: "2 of 3", mono: true },
          { label: "First failed", value: "14:47:11", mono: true }
        ],
        acts: /* @__PURE__ */ jsxs(Fragment, { children: [
          /* @__PURE__ */ jsx(Button, { variant: "secondary", children: "Restart step" }),
          /* @__PURE__ */ jsx(Button, { variant: "primary", children: "Redirect" })
        ] }),
        notice: {
          tone: "failed",
          title: "The suite failed, and the Drone has been given the output to fix.",
          children: "cargo nextest run --workspace exited 101 with 3 failures. Attempt 2 of 3 is running. Nothing needs you unless it runs out of attempts."
        },
        phases: {
          note: "The Check went back to the Drone with its output. The tiers behind it are still ahead, not cancelled.",
          stages: [
            { id: "instructed", label: "Instructed", state: "cleared" },
            { id: "working", label: "Working", state: "current" },
            { id: "submitted", label: "Submitted", state: "cleared" },
            {
              id: "checks",
              label: "test failed · fixing",
              kind: "checks",
              state: "failed",
              stands: "exit 101 · attempt 2 of 3",
              rows: [
                { label: "cargo build --workspace --locked", mono: true, result: "exit 0 · 47s", named: "passed" },
                { label: "cargo nextest run --workspace", mono: true, result: "exit 101 · 3 failures", named: "failed" }
              ]
            },
            { id: "judge", label: "Judge · 2 criteria", kind: "judge", state: "ahead", stands: "not reached" },
            { id: "you", label: "You", kind: "human", state: "ahead" }
          ]
        },
        chapters: REPAIR_CHAPTERS
      }
    }
  ) })
};
const OutOfAttempts = {
  render: () => /* @__PURE__ */ jsx("div", { className: "armada-screen", children: /* @__PURE__ */ jsx(
    InsideAJob,
    {
      heading: { ...ESCALATED_HEADING, actions: JOB_ACTS },
      run: RUN_STOPPED,
      runElapsed: "21m 55s",
      where: WHERE,
      brief: BRIEF,
      step: {
        label: "Regression check",
        fields: [
          { label: "Held for", value: "6m 40s", mono: true },
          { label: "Attempts", value: "3 of 3", mono: true },
          { label: "Drone", value: "alive, idle" }
        ],
        acts: /* @__PURE__ */ jsxs(Fragment, { children: [
          /* @__PURE__ */ jsx(Button, { variant: "secondary", children: "Restart step" }),
          /* @__PURE__ */ jsx(Button, { variant: "primary", children: "Redirect" })
        ] }),
        notice: {
          tone: "stopped",
          title: "Three attempts at the same failure. The Drone is holding, waiting on you.",
          children: "The same test has failed each time — visible_manifests_memoises. The Drone still has its session and its worktree, so a word from you costs no respawn."
        },
        phases: {
          note: "A Check that fails ends the step before the Judge reads anything, so there is no verdict here. The Judge tier was never reached.",
          stages: [
            { id: "instructed", label: "Instructed", state: "cleared" },
            { id: "working", label: "Working", state: "cleared" },
            { id: "submitted", label: "Submitted", state: "cleared" },
            {
              id: "checks",
              label: "test failed · retries spent",
              kind: "checks",
              state: "failed",
              stands: "exit 101 · 3 of 3 attempts",
              rows: [
                { label: "cargo build --workspace --locked", mono: true, result: "exit 0 · 47s", named: "passed" },
                { label: "cargo nextest run --workspace", mono: true, result: "exit 101 · same failure ×3", named: "failed" }
              ]
            },
            { id: "judge", label: "Judge · 2 criteria", kind: "judge", state: "ahead", stands: "not reached" },
            { id: "you", label: "You", kind: "human", state: "ahead" }
          ]
        },
        before: /* @__PURE__ */ jsxs(Fragment, { children: [
          /* @__PURE__ */ jsxs("div", { className: "armada-screen__sunken", children: [
            /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", children: "The failure, every time" }),
            /* @__PURE__ */ jsx("pre", { className: "armada-screen__output", children: `FAIL settings::selectors::visible_manifests_memoises
  assert_eq!(a, b) — expected the same reference on repeat calls
  left:  Manifests([..]) @0x7f9c2a
  right: Manifests([..]) @0x7f9c31
  packages/settings/test/selectors.test.ts:112` }),
            /* @__PURE__ */ jsx("p", { className: "armada-screen__caption", "data-note": true, children: "The same assertion, at the same line, on all three attempts." })
          ] }),
          /* @__PURE__ */ jsxs("div", { className: "armada-screen__sunken", children: [
            /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", children: "What it tried, and what it said it was doing" }),
            /* @__PURE__ */ jsx("p", { className: "armada-screen__why", children: "Attempt 1 · +18 −4 selectors.ts · same failure — memoised on the selector itself with a module-level cache." }),
            /* @__PURE__ */ jsx("p", { className: "armada-screen__why", children: "Attempt 2 · +22 −18 selectors.ts · same failure — replaced the cache with a WeakMap keyed on the state object." }),
            /* @__PURE__ */ jsx("p", { className: "armada-screen__why", children: "Attempt 3 · +6 −22 selectors.ts · same failure — went back to the module cache and widened the key." }),
            /* @__PURE__ */ jsx("p", { className: "armada-screen__recourse", children: "Three different fixes, one unchanged failure. It is caching in the wrong place, not caching wrongly." })
          ] })
        ] }),
        chapters: [
          { ...REPAIR_CHAPTERS[0], summary: "14:44:20" },
          { ...REPAIR_CHAPTERS[1], summary: "126 entries · three attempts" },
          { ...REPAIR_CHAPTERS[2], summary: "4 files · on the branch" }
        ],
        after: /* @__PURE__ */ jsxs("div", { className: "armada-screen__sunken", children: [
          /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", children: "Tell it what it is missing — the Drone carries on, no attempt spent" }),
          /* @__PURE__ */ jsx("p", { className: "armada-screen__why", children: "Before it stopped, the Drone was asked what it would try next. Picking one drafts the instruction; it stays yours to edit, and writing your own from nothing is always available." }),
          /* @__PURE__ */ jsxs("div", { className: "armada-screen__actions", children: [
            /* @__PURE__ */ jsx(Button, { variant: "primary", children: "Redirect" }),
            /* @__PURE__ */ jsx(Button, { variant: "secondary", children: "Restart step" }),
            /* @__PURE__ */ jsx(Button, { variant: "ghost", children: "Redispatch" }),
            /* @__PURE__ */ jsx(Button, { variant: "destructive", children: "Kill" })
          ] })
        ] })
      }
    }
  ) })
};
const Failed$1 = {
  render: () => /* @__PURE__ */ jsx("div", { className: "armada-screen", children: /* @__PURE__ */ jsx(
    InsideAJob,
    {
      heading: { ...FAILED_HEADING, actions: /* @__PURE__ */ jsx(Button, { variant: "ghost", children: "Redispatch" }) },
      run: RUN_FAILED,
      runElapsed: "13m 54s",
      pulsing: false,
      where: WHERE,
      whereNote: "The worktree and the branch are left in place. Nothing was rolled back.",
      brief: BRIEF,
      step: {
        label: "Regression check",
        fields: [
          { label: "Took", value: "2m 51s", mono: true },
          { label: "Attempt", value: "1", mono: true },
          { label: "Drone", value: "gone" }
        ],
        notice: {
          tone: "failed",
          title: "A Check failed and the Job ended at completed_failed.",
          children: "cargo nextest run --workspace exited 101. The Judge was never reached, and nothing below this step ran."
        },
        phases: {
          note: "Nothing advances this Job. Redispatch mints a replacement; it does not reopen this one.",
          stages: [
            { id: "instructed", label: "Instructed", state: "cleared" },
            { id: "working", label: "Working", state: "cleared" },
            { id: "submitted", label: "Submitted", state: "cleared" },
            {
              id: "checks",
              label: "test failed",
              kind: "checks",
              state: "failed",
              stands: "exit 101",
              rows: [
                { label: "cargo build --workspace --locked", mono: true, result: "exit 0 · 47s", named: "passed" },
                { label: "cargo nextest run --workspace", mono: true, result: "exit 101", named: "failed" }
              ]
            },
            { id: "judge", label: "Judge · 2 criteria", kind: "judge", state: "ahead", stands: "not reached" }
          ]
        },
        chapters: REPAIR_CHAPTERS
      }
    }
  ) })
};
const NothingServesTheStep = {
  render: () => /* @__PURE__ */ jsx("div", { className: "armada-screen", children: /* @__PURE__ */ jsx(
    InsideAJob,
    {
      heading: { ...HEADING, actions: JOB_ACTS },
      run: [],
      runAbsent: "Fleet did not answer for this Job, so its steps are unknown.",
      where: void 0,
      whereAbsent: "Nothing serves this Job's paths, and no branch exists yet.",
      brief: void 0,
      briefAbsent: "Nothing serves this Job's brief or its acceptance criteria.",
      step: void 0,
      stepAbsent: "No step is open, because the run could not be read."
    }
  ) })
};
const __vite_glob_0_69 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  ACheckFailed,
  Failed: Failed$1,
  NothingServesTheStep,
  OutOfAttempts,
  Running: Running$1,
  WaitingOnYou,
  default: meta$2
}, Symbol.toStringTag, { value: "Module" }));
const APPROVAL_TRACKS = [
  "var(--armada-track-origin)",
  "var(--armada-track-bar)",
  "var(--armada-track-step)",
  "var(--armada-track-created)",
  "var(--armada-track-provenance)"
].join(" ");
function TheListSixStatesOneRowShape({
  heading: heading2,
  summary,
  action,
  controls,
  rows,
  empty,
  selectable,
  label: label2,
  onCopied
}) {
  return /* @__PURE__ */ jsx(
    ActiveJobsList,
    {
      heading: heading2,
      summary,
      action,
      controls,
      empty,
      selectable,
      label: label2,
      children: rows.map((row, index) => /* @__PURE__ */ jsx(
        JobRowStacked,
        {
          ...row,
          onCopied: row.onCopied ?? onCopied
        },
        row.jobId ?? index
      ))
    }
  );
}
const meta$1 = {
  title: "Screens/The list — six states, one row shape",
  component: TheListSixStatesOneRowShape
};
const menu = [
  { label: "Copy job id", shortcut: "⌘C" },
  { label: "Kill", shortcut: "x", danger: true }
];
const open = /* @__PURE__ */ jsx(SplitButton, { ground: "card", items: menu, children: "Open" });
const workflow = /* @__PURE__ */ jsxs(Fragment, { children: [
  /* @__PURE__ */ jsx("span", { className: "armada-screen__mono", children: "bug" }),
  ", 4 steps"
] });
const awaitingApproval = {
  status: "awaiting-approval",
  statusIcon: UserCheck,
  statusLabel: "Needs approval",
  headline: "Coalesce concurrent token refreshes",
  jobId: "job_7c31",
  tracks: APPROVAL_TRACKS,
  fields: [
    { value: workflow },
    { value: /* @__PURE__ */ jsx(StepBar, { total: 4, current: 0, label: "Not started, 4 steps" }) },
    { value: "Not started", quiet: true },
    { value: "created 09:12", quiet: true },
    { value: "Dispatched by you" }
  ],
  // **Review, not Approve.** The drawing gave this row an Approve control and
  // flagged it as a departure from the settled rule that approval is a second
  // act from detail; it was settled 2026-08-31 in favour of the rule. Review is
  // the word an `awaiting_review` row already carries and means the same thing
  // in both places — go read this — because in both places the act is on
  // detail. Nothing on the Board approves.
  action: /* @__PURE__ */ jsx(SplitButton, { ground: "card", items: menu, children: "Review" })
};
const queued = {
  status: "not-started",
  statusIcon: Cpu,
  // The reason supplies the verb as well as the glyph. This carried cpu with
  // "Queued" beside it, which is half the registry's rule, and the field
  // beneath said "Waiting on a drone" — the reason a second time, and named
  // for the one drone Fleet used to run. The resource is the concurrency cap.
  statusLabel: "Waiting on resources",
  headline: "Retire the legacy poke path",
  jobId: "job_8b42",
  tracks: APPROVAL_TRACKS,
  fields: [
    { value: workflow },
    { value: /* @__PURE__ */ jsx(StepBar, { total: 4, current: 0, label: "Not started, 4 steps" }) },
    { value: "Not started", quiet: true },
    { value: "approved 09:20", quiet: true },
    { value: "Dispatched by you" }
  ],
  action: open
};
const running = {
  status: "running",
  statusIcon: CircleDot,
  statusLabel: "Running",
  headline: "Split the settings reducer",
  jobId: "job_2d90bb",
  pulsing: true,
  fields: [
    {
      value: "fix/settings-split",
      mono: true,
      icon: GitBranch,
      copyValue: "fix/settings-split"
    },
    { value: /* @__PURE__ */ jsx(StepBar, { total: 4, current: 2, activity: "running", label: "Step 2 of 4" }) },
    { value: "Implement", emphasis: true },
    { value: "11m 03s", mono: true },
    { value: "~$1.80", mono: true },
    { value: "Dispatched by you" }
  ],
  action: open
};
const failed = {
  status: "completed-failed",
  statusIcon: X,
  statusLabel: "Failed",
  headline: "Cache the manifest read",
  jobId: "job_91ab",
  fields: [
    {
      value: "feat/manifest-cache",
      mono: true,
      icon: GitBranch,
      copyValue: "feat/manifest-cache"
    },
    { value: /* @__PURE__ */ jsx(StepBar, { total: 4, current: 3, activity: "failed", label: "Step 3 of 4" }) },
    { value: "Run tests", emphasis: true },
    { value: "22m 41s", mono: true },
    { value: "~$2.10", mono: true },
    { value: "Found by Fleet" }
  ],
  action: open
};
const done = {
  status: "completed-success",
  statusIcon: Check,
  statusLabel: "Done",
  headline: "Add a retry ceiling to the poke loop",
  jobId: "job_4f10",
  fields: [
    {
      value: "fix/poke-ceiling",
      mono: true,
      icon: GitBranch,
      copyValue: "fix/poke-ceiling"
    },
    {
      value: /* @__PURE__ */ jsx(StepBar, { total: 4, current: 5, activity: "advanced", label: "All 4 of 4 steps advanced" })
    },
    { value: "Summarise" },
    { value: "18m 22s", mono: true },
    { value: "~$2.40", mono: true },
    { value: "Drafted in Helm" }
  ],
  action: open
};
const killed = {
  status: "killed",
  statusIcon: Power,
  statusLabel: "Killed",
  headline: "Rename the session token field",
  jobId: "job_5e88",
  fields: [
    {
      value: "feat/session-rename",
      mono: true,
      icon: GitBranch,
      copyValue: "feat/session-rename"
    },
    { value: /* @__PURE__ */ jsx(StepBar, { total: 4, current: 2, activity: "killed", label: "Step 2 of 4" }) },
    { value: "Implement", emphasis: true },
    { value: "4m 09s", mono: true },
    { value: "~$0.60", mono: true },
    { value: "Workflow-triggered" }
  ],
  action: open
};
const SIX = [awaitingApproval, queued, running, failed, done, killed];
function one(row) {
  return /* @__PURE__ */ jsx("div", { className: "armada-screen", children: /* @__PURE__ */ jsx(TheListSixStatesOneRowShape, { label: "Active jobs", rows: [row] }) });
}
const TheList = {
  render: () => /* @__PURE__ */ jsx("div", { className: "armada-screen", children: /* @__PURE__ */ jsx(
    TheListSixStatesOneRowShape,
    {
      heading: "Active jobs",
      summary: "6 jobs. 1 awaiting approval.",
      action: /* @__PURE__ */ jsx(Button, { variant: "primary", children: "New job" }),
      rows: SIX
    }
  ) })
};
const TheBoard = {
  render: () => /* @__PURE__ */ jsx("div", { className: "armada-screen", children: /* @__PURE__ */ jsx(
    TheListSixStatesOneRowShape,
    {
      heading: "Active jobs",
      summary: "1 job needs you. 6 on the Board.",
      action: /* @__PURE__ */ jsx(Button, { variant: "primary", children: "New job" }),
      controls: /* @__PURE__ */ jsx(
        BoardControls,
        {
          query: "",
          onQuery: () => {
          },
          searchKey: "/",
          sorts: [
            { id: "critical_first", label: "Critical first" },
            { id: "oldest_first", label: "Oldest first" }
          ],
          sort: "critical_first",
          onSort: () => {
          },
          tabs: [
            { id: "all", label: "All", count: 6, shortcut: "1" },
            { id: "needs-you", label: "Needs you", count: 1, shortcut: "2" },
            { id: "running", label: "Running", count: 1, shortcut: "3" },
            { id: "queued", label: "Queued", count: 1, shortcut: "4" },
            { id: "finished", label: "Finished", count: 3, shortcut: "5" }
          ],
          tab: "all",
          onTab: () => {
          }
        }
      ),
      rows: SIX.map((row, i) => ({
        ...row,
        actionKey: KEYS[i],
        focused: i === 0 || void 0
      }))
    }
  ) })
};
const KEYS = ["r", "o", "o", "o", "o", "o"];
const AwaitingApproval = { render: () => one(awaitingApproval) };
const Queued = { render: () => one(queued) };
const Running = { render: () => one(running) };
const Failed = { render: () => one(failed) };
const Done = { render: () => one(done) };
const Killed = { render: () => one(killed) };
const WhatTheWireServes = {
  render: () => /* @__PURE__ */ jsx("div", { className: "armada-screen", children: /* @__PURE__ */ jsx(
    TheListSixStatesOneRowShape,
    {
      heading: "Active jobs",
      summary: "6 jobs. 1 awaiting approval.",
      rows: SIX.map((row) => ({
        ...row,
        tracks: void 0,
        // Track one is the branch where the row has one and the workflow
        // where it does not; the step is the id it was dispatched on; and
        // elapsed is only on a Job still working. Spend is dropped, never
        // drawn empty.
        fields: [
          row === awaitingApproval || row === queued ? { value: workflow } : row.fields[0],
          row.fields[1],
          row === awaitingApproval || row === queued ? { value: "Not started", quiet: true } : { ...row.fields[2], mono: true },
          ...row === awaitingApproval || row === queued || row === running ? [{ value: STILL_RUNNING[SIX.indexOf(row)] ?? "", mono: true, quiet: true }] : []
        ]
      }))
    }
  ) })
};
const STILL_RUNNING = { 0: "1h 04m", 1: "38m 12s", 2: "11m 03s" };
const __vite_glob_0_70 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AwaitingApproval,
  Done,
  Failed,
  Killed,
  Queued,
  Running,
  TheBoard,
  TheList,
  WhatTheWireServes,
  default: meta$1
}, Symbol.toStringTag, { value: "Module" }));
function TheShell({
  appName = "Armada",
  railHeader,
  surfaces: surfaces2,
  activeId,
  collapsed,
  onSelect,
  title,
  summary,
  actions,
  children,
  status
}) {
  return /* @__PURE__ */ jsxs("div", { className: "armada-shell", children: [
    /* @__PURE__ */ jsxs("div", { className: "armada-shell__body", children: [
      /* @__PURE__ */ jsx(
        Sidebar,
        {
          appName,
          header: railHeader,
          surfaces: surfaces2,
          activeId,
          collapsed,
          onSelect
        }
      ),
      /* @__PURE__ */ jsxs("div", { className: "armada-shell__panel", children: [
        /* @__PURE__ */ jsxs("div", { className: "armada-screen__panel-head", children: [
          /* @__PURE__ */ jsxs("div", { className: "armada-screen__titles", children: [
            /* @__PURE__ */ jsx("span", { className: "armada-screen__title", children: title }),
            summary === void 0 ? null : /* @__PURE__ */ jsx("span", { className: "armada-screen__summary", children: summary })
          ] }),
          actions === void 0 ? null : /* @__PURE__ */ jsx("div", { className: "armada-shell__actions", children: actions })
        ] }),
        /* @__PURE__ */ jsx("div", { className: "armada-shell__mount", children })
      ] })
    ] }),
    /* @__PURE__ */ jsx(StatusBar, { ...status })
  ] });
}
const meta = {
  title: "Screens/The shell",
  component: TheShell
};
const shell = {
  railHeader: /* @__PURE__ */ jsx(Select, { "aria-label": "Project", children: /* @__PURE__ */ jsx("option", { children: "armada" }) }),
  // The drawing's rail row carries a label and a count and no glyph. Sidebar
  // requires one, and `activity` is what the registry assigns to Active jobs,
  // so that is the glyph. Reported.
  surfaces: [{ id: "active", label: "Active jobs", icon: Activity, count: 6 }],
  activeId: "active",
  title: "Active jobs",
  summary: "6 jobs. 1 awaiting approval.",
  actions: /* @__PURE__ */ jsx(Button, { variant: "primary", children: "New job" }),
  children: /* @__PURE__ */ jsx("div", { className: "armada-screen__mount", children: "The list mounts here — 1d" }),
  status: {
    fleet: "running",
    fleetLabel: "Fleet running",
    detail: "pid 4417 · port 7411",
    items: ["6 jobs"],
    approvals: 1
  }
};
const Shell = {
  args: shell,
  render: (args) => /* @__PURE__ */ jsx("div", { className: "armada-screen", children: /* @__PURE__ */ jsx("div", { className: "armada-screen__window", children: /* @__PURE__ */ jsx(TheShell, { ...args }) }) })
};
const CollapsedRail = {
  args: { ...shell, collapsed: true },
  render: Shell.render
};
const FleetIsNotRunning = {
  args: {
    ...shell,
    summary: "No jobs.",
    surfaces: [{ id: "active", label: "Active jobs", icon: Activity, count: 0 }],
    status: {
      fleet: "not-running",
      fleetLabel: "Fleet is not running",
      detail: "no runtime file at ~/Library/Application Support/Armada/fleet.json",
      advice: "Start Fleet. Bridge reconnects on its own."
    }
  },
  render: Shell.render
};
const __vite_glob_0_71 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  CollapsedRail,
  FleetIsNotRunning,
  Shell,
  default: meta
}, Symbol.toStringTag, { value: "Module" }));
const stories = /* @__PURE__ */ Object.assign({
  "../src/compositions/ActiveJobsList/ActiveJobsList.stories.tsx": __vite_glob_0_0,
  "../src/compositions/ActivityLog/ActivityLog.stories.tsx": __vite_glob_0_1,
  "../src/compositions/BoardControls/BoardControls.stories.tsx": __vite_glob_0_2,
  "../src/compositions/BoardEmptyState/BoardEmptyState.stories.tsx": __vite_glob_0_3,
  "../src/compositions/ChangedFiles/ChangedFiles.stories.tsx": __vite_glob_0_4,
  "../src/compositions/Chapter/Chapter.stories.tsx": __vite_glob_0_5,
  "../src/compositions/CriterionVerdicts/CriterionVerdicts.stories.tsx": __vite_glob_0_6,
  "../src/compositions/DroneQuestion/DroneQuestion.stories.tsx": __vite_glob_0_7,
  "../src/compositions/DroneTurns/DroneTurns.stories.tsx": __vite_glob_0_8,
  "../src/compositions/EvidenceCard/EvidenceCard.stories.tsx": __vite_glob_0_9,
  "../src/compositions/EvidenceTrail/EvidenceTrail.stories.tsx": __vite_glob_0_10,
  "../src/compositions/FactChip/FactChip.stories.tsx": __vite_glob_0_11,
  "../src/compositions/FailureNotice/FailureNotice.stories.tsx": __vite_glob_0_12,
  "../src/compositions/GamingFlags/GamingFlags.stories.tsx": __vite_glob_0_13,
  "../src/compositions/JobBrief/JobBrief.stories.tsx": __vite_glob_0_14,
  "../src/compositions/JobComposer/JobComposer.stories.tsx": __vite_glob_0_15,
  "../src/compositions/JobDetailHeaderActions/JobDetailHeaderActions.stories.tsx": __vite_glob_0_16,
  "../src/compositions/JobLogReference/JobLogReference.stories.tsx": __vite_glob_0_17,
  "../src/compositions/JobOutcome/JobOutcome.stories.tsx": __vite_glob_0_18,
  "../src/compositions/JobRecord/JobRecord.stories.tsx": __vite_glob_0_19,
  "../src/compositions/JobRowStacked/JobRowStacked.stories.tsx": __vite_glob_0_20,
  "../src/compositions/LogEntry/LogEntry.stories.tsx": __vite_glob_0_21,
  "../src/compositions/PathChip/PathChip.stories.tsx": __vite_glob_0_22,
  "../src/compositions/PhaseCard/PhaseCard.stories.tsx": __vite_glob_0_23,
  "../src/compositions/PhaseStrip/PhaseStrip.stories.tsx": __vite_glob_0_24,
  "../src/compositions/ReviewDecision/ReviewDecision.stories.tsx": __vite_glob_0_25,
  "../src/compositions/RunTree/RunTree.stories.tsx": __vite_glob_0_26,
  "../src/compositions/Sidebar/Sidebar.stories.tsx": __vite_glob_0_27,
  "../src/compositions/StatusBar/StatusBar.stories.tsx": __vite_glob_0_28,
  "../src/compositions/StepActivityMark/StepActivityMark.stories.tsx": __vite_glob_0_29,
  "../src/compositions/StepBar/StepBar.stories.tsx": __vite_glob_0_30,
  "../src/compositions/StepRow/StepRow.stories.tsx": __vite_glob_0_31,
  "../src/compositions/StepStory/StepStory.stories.tsx": __vite_glob_0_32,
  "../src/compositions/TransitionHistory/TransitionHistory.stories.tsx": __vite_glob_0_33,
  "../src/compositions/UnifiedDiff/UnifiedDiff.stories.tsx": __vite_glob_0_34,
  "../src/compositions/WhereRow/WhereRow.stories.tsx": __vite_glob_0_35,
  "../src/compositions/WorkflowRail/WorkflowRail.stories.tsx": __vite_glob_0_36,
  "../src/errors/ErrorCode/ErrorCode.stories.tsx": __vite_glob_0_37,
  "../src/errors/ErrorNotice/ErrorNotice.stories.tsx": __vite_glob_0_38,
  "../src/errors/FileAnIssue/FileAnIssue.stories.tsx": __vite_glob_0_39,
  "../src/primitives/Alert/Alert.stories.tsx": __vite_glob_0_40,
  "../src/primitives/AttachmentChip/AttachmentChip.stories.tsx": __vite_glob_0_41,
  "../src/primitives/Badge/Badge.stories.tsx": __vite_glob_0_42,
  "../src/primitives/Button/Button.stories.tsx": __vite_glob_0_43,
  "../src/primitives/Card/Card.stories.tsx": __vite_glob_0_44,
  "../src/primitives/Checkbox/Checkbox.stories.tsx": __vite_glob_0_45,
  "../src/primitives/CommandPalette/CommandPalette.stories.tsx": __vite_glob_0_46,
  "../src/primitives/Dialog/Dialog.stories.tsx": __vite_glob_0_47,
  "../src/primitives/DropdownMenu/DropdownMenu.stories.tsx": __vite_glob_0_48,
  "../src/primitives/Input/Input.stories.tsx": __vite_glob_0_49,
  "../src/primitives/Kbd/Kbd.stories.tsx": __vite_glob_0_50,
  "../src/primitives/Popover/Popover.stories.tsx": __vite_glob_0_51,
  "../src/primitives/Prose/Prose.stories.tsx": __vite_glob_0_52,
  "../src/primitives/Radio/Radio.stories.tsx": __vite_glob_0_53,
  "../src/primitives/ScrollArea/ScrollArea.stories.tsx": __vite_glob_0_54,
  "../src/primitives/Select/Select.stories.tsx": __vite_glob_0_55,
  "../src/primitives/Separator/Separator.stories.tsx": __vite_glob_0_56,
  "../src/primitives/Sheet/Sheet.stories.tsx": __vite_glob_0_57,
  "../src/primitives/Skeleton/Skeleton.stories.tsx": __vite_glob_0_58,
  "../src/primitives/SplitButton/SplitButton.stories.tsx": __vite_glob_0_59,
  "../src/primitives/Switch/Switch.stories.tsx": __vite_glob_0_60,
  "../src/primitives/Table/Table.stories.tsx": __vite_glob_0_61,
  "../src/primitives/Tabs/Tabs.stories.tsx": __vite_glob_0_62,
  "../src/primitives/TabsWithCounts/TabsWithCounts.stories.tsx": __vite_glob_0_63,
  "../src/primitives/Textarea/Textarea.stories.tsx": __vite_glob_0_64,
  "../src/primitives/Toast/Toast.stories.tsx": __vite_glob_0_65,
  "../src/primitives/Tooltip/Tooltip.stories.tsx": __vite_glob_0_66,
  "../src/screens/DispatchAJobFullWithTheM1SubsetMarked/DispatchAJobFullWithTheM1SubsetMarked.stories.tsx": __vite_glob_0_67,
  "../src/screens/FirstLaunch/FirstLaunch.stories.tsx": __vite_glob_0_68,
  "../src/screens/InsideAJobOneArrangementAtEveryState/InsideAJobOneArrangementAtEveryState.stories.tsx": __vite_glob_0_69,
  "../src/screens/TheListSixStatesOneRowShape/TheListSixStatesOneRowShape.stories.tsx": __vite_glob_0_70,
  "../src/screens/TheShell/TheShell.stories.tsx": __vite_glob_0_71
});
function label(key) {
  return key.replace(/([A-Z])/g, " $1").replace(/^./, (c) => c.toUpperCase()).trim();
}
function collect() {
  const out = [];
  for (const mod of Object.values(stories)) {
    const meta2 = mod.default;
    if (!meta2?.title) continue;
    const Component = meta2.component;
    const rendered = [];
    for (const [key, story] of Object.entries(mod)) {
      if (key === "default" || !story || typeof story !== "object") continue;
      const s = story;
      try {
        const el = s.render ? s.render({ ...meta2.args, ...s.args }) : Component ? createElement(Component, { ...meta2.args, ...s.args }) : null;
        if (el) rendered.push({ name: label(key), html: renderToStaticMarkup(el) });
      } catch (e) {
        rendered.push({
          name: label(key),
          html: `<p style="color:var(--status-failed)">did not render: ${String(e)}</p>`
        });
      }
    }
    if (rendered.length) out.push({ title: meta2.title, stories: rendered });
  }
  return out.sort((a, b) => a.title.localeCompare(b.title));
}
export {
  collect
};
