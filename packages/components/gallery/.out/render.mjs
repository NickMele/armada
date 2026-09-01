import { jsx, jsxs, Fragment } from "react/jsx-runtime";
import { useState, createContext, useCallback, useContext, useRef, useEffect, Children, cloneElement, useId, Fragment as Fragment$1, useMemo, createElement } from "react";
import { ChevronDown, UserCheck, Cpu, GitBranch, CircleDot, X, Check, Power, Flag, RotateCw, Eye, CircleX, CircleCheck, ShieldCheck, ChevronRight, ExternalLink, File, TriangleAlert, OctagonAlert, Folder, GitCommitHorizontal, GitPullRequest, FileCheck, Clock, MessageSquare, ClipboardList, Activity, Bell, ScrollText, Stethoscope, FileCog, ShieldMinus, ShieldOff, ShieldX, Lock, Stamp, Terminal, Link, Ban, Archive, RefreshCw, FileQuestionMark, Split, Unplug, ArrowUpToLine, Send, CornerUpRight, Settings } from "lucide-react";
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
const meta$$ = {
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
  default: meta$$
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
const meta$_ = {
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
function Live(props) {
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
const Resting$2 = { render: () => /* @__PURE__ */ jsx(Live, {}) };
const Searching = {
  render: () => /* @__PURE__ */ jsx(
    Live,
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
    Live,
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
const __vite_glob_0_1 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  NothingNeedsYou,
  Resting: Resting$2,
  Searching,
  default: meta$_
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
const meta$Z = {
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
const __vite_glob_0_2 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  FleetIsNotRunning: FleetIsNotRunning$1,
  FleetRunningNoJobs,
  default: meta$Z
}, Symbol.toStringTag, { value: "Module" }));
const OUTSIDE_PLAN$1 = "outside plan";
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
  return /* @__PURE__ */ jsxs("div", { className: "armada-files", children: [
    /* @__PURE__ */ jsx("ol", { className: "armada-files__list", children: files.map((file) => /* @__PURE__ */ jsxs(
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
          /* @__PURE__ */ jsx("span", { className: "armada-files__mark", children: file.outsidePlan === true ? OUTSIDE_PLAN$1 : null })
        ]
      },
      file.path
    )) }),
    note === void 0 ? null : /* @__PURE__ */ jsx("p", { className: "armada-files__note", children: note })
  ] });
}
const meta$Y = {
  title: "Compositions/Changed files",
  component: ChangedFiles
};
const NOTHING_YET$3 = "This drone has not changed anything yet.";
const WhatADroneHasTouched = {
  args: {
    emptyNote: NOTHING_YET$3,
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
    emptyNote: NOTHING_YET$3,
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
    emptyNote: NOTHING_YET$3,
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
    emptyNote: NOTHING_YET$3,
    files: [
      { path: "docs/scope.md", change: "renamed" },
      { path: "docs/journeys/watch-a-drone.md", change: "copied" },
      { path: "crates/fleet/src/serving.rs", change: "conflicted" },
      { path: "assets/AppIcon.icns", change: "unreadable" }
    ]
  }
};
const NothingChangedYet = {
  args: { files: [], emptyNote: NOTHING_YET$3 }
};
const __vite_glob_0_3 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  NoPlanIsRecordedAgainstIt,
  NothingChangedYet,
  TheKindsThatAreNotAnEdit,
  TwoPathsOutsideThePlan,
  WhatADroneHasTouched,
  default: meta$Y
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
    if (heading$5(trimmed)) {
      blocks.push({ kind: "said", line: trimmed.replace(/^#+\s+/, "") });
      at += 1;
      continue;
    }
    const held = [];
    while (at < lines.length) {
      const next = (lines[at] ?? "").trim();
      if (next === "" || next.startsWith(FENCE) || bulleted(next) || heading$5(next)) break;
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
function heading$5(line) {
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
function GamingFlags({ flags, said, citation = "clipped" }) {
  if (flags.length === 0) return null;
  return /* @__PURE__ */ jsxs("div", { className: "armada-gaming-flags", "data-citation": citation, children: [
    said === void 0 ? null : /* @__PURE__ */ jsx("span", { className: "armada-gaming-flags__said", children: said }),
    /* @__PURE__ */ jsx("ul", { className: "armada-gaming-flags__list", children: flags.map((flag, at) => /* @__PURE__ */ jsxs("li", { className: "armada-gaming-flags__flag", children: [
      /* @__PURE__ */ jsx("span", { className: "armada-gaming-flags__pattern", children: flag.pattern }),
      flag.cited === void 0 || flag.cited === "" ? null : citation === "whole" ? /* @__PURE__ */ jsx("div", { className: "armada-gaming-flags__cited", children: /* @__PURE__ */ jsx(Prose, { text: flag.cited }) }) : (
        // The whole citation stays in the title however narrow the row
        // gets, the way the Check's output path does.
        /* @__PURE__ */ jsx("span", { className: "armada-gaming-flags__cited", title: flag.cited, children: flag.cited })
      )
    ] }, `flag-${at}`)) })
  ] });
}
const GLYPH = {
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
  const Icon = GLYPH[activity];
  const animates = pulsing && activity === "running";
  return /* @__PURE__ */ jsxs("span", { className: "armada-step-mark", "data-activity": activity, "data-pulsing": animates || void 0, children: [
    Icon ? /* @__PURE__ */ jsx(Icon, { size: MARK_ICON, strokeWidth: MARK_STROKE$1, "aria-hidden": true }) : ordinal !== void 0 ? /* @__PURE__ */ jsx("span", { className: "armada-step-mark__ordinal", "aria-hidden": true, children: ordinal }) : null,
    /* @__PURE__ */ jsx("span", { className: "armada-step-mark__name", children: label2 })
  ] });
}
const GATE_ICON = 12;
const GATE_STROKE = 2;
const FLAGGED = "the gaming check flagged this evidence";
function named(step) {
  return (step.evidence?.label ?? "") !== "";
}
function WorkflowRail({ steps: steps2, pulsing = false, onCopied }) {
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
  return /* @__PURE__ */ jsx("ol", { className: "armada-rail", children: steps2.map((step, i) => {
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
const meta$X = {
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
const __vite_glob_0_4 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  ARefusal,
  BeneathTheStepItJudged,
  NoObjection,
  RefusalsSortFirst,
  TheCriterionIsNotOnScreen,
  TheRegistryHasNoWordForIt,
  WhereTheBriefWasKept,
  default: meta$X
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
const MARK = 12;
const MARK_STROKE = 2;
const CARET = 16;
function QuietRun({ turns: turns2, working, open: open2, onToggle }) {
  const head = turns2[0];
  const held = open2 ? turns2.map((turn) => rowId(turn)).join(" ") : void 0;
  return /* @__PURE__ */ jsxs(Fragment$1, { children: [
    /* @__PURE__ */ jsxs("li", { className: "armada-turns__turn", "data-quiet": true, "data-open": open2 || void 0, children: [
      /* @__PURE__ */ jsx("span", { className: "armada-turns__at", children: head.at }),
      /* @__PURE__ */ jsx("span", { className: "armada-turns__mark", "data-working": working || void 0, children: /* @__PURE__ */ jsx(CircleDot, { size: MARK, strokeWidth: MARK_STROKE, "aria-hidden": true }) }),
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
const meta$W = {
  title: "Compositions/Drone turns",
  component: DroneTurns
};
const NOTHING_YET$2 = "This job has no turns. It was never dispatched, so no drone has written one.";
function thinking$1(from, rows, at) {
  return Array.from({ length: rows }, (_, n) => ({
    id: String(from + n),
    at,
    kind: "unrecognised",
    subject: n % 4 === 3 ? "a turn with nothing in it Armada names" : "system/thinking_tokens",
    quiet: true
  }));
}
const turns$1 = [
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
  args: { turns: turns$1, emptyNote: NOTHING_YET$2 }
};
const AJobWithNoTranscript$1 = {
  args: { turns: [], emptyNote: NOTHING_YET$2 }
};
const RefusedUnrecognisedAndUnreadable = {
  args: {
    emptyNote: NOTHING_YET$2,
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
    emptyNote: NOTHING_YET$2,
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
const ADroneThinking$1 = {
  args: {
    live: true,
    emptyNote: NOTHING_YET$2,
    turns: [
      { id: "1", at: "09:14:02", kind: "said", said: "Starting on the settings split." },
      ...thinking$1(10, 9, "09:14:03"),
      { id: "20", at: "09:14:12", kind: "called", subject: "Read", detail: "src/settings.rs", answer: "Answered." },
      ...thinking$1(30, 14, "09:14:15"),
      { id: "50", at: "09:14:31", kind: "called", subject: "Edit", detail: "src/settings.rs +42 -18", answer: "Answered." },
      ...thinking$1(60, 6, "09:14:40")
    ]
  }
};
const AFinishedRun$1 = {
  args: {
    live: false,
    emptyNote: NOTHING_YET$2,
    turns: [
      { id: "1", at: "09:14:02", kind: "said", said: "Starting on the settings split." },
      ...thinking$1(10, 9, "09:14:03"),
      { id: "20", at: "09:14:12", kind: "called", subject: "Read", detail: "src/settings.rs", answer: "Answered." },
      ...thinking$1(30, 14, "09:14:15"),
      { id: "50", at: "09:14:31", kind: "called", subject: "Edit", detail: "src/settings.rs +42 -18", answer: "Answered." },
      { id: "60", at: "09:14:44", kind: "said", said: "The public signature is unchanged. Submitting." },
      { id: "61", at: "09:14:45", kind: "ended", subject: "18 turns · ~$0.42 · no calls refused" }
    ]
  }
};
const WhatTheRunCost = {
  args: {
    live: false,
    emptyNote: NOTHING_YET$2,
    turns: [
      { id: "1", at: "09:14:02", kind: "said", said: "Starting on the settings split." },
      { id: "2", at: "09:55:10", kind: "ended", subject: "41 turns · ~$1.53 · 6 calls refused" },
      { id: "3", at: "10:02:11", kind: "said", said: "Retrying the step. Reading the refusal first." },
      { id: "4", at: "10:03:40", kind: "ended", subject: "4 turns · ~$0.0018 · no calls refused" }
    ]
  }
};
const NothingButToolCalls$1 = {
  args: {
    live: true,
    emptyNote: NOTHING_YET$2,
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
    emptyNote: NOTHING_YET$2,
    turns: [
      { id: "1", at: "09:14:02", step: REPRO, kind: "started", subject: "sess_01JB4 · the job's model · 2 mcp servers" },
      { id: "2", at: "09:14:03", step: REPRO, kind: "said", said: "Writing the failing test before I touch the reducer." },
      ...thinking$1(10, 5, "09:14:04").map((turn) => ({ ...turn, step: REPRO })),
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
    emptyNote: NOTHING_YET$2,
    turns: [
      { id: "1", at: "09:22:01", step: { id: "implement", label: "implement", labelIsAnIdentifier: true }, kind: "called", subject: "Edit", detail: "src/settings.rs +42 -18", answer: "Answered." },
      { id: "2", at: "09:24:40", step: { id: "regression_verify", label: "regression_verify", labelIsAnIdentifier: true }, kind: "called", subject: "Bash", detail: "cargo nextest run --workspace", answer: "Answered." },
      { id: "3", at: "09:26:12", step: { id: "write_up", label: "write_up", labelIsAnIdentifier: true }, kind: "said", said: "Submitting the evidence report." }
    ]
  }
};
const RowsWrittenBeforeTheStepWasRecorded = {
  args: {
    emptyNote: NOTHING_YET$2,
    turns: [
      { id: "1", at: "08:59:14", kind: "started", subject: "sess_01J9Z · the job's model · 2 mcp servers" },
      { id: "2", at: "08:59:20", kind: "called", subject: "Read", detail: "src/settings.rs", answer: "Answered." },
      { id: "3", at: "09:01:02", kind: "said", said: "Reading the reducer before I split it." },
      { id: "4", at: "09:12:41", step: FIX, kind: "called", subject: "Edit", detail: "src/settings.rs +42 -18", answer: "Answered." },
      { id: "5", at: "09:13:10", step: FIX, kind: "called", subject: "Bash", detail: "cargo test -p settings --lib", answer: "Answered." }
    ]
  }
};
const __vite_glob_0_5 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  ADroneThinking: ADroneThinking$1,
  ADroneWorking,
  AFinishedRun: AFinishedRun$1,
  AJobWithNoTranscript: AJobWithNoTranscript$1,
  AStepWithNoNameOfItsOwn,
  NothingButToolCalls: NothingButToolCalls$1,
  RefusedUnrecognisedAndUnreadable,
  RowsWrittenBeforeTheStepWasRecorded,
  TurnsUnderTheirSteps,
  WhatEachCallDid,
  WhatTheRunCost,
  default: meta$W
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
const meta$V = {
  title: "Compositions/Evidence card",
  component: EvidenceCard
};
const NO_GLYPH_IN_REGISTRY$4 = void 0;
const PlanTheChange = {
  args: {
    icon: NO_GLYPH_IN_REGISTRY$4,
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
    icon: NO_GLYPH_IN_REGISTRY$4,
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
    icon: NO_GLYPH_IN_REGISTRY$4,
    iconLabel: "Evidence",
    step: "Summarise",
    time: "14:20",
    claimed: "The change is on fix/poke-ceiling and ready to read.",
    shownBy: "3 files +214 −96 · branch fix/poke-ceiling"
  }
};
const __vite_glob_0_6 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AnArtifactThatIsACommand,
  NotClaimedEmpty: NotClaimedEmpty$1,
  PlanTheChange,
  default: meta$V
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
const meta$U = {
  title: "Compositions/Evidence trail",
  component: EvidenceTrail
};
const NO_GLYPH_IN_REGISTRY$3 = void 0;
const AFinishedJob$1 = {
  args: {
    entries: [
      {
        icon: NO_GLYPH_IN_REGISTRY$3,
        iconLabel: "Evidence",
        step: "Plan the change",
        provenance: "14:02 · facts_note · no check",
        claimed: "The poke loop stops after 3 attempts and the job records how many it spent.",
        shownBy: "core/fleet/src/lease.rs · the loop has no ceiling today",
        notClaimed: "Does not change the poke interval, and does not decide what happens at the third failure."
      },
      {
        icon: NO_GLYPH_IN_REGISTRY$3,
        iconLabel: "Evidence",
        step: "Implement",
        provenance: "14:11 · diff · build exit 0 · diff_nonempty passed",
        claimed: "A drone that stops answering is poked at most 3 times, and the count is on the job record.",
        shownBy: "core/fleet/src/lease.rs +38 −7 · core/model/src/job.rs +14 −0",
        notClaimed: "The count is not surfaced in Bridge. Nothing acts on reaching the ceiling yet — the loop exits and the job keeps its status."
      },
      {
        icon: NO_GLYPH_IN_REGISTRY$3,
        iconLabel: "Evidence",
        step: "Run tests",
        provenance: "14:16 · test_suite_run · test exit 0",
        claimed: "The ceiling holds at 3 and the counter increments once per poke.",
        shownBy: "cargo test --workspace · 86 passed 0 failed 5.1s · lease::poke_ceiling_holds, lease::poke_count_increments",
        notClaimed: "No test covers a drone that answers on the third poke. The suite was green before this change and is green after, so it does not prove the ceiling is reached in practice."
      },
      {
        icon: NO_GLYPH_IN_REGISTRY$3,
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
        icon: NO_GLYPH_IN_REGISTRY$3,
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
        icon: NO_GLYPH_IN_REGISTRY$3,
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
const __vite_glob_0_7 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AFinishedJob: AFinishedJob$1,
  NotClaimedEmpty,
  OneEntry,
  default: meta$U
}, Symbol.toStringTag, { value: "Module" }));
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
      const [head, tail2] = halves(row.value);
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
                tail2 === "" ? null : /* @__PURE__ */ jsx("span", { className: "armada-log-ref__tail", children: tail2 })
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
const meta$T = {
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
const __vite_glob_0_8 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AJobCannotBeRead,
  FleetIsNotRunningDeadPid,
  FleetIsNotRunningNoFile,
  FleetIsNotRunningPidReused,
  FleetIsUnreachable,
  FleetRefusedTheCommand,
  NothingButTheSentence,
  TheRendererThrew,
  default: meta$T
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
const meta$S = {
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
const __vite_glob_0_9 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  FlaggedAndNotCited,
  OnTheRail,
  OverrulingTwoFlags,
  TwoFlagsReadInFull,
  default: meta$S
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
      /* @__PURE__ */ jsx("span", { className: "armada-job-brief__label", children: waitingLabel }),
      /* @__PURE__ */ jsx("p", { className: "armada-job-brief__facts", children: waiting })
    ] }),
    only === "facts" ? null : /* @__PURE__ */ jsxs("div", { className: "armada-job-brief__block", children: [
      /* @__PURE__ */ jsx("span", { className: "armada-job-brief__label", children: criteriaLabel }),
      criteria2.length === 0 ? /* @__PURE__ */ jsx("p", { className: "armada-job-brief__note", children: criteriaAbsent }) : /* @__PURE__ */ jsx("ol", { className: "armada-job-brief__criteria", children: criteria2.map((criterion, i) => /* @__PURE__ */ jsxs("li", { className: "armada-job-brief__criterion", children: [
        /* @__PURE__ */ jsx("span", { className: "armada-job-brief__ordinal", children: i + 1 }),
        /* @__PURE__ */ jsx("span", { className: "armada-job-brief__text", children: criterion.text }),
        criterion.source === void 0 ? /* @__PURE__ */ jsx("span", {}) : /* @__PURE__ */ jsx("span", { className: "armada-job-brief__source", children: criterion.source })
      ] }, i)) })
    ] }),
    only === "criteria" ? null : /* @__PURE__ */ jsxs("div", { className: "armada-job-brief__block", children: [
      /* @__PURE__ */ jsx("span", { className: "armada-job-brief__label", children: factsLabel }),
      facts2 === void 0 ? /* @__PURE__ */ jsx("p", { className: "armada-job-brief__note", children: factsAbsent }) : /* @__PURE__ */ jsx("p", { className: "armada-job-brief__facts", children: facts2 })
    ] })
  ] });
}
const meta$R = {
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
const __vite_glob_0_10 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Brief,
  CriteriaOnly,
  FactsOnly,
  NoCriteria,
  NoFacts,
  default: meta$R
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
  brief: brief2,
  workflows,
  project,
  glance,
  provenance,
  onCancel,
  onDispatch
}) {
  return /* @__PURE__ */ jsxs(Card$7, { className: "armada-job-composer", children: [
    /* @__PURE__ */ jsx(Input, { label: "Title", defaultValue: title }),
    /* @__PURE__ */ jsx(Textarea, { label: "Brief", defaultValue: brief2 }),
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
const meta$Q = {
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
const __vite_glob_0_11 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  NoChecksOnTheWorkflow,
  WhatM1Renders,
  default: meta$Q
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
const meta$P = {
  title: "Compositions/Job detail header actions",
  component: JobDetailHeaderActions
};
const ARunningJob$1 = {
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
    ...ARunningJob$1.args,
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
const __vite_glob_0_12 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AFailedJob,
  AFinishedJob,
  ARunningJob: ARunningJob$1,
  AtTheApprovalGate,
  BothKills,
  BothKillsMenuOpen,
  StoppedWithARedispatch,
  default: meta$P
}, Symbol.toStringTag, { value: "Module" }));
const meta$O = {
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
const __vite_glob_0_13 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  LongPaths,
  OnAFailedJob,
  OnAFinishedJob,
  OnARunningJob,
  WhatOpensAndWhatOnlyCopies,
  WhenThePathIsGone,
  WithErrors,
  default: meta$O
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
const meta$N = {
  title: "Compositions/Job outcome",
  component: JobOutcome
};
const NOTE$2 = "Armada does not merge. The branch is pushed and the review is yours to take.";
const WhatIsServedToday = {
  args: {
    note: NOTE$2,
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
const EveryPartServed$1 = {
  args: {
    note: NOTE$2,
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
const __vite_glob_0_14 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  EveryPartServed: EveryPartServed$1,
  NoBranch,
  WhatIsServedToday,
  default: meta$N
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
const meta$M = {
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
const __vite_glob_0_15 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  ASectionOpened,
  FoldedRecord,
  NothingRecorded,
  default: meta$M
}, Symbol.toStringTag, { value: "Module" }));
const meta$L = {
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
const Running$5 = {
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
  args: { ...Running$5.args, focused: true }
};
const FocusedWithItsKey = {
  args: { ...Running$5.args, focused: true, actionKey: "o" }
};
const UnfocusedWithItsKey = {
  args: { ...Running$5.args, actionKey: "o" }
};
const Selected = {
  args: { ...Running$5.args, selected: true }
};
const Dimmed$2 = {
  args: { ...Running$5.args, dimmed: true }
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
const Failed$4 = {
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
    ...Running$5.args,
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
    ...Running$5.args,
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
      { value: "Held for CPU headroom", emphasis: true },
      { value: "queued 09:41", quiet: true },
      { value: "Sub-dispatched by job_2d90bb" }
    ],
    action: /* @__PURE__ */ jsx(SplitButton, { ground: "card", variant: "destructive", items: [], children: "Kill" })
  }
};
const __vite_glob_0_16 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AtTheWidthFloor,
  Convoy,
  Dimmed: Dimmed$2,
  Done: Done$1,
  EscalatedSecondTime,
  EscalatedStalled,
  Failed: Failed$4,
  FocusedWithItsKey,
  Killed: Killed$6,
  NeedsApproval,
  Queued: Queued$2,
  Running: Running$5,
  RunningFocused,
  Selected,
  SpendAsQuota,
  SubDispatchedWaitingOnResources,
  UnfocusedWithItsKey,
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
const __vite_glob_0_17 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  ADecisionAlreadySent,
  ANoteWritten,
  NotConnectedToFleet,
  NothingWrittenYet,
  default: meta$K
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
const meta$J = {
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
const __vite_glob_0_18 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AtMaximumWidth,
  AtMinimumWidth,
  CollapsedRail: CollapsedRail$1,
  Expanded,
  FlatForContrast,
  HelmActive,
  M1OneSurface,
  default: meta$J
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
const meta$I = {
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
    detail: "pid 4417 · port 7411 · 1 drone",
    items: ["3 jobs"],
    spend: "68% quota left"
  }
};
const FleetRunningWorkMachine = {
  args: {
    fleet: "running",
    fleetLabel: "Fleet running",
    detail: "pid 4417 · port 7411 · 1 drone",
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
const AtTheItemCeiling = {
  args: {
    fleet: "running",
    fleetLabel: "Fleet running",
    detail: "pid 4417 · port 7411 · 1 drone",
    items: ["3 jobs"],
    escalations: 2,
    spend: "~$2.40 of $20"
  }
};
const __vite_glob_0_19 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AtTheItemCeiling,
  FleetNotRunning,
  FleetRunningIdle,
  FleetRunningPersonalMachine,
  FleetRunningWorkMachine,
  FleetUnreachable,
  WithBothCounts,
  WithEscalationsOnly,
  WithOneOfEach,
  default: meta$I
}, Symbol.toStringTag, { value: "Module" }));
const meta$H = {
  title: "Compositions/Step activity mark",
  component: StepActivityMark
};
const NotStarted$2 = {
  args: { activity: "not_started", label: "Not started", ordinal: 3 }
};
const NotStartedWithNoOrdinal = {
  args: { activity: "not_started", label: "Not started" }
};
const Running$4 = {
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
const Advanced = {
  args: { activity: "advanced", label: "Advanced" }
};
const Stopped$1 = {
  args: { activity: "stopped", label: "Stopped" }
};
const Killed$5 = {
  args: { activity: "killed", label: "Killed" }
};
const Failed$3 = {
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
const __vite_glob_0_20 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Advanced,
  AwaitingHuman: AwaitingHuman$1,
  EveryValue,
  Failed: Failed$3,
  Killed: Killed$5,
  NotStarted: NotStarted$2,
  NotStartedWithNoOrdinal,
  Retrying,
  Running: Running$4,
  RunningPulsing: RunningPulsing$1,
  Stopped: Stopped$1,
  default: meta$H
}, Symbol.toStringTag, { value: "Module" }));
const meta$G = {
  title: "Compositions/Step bar",
  component: StepBar
};
const NotStarted$1 = {
  args: { total: 4, current: 0, label: "Not started, 4 steps" }
};
const Running$3 = {
  args: { total: 4, current: 2, activity: "running", label: "Step 2 of 4" }
};
const RunningLongWorkflow = {
  args: { total: 7, current: 5, activity: "running", label: "Step 5 of 7" }
};
const AwaitingHuman = {
  args: { total: 4, current: 3, activity: "awaiting_human", label: "Step 3 of 4" }
};
const Failed$2 = {
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
const __vite_glob_0_21 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AllAdvanced,
  AwaitingHuman,
  Failed: Failed$2,
  Killed: Killed$4,
  NotStarted: NotStarted$1,
  Running: Running$3,
  RunningLongWorkflow,
  RunningNeverPulses,
  default: meta$G
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
const meta$F = {
  title: "Compositions/Transition history",
  component: TransitionHistory
};
const NOTE$1 = "What Armada did. What the drone said is in its turns.";
const NOTHING_YET$1 = "This job has not moved yet. Creation is not a transition, so no row describes it.";
const AJobThatRanClean = {
  args: {
    note: NOTE$1,
    emptyNote: NOTHING_YET$1,
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
    note: NOTE$1,
    emptyNote: NOTHING_YET$1,
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
    note: NOTE$1,
    emptyNote: NOTHING_YET$1,
    moves: [
      { seq: 41, at: "16:02:57", kind: "step", subject: "verify", moved: "running → advanced", actor: "fleet" },
      { seq: 42, at: "16:02:57", kind: "status", moved: "running → awaiting_review", actor: "fleet" }
    ]
  }
};
const NothingRecordedYet = {
  args: { moves: [], emptyNote: NOTHING_YET$1 }
};
const __vite_glob_0_22 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AJobThatEndedSomewhereSurprising,
  AJobThatRanClean,
  NothingRecordedYet,
  TwoMovesInOneInstant,
  default: meta$F
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
const meta$E = {
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
const APatchTooLongToDraw$1 = {
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
const __vite_glob_0_23 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  ADroneThatChangedNothing,
  AFileOutsideTheDeclaredPlan,
  AJobWithNoWorktree,
  APatchToDecideOn,
  APatchTooLongToDraw: APatchTooLongToDraw$1,
  default: meta$E
}, Symbol.toStringTag, { value: "Module" }));
const meta$D = {
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
const Running$2 = {
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
const Failed$1 = {
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
const __vite_glob_0_24 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AChecksPathsAreDrawnBeforeItRuns,
  AFailedCheckNamesItsOutput,
  AFlaggedStepIsNeverUngated,
  AVerdictBesideTheState,
  AWorkflowBeforeItRuns,
  EveryStepState,
  EvidenceFlaggedByTheGamingCheck,
  Failed: Failed$1,
  FourJobsOverAndOneResumable,
  HardPrerequisite,
  Killed: Killed$3,
  LabelsMissing,
  OneStep,
  OverruledByAPerson,
  Running: Running$2,
  RunningPulseElsewhere,
  ServedSteps,
  Stopped,
  TheSixCheckOutcomes,
  UngatedAndUnanswerable,
  WaitingAndRetrying,
  WhatAStepDeclares,
  default: meta$D
}, Symbol.toStringTag, { value: "Module" }));
function ErrorCode({ kind, code }) {
  return /* @__PURE__ */ jsx("span", { className: `armada-error-code armada-error-code--${kind}`, "data-error-class": kind, children: code });
}
const meta$C = {
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
const __vite_glob_0_25 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AgainstAStatusBadge,
  Degraded,
  Fault,
  default: meta$C
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
  const tail2 = [`bridge protocol ${payload.bridgeProtocol}`];
  if (payload.fleetProtocol !== void 0) tail2.push(`fleet protocol ${payload.fleetProtocol}`);
  tail2.push(`taken ${payload.at}`);
  blocks.push([tail2.join("  ")]);
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
const meta$B = {
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
const __vite_glob_0_26 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
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
  default: meta$B
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
const meta$A = {
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
const __vite_glob_0_27 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AnItemRemoved,
  AnItemThatCanBeRemoved,
  OneItemAndItCannotBeRemoved,
  TheControl,
  default: meta$A
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
const meta$z = {
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
const __vite_glob_0_28 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Escalated: Escalated$1,
  Neutral,
  default: meta$z
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
const meta$y = {
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
const __vite_glob_0_29 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Default: Default$7,
  LongFilename,
  ReadOnly,
  default: meta$y
}, Symbol.toStringTag, { value: "Module" }));
const meta$x = {
  title: "Primitives/Badge",
  component: Badge
};
const NO_GLYPH_IN_REGISTRY$2 = void 0;
const NotStarted = {
  args: { status: "not-started", icon: NO_GLYPH_IN_REGISTRY$2, children: "Not started" }
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
const Running$1 = {
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
  args: { status: "escalated", icon: NO_GLYPH_IN_REGISTRY$2, children: "Escalated" }
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
const __vite_glob_0_30 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
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
  Running: Running$1,
  RunningPulsing,
  Superseded,
  default: meta$x
}, Symbol.toStringTag, { value: "Module" }));
const meta$w = {
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
const __vite_glob_0_31 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
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
  default: meta$w
}, Symbol.toStringTag, { value: "Module" }));
const meta$v = {
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
const __vite_glob_0_32 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Default: Default$6,
  Dimmed: Dimmed$1,
  WithHeader,
  default: meta$v
}, Symbol.toStringTag, { value: "Module" }));
const meta$u = {
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
const __vite_glob_0_33 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Checked,
  Disabled: Disabled$6,
  Focused: Focused$6,
  Light: Light$6,
  Unchecked,
  default: meta$u
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
const meta$t = {
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
const __vite_glob_0_34 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AliasFindsTheLexiconTerm,
  DestructiveEntryConfirms,
  NoMatch,
  Resting: Resting$1,
  default: meta$t
}, Symbol.toStringTag, { value: "Module" }));
const meta$s = {
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
const __vite_glob_0_35 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Confirmation,
  KillTheDrone,
  KillTheJob,
  MoreThanFitsWithAFieldToReach,
  NeutralConfirm,
  RedispatchAsANewJob,
  RestartTheStep,
  default: meta$s
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
const meta$r = {
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
const __vite_glob_0_36 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AtTheLeftEdge: AtTheLeftEdge$2,
  AtTheRightEdge: AtTheRightEdge$2,
  RowMenu,
  WithNoRoomBelow: WithNoRoomBelow$2,
  WithSectionLabels,
  default: meta$r
}, Symbol.toStringTag, { value: "Module" }));
const meta$q = {
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
const __vite_glob_0_37 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Default: Default$5,
  Disabled: Disabled$5,
  Focused: Focused$5,
  Invalid: Invalid$2,
  Light: Light$5,
  Mono,
  Placeholder: Placeholder$1,
  default: meta$q
}, Symbol.toStringTag, { value: "Module" }));
const meta$p = {
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
const __vite_glob_0_38 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Chord,
  ContextualKeys,
  Default: Default$4,
  default: meta$p
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
const meta$o = {
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
const __vite_glob_0_39 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AlignedToTheEnd,
  AtTheLeftEdge: AtTheLeftEdge$1,
  AtTheRightEdge: AtTheRightEdge$1,
  Open: Open$1,
  WithNoRoomBelow: WithNoRoomBelow$1,
  default: meta$o
}, Symbol.toStringTag, { value: "Module" }));
const meta$n = {
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
const __vite_glob_0_40 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AFlagsCitation,
  AJudgesConsequence,
  AListAndAHeading,
  WhatItWillNotDraw,
  default: meta$n
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
const meta$m = {
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
const __vite_glob_0_41 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Default: Default$3,
  Disabled: Disabled$4,
  Focused: Focused$4,
  Light: Light$4,
  default: meta$m
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
const meta$l = {
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
const __vite_glob_0_42 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Scrolling,
  WithinBounds,
  default: meta$l
}, Symbol.toStringTag, { value: "Module" }));
const meta$k = {
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
const __vite_glob_0_43 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Default: Default$2,
  Disabled: Disabled$3,
  Focused: Focused$3,
  Invalid: Invalid$1,
  Light: Light$3,
  default: meta$k
}, Symbol.toStringTag, { value: "Module" }));
const meta$j = {
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
const __vite_glob_0_44 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Announced,
  Horizontal,
  Vertical,
  default: meta$j
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
const meta$i = {
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
const __vite_glob_0_45 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Left,
  Right,
  default: meta$i
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
const meta$h = {
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
const __vite_glob_0_46 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  InACard,
  Single,
  Text,
  default: meta$h
}, Symbol.toStringTag, { value: "Module" }));
const meta$g = {
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
const __vite_glob_0_47 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Closed,
  Disabled: Disabled$2,
  EscalatedRow,
  Focused: Focused$2,
  FocusedOnPrimary,
  Light: Light$2,
  Open,
  PrimaryOnJobDetail,
  default: meta$g
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
const meta$f = {
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
const __vite_glob_0_48 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Disabled: Disabled$1,
  Focused: Focused$1,
  Light: Light$1,
  Off,
  On,
  WithADescription,
  default: meta$f
}, Symbol.toStringTag, { value: "Module" }));
const meta$e = {
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
const __vite_glob_0_49 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Default: Default$1,
  Dimmed,
  FocusedAndSelected,
  MonoValuesCopy,
  RowsGrowWithContent,
  default: meta$e
}, Symbol.toStringTag, { value: "Module" }));
const meta$d = {
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
const __vite_glob_0_50 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  LastActive,
  SectionsOfOneObject,
  default: meta$d
}, Symbol.toStringTag, { value: "Module" }));
const meta$c = {
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
const __vite_glob_0_51 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Queues,
  Suspended,
  TheBoard: TheBoard$1,
  Zero,
  default: meta$c
}, Symbol.toStringTag, { value: "Module" }));
const meta$b = {
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
const BRIEF = "A burst of 401s should produce one refresh call, not one per request. Keep the retry ceiling where it is.";
const Default = {
  args: { label: "Brief", defaultValue: BRIEF },
  render: (args) => /* @__PURE__ */ jsx(Card, { children: /* @__PURE__ */ jsx(Textarea, { ...args }) })
};
const Placeholder = {
  args: { label: "Brief", placeholder: BRIEF },
  render: (args) => /* @__PURE__ */ jsx(Card, { children: /* @__PURE__ */ jsx(Textarea, { ...args }) })
};
const Focused = {
  render: () => /* @__PURE__ */ jsx(Card, { children: /* @__PURE__ */ jsx(Textarea, { label: "Brief", defaultValue: BRIEF, "data-preview-focus": "" }) })
};
const Invalid = {
  render: () => /* @__PURE__ */ jsx(Card, { children: /* @__PURE__ */ jsx(Textarea, { label: "Brief", invalid: true, message: "A job needs a brief. Write what the work is." }) })
};
const Disabled = {
  render: () => /* @__PURE__ */ jsx(Card, { children: /* @__PURE__ */ jsx(Textarea, { label: "Brief", defaultValue: BRIEF, disabled: true }) })
};
const Rows = {
  render: () => /* @__PURE__ */ jsx(Card, { children: /* @__PURE__ */ jsx(Textarea, { label: "Brief", rows: 6, defaultValue: BRIEF }) })
};
const Overflowing = {
  render: () => /* @__PURE__ */ jsx(Card, { children: /* @__PURE__ */ jsx(
    Textarea,
    {
      label: "Brief",
      defaultValue: `${BRIEF} The retry ceiling is three, set where the transport is configured rather than at the call site, and moving it is a separate job. What is in scope is the coalescing: one refresh in flight, every waiter parked on it.`
    }
  ) })
};
const Light = {
  render: () => /* @__PURE__ */ jsx("div", { "data-theme": "light", children: /* @__PURE__ */ jsx(Card, { children: /* @__PURE__ */ jsx(Textarea, { label: "Brief", defaultValue: BRIEF }) }) })
};
const __vite_glob_0_52 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Default,
  Disabled,
  Focused,
  Invalid,
  Light,
  Overflowing,
  Placeholder,
  Rows,
  default: meta$b
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
const meta$a = {
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
const __vite_glob_0_53 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Copied,
  Killed: Killed$1,
  Landed,
  default: meta$a
}, Symbol.toStringTag, { value: "Module" }));
const meta$9 = {
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
const __vite_glob_0_54 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AtTheLeftEdge,
  AtTheRightEdge,
  Resting,
  TruncatedValue,
  WithNoRoomBelow,
  WithShortcut,
  default: meta$9
}, Symbol.toStringTag, { value: "Module" }));
function Absent({ name, note }) {
  return /* @__PURE__ */ jsxs("div", { className: "armada-screen-absent", role: "note", children: [
    /* @__PURE__ */ jsx("span", { className: "armada-screen-absent__name", children: name }),
    /* @__PURE__ */ jsx("span", { className: "armada-screen-absent__why", children: note })
  ] });
}
function AFailedJobADeadEndReadAsOne({
  heading: heading2,
  why,
  whyAbsent = "The Job carries no stored reason, and none is written here.",
  recourse,
  steps: steps2,
  ranLabel = "What ran",
  stepsAbsent = "Nothing serves this Job's workflow, so its steps are unknown.",
  output,
  outputAbsent = "Nothing serves a check's output yet.",
  work: work2,
  workAbsent = "Nothing serves this Job's paths, its branch or its brief.",
  record: record2,
  recordValue,
  onRecordChange,
  recordAbsent,
  recordLabel = "What it left behind",
  onCopied
}) {
  return /* @__PURE__ */ jsxs("div", { className: "armada-screen__detail", children: [
    /* @__PURE__ */ jsx(JobDetailHeaderActions, { ...heading2, onCopied }),
    /* @__PURE__ */ jsxs("div", { className: "armada-screen__sunken", children: [
      /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", children: "Why this stopped" }),
      why === void 0 ? /* @__PURE__ */ jsx("div", { className: "armada-screen__slot", children: /* @__PURE__ */ jsx(Absent, { name: "Why this stopped", note: whyAbsent }) }) : /* @__PURE__ */ jsx("p", { className: "armada-screen__why", children: why }),
      recourse === void 0 ? null : /* @__PURE__ */ jsx("p", { className: "armada-screen__recourse", children: recourse })
    ] }),
    /* @__PURE__ */ jsxs("div", { className: "armada-screen__split", "data-wide": true, children: [
      /* @__PURE__ */ jsxs("div", { className: "armada-screen__col", children: [
        /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", children: ranLabel }),
        steps2.length === 0 ? /* @__PURE__ */ jsx("div", { className: "armada-screen__slot", children: /* @__PURE__ */ jsx(Absent, { name: "What ran", note: stepsAbsent }) }) : /* @__PURE__ */ jsx(WorkflowRail, { steps: steps2, onCopied })
      ] }),
      /* @__PURE__ */ jsxs("div", { className: "armada-screen__col", "data-loose": true, children: [
        /* @__PURE__ */ jsxs("div", { className: "armada-screen__col", children: [
          /* @__PURE__ */ jsxs("div", { className: "armada-screen__head-row", children: [
            /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", children: "Check output" }),
            output?.meta ? /* @__PURE__ */ jsx("span", { className: "armada-screen__tag", children: output.meta }) : null
          ] }),
          output === void 0 ? /* @__PURE__ */ jsx("div", { className: "armada-screen__slot", children: /* @__PURE__ */ jsx(Absent, { name: "Check output", note: outputAbsent }) }) : /* @__PURE__ */ jsx("pre", { className: "armada-screen__output", children: output.tail })
        ] }),
        /* @__PURE__ */ jsxs("div", { className: "armada-screen__col", children: [
          /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", children: "Where the work is" }),
          work2 === void 0 ? /* @__PURE__ */ jsx("div", { className: "armada-screen__slot", children: /* @__PURE__ */ jsx(Absent, { name: "Where the work is", note: workAbsent }) }) : /* @__PURE__ */ jsxs(Fragment, { children: [
            work2.brief === void 0 ? null : /* @__PURE__ */ jsx(JobBrief, { ...work2.brief }),
            /* @__PURE__ */ jsx(JobLogReference, { rows: work2.rows, onCopied, children: work2.note }),
            work2.actions ? /* @__PURE__ */ jsx("div", { className: "armada-screen__actions", children: work2.actions }) : null
          ] })
        ] })
      ] })
    ] }),
    record2 === void 0 ? null : /* @__PURE__ */ jsxs("div", { className: "armada-screen__col", children: [
      /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", children: recordLabel }),
      /* @__PURE__ */ jsx(
        JobRecord,
        {
          sections: record2,
          value: recordValue,
          onChange: onRecordChange,
          emptyNote: recordAbsent
        }
      )
    ] })
  ] });
}
const NO_GLYPH_IN_REGISTRY$1 = void 0;
const steps$1 = [
  {
    id: "plan",
    label: "Plan the change",
    activity: "advanced",
    status: "advanced",
    // The drawing draws no row under Plan the change here. The rail always
    // draws one. Reported.
    evidence: { icon: NO_GLYPH_IN_REGISTRY$1, iconLabel: "Evidence", label: "" }
  },
  {
    id: "implement",
    label: "Implement",
    activity: "advanced",
    status: "advanced",
    gates: [
      {
        command: "build · cargo build --workspace",
        result: "exit 0",
        icon: ShieldCheck,
        iconLabel: "Passed"
      },
      // The drawing draws `shield-minus` on this row, whose registry entry
      // means "not reached", beside the result "passed". A glyph is never
      // written by hand against the registry, so the row takes `shield-check`.
      // Reported as a slip in the drawing.
      {
        command: "diff_nonempty",
        result: "passed",
        icon: ShieldCheck,
        iconLabel: "Passed"
      }
    ]
  },
  {
    id: "verify",
    label: "Run tests",
    activity: "failed",
    status: "failed a check",
    gates: [
      {
        command: "test · cargo test --workspace",
        result: "exit 1",
        icon: ShieldX,
        iconLabel: "Failed"
      }
    ]
  },
  {
    id: "handoff",
    label: "Summarise",
    activity: "not_started",
    status: "not started",
    evidence: { icon: NO_GLYPH_IN_REGISTRY$1, iconLabel: "Evidence", label: "" }
  }
];
const tail = [
  "running 84 tests",
  "test manifest::cache::reads_once ... FAILED",
  "test manifest::cache::invalidates_on_write ... FAILED",
  "",
  "failures:",
  "",
  "---- manifest::cache::reads_once stdout ----",
  "assertion `left == right` failed",
  "  left: 2",
  " right: 1",
  "   at core/manifest/src/cache.rs:214",
  "",
  "test result: FAILED. 82 passed; 2 failed"
].join("\n");
const record$1 = [
  {
    id: "moves",
    label: "Every move it made",
    panel: /* @__PURE__ */ jsx(
      TransitionHistory,
      {
        emptyNote: "This job has no recorded moves.",
        note: "Every move Fleet recorded for this job, oldest first.",
        moves: [
          { seq: 1, at: "14:02:11", kind: "created", moved: "queued", actor: "you" },
          { seq: 2, at: "14:02:12", kind: "approved", moved: "queued → running", actor: "you" },
          {
            seq: 3,
            at: "14:19:40",
            kind: "step_advanced",
            subject: "implement",
            moved: "implement → verify",
            actor: "fleet"
          },
          {
            seq: 4,
            at: "14:24:52",
            kind: "escalated",
            subject: "verify",
            moved: "running → escalated",
            why: "gate_failure",
            actor: "fleet"
          }
        ]
      }
    )
  },
  {
    id: "turns",
    label: "The drone's turns",
    panel: /* @__PURE__ */ jsx(
      DroneTurns,
      {
        emptyNote: "This job has no turns.",
        turns: [
          {
            id: "t1",
            at: "14:19:44",
            kind: "tool_use",
            subject: "Edit",
            detail: "core/manifest/src/cache.rs"
          },
          {
            id: "t2",
            at: "14:22:03",
            kind: "tool_use",
            subject: "Bash",
            detail: "cargo test --workspace"
          },
          {
            id: "t3",
            at: "14:24:48",
            kind: "assistant",
            said: "Two cache tests fail against the new key. Submitting anyway to get a verdict."
          }
        ]
      }
    )
  },
  {
    id: "files",
    label: "Files changed",
    panel: /* @__PURE__ */ jsx(
      ChangedFiles,
      {
        emptyNote: "This job's worktree was read when it stopped and held no change against the branch it was cut from.",
        note: "Read from this job's worktree when the job stopped, and kept — so it says the same thing whether or not anyone was watching. Measured against what plan, implement declared. 2 of 4 paths are outside all of them.",
        files: [
          { path: "core/manifest/src/cache.rs", change: "modified" },
          { path: "core/manifest/src/cache_tests.rs", change: "added" },
          { path: "core/manifest/src/lib.rs", change: "modified", outsidePlan: true },
          { path: "scripts/dev", change: "modified", outsidePlan: true }
        ]
      }
    )
  },
  {
    id: "changed",
    label: "What it changed",
    panel: /* @__PURE__ */ jsx(
      UnifiedDiff,
      {
        emptyNote: "This job's worktree holds no change against the branch it was cut from.",
        note: "Read from this job's worktree against the branch it was cut from. The plan this step declared is not readable once its drone has stopped, so no file is marked here. Files changed is the record kept when the job stopped, and it marks every path that fell outside the plans the steps declared.",
        files: [
          {
            path: "core/manifest/src/cache.rs",
            lines: [
              { kind: "hunk", text: "@@ -18,7 +18,9 @@ impl Cache {" },
              { kind: "context", text: "     pub fn read(&self, path: &Path) -> Manifest {" },
              { kind: "removed", text: "-        self.load(path)" },
              { kind: "added", text: "+        let key = path.canonicalize().unwrap_or_else(|_| path.into());" },
              { kind: "added", text: "+        self.entries.entry(key).or_insert_with(|| self.load(path)).clone()" },
              { kind: "context", text: "     }" }
            ]
          }
        ]
      }
    )
  },
  {
    id: "claims",
    label: "What the drone claimed",
    panel: /* @__PURE__ */ jsx(
      EvidenceTrail,
      {
        entries: [
          {
            step: "Implement",
            provenance: "self_reported",
            icon: FileCheck,
            iconLabel: "Evidence",
            claimed: "The manifest is read once per dispatch and cached on the absolute path.",
            shownBy: "core/manifest/src/cache.rs, and the two tests beside it",
            notClaimed: "Nothing about a manifest that changes while a job is running."
          }
        ]
      }
    )
  }
];
const heading$4 = {
  status: "completed-failed",
  statusIcon: X,
  statusLabel: "Failed",
  headline: "Cache the manifest read",
  jobId: "job_91ab",
  fields: [
    // A step name is a label, so it stays sans beside its mono siblings, and
    // the two halves are one fact joined by a comma.
    { label: "Stopped at", value: "Run tests" },
    { label: "step", value: "3 of 4", mono: true, continues: true },
    { label: "Ran", value: "22m 41s", mono: true },
    { label: "Spend, estimated", value: "~$2.10", mono: true },
    { label: "Dispatched by you" }
  ]
};
const meta$8 = {
  title: "Screens/A failed job — a dead end, read as one",
  component: AFailedJobADeadEndReadAsOne
};
const FailedJob = {
  render: function FailedJobStory() {
    const [section, setSection] = useState("moves");
    return /* @__PURE__ */ jsx("div", { className: "armada-screen", children: /* @__PURE__ */ jsx(
      AFailedJobADeadEndReadAsOne,
      {
        heading: heading$4,
        why: /* @__PURE__ */ jsxs(Fragment, { children: [
          "The test check exited 1 at Run tests, on 2 assertions in",
          " ",
          /* @__PURE__ */ jsx("span", { className: "armada-screen__mono", children: "core/manifest" }),
          ". The job is over. Nothing runs from here without you."
        ] }),
        recourse: "Nothing resumes this job. Redirect and restart both take a job a person is holding, which is an escalated one, and this job is failed. A redispatch mints a new job from the approval gate and carries none of the work over.",
        record: record$1,
        recordValue: section,
        onRecordChange: setSection,
        steps: steps$1,
        output: { tail, meta: "exit 1 · 4.2s · tail 12 lines" },
        work: {
          brief: {
            criteria: [
              { text: "The manifest is read once per dispatch, not once per step.", source: "check" },
              { text: "A changed armada.yml is picked up without restarting Fleet.", source: "judge" }
            ],
            facts: "`config::manifest` is the only reader. The cache key is the absolute path."
          },
          rows: [
            {
              icon: GitBranch,
              iconLabel: "Branch",
              value: "feat/manifest-cache",
              copyValue: "feat/manifest-cache",
              meta: "2 files +48 −11"
            },
            // `folder` means "workspace" in the registry. A worktree is not a
            // workspace, and the registry has no row for one. Reported.
            {
              icon: Folder,
              iconLabel: "Worktree",
              value: "/repos/armada/.armada/worktrees/job_91ab",
              copyValue: "/repos/armada/.armada/worktrees/job_91ab"
            },
            {
              icon: File,
              iconLabel: "Log",
              value: "/repos/armada/.armada/logs/job_91ab.jsonl",
              copyValue: "/repos/armada/.armada/logs/job_91ab.jsonl",
              separated: true
            },
            // No registered glyph means a transcript, so the mark keeps its
            // column and renders empty rather than borrowing one. Reported.
            {
              iconLabel: "Transcript",
              value: "/repos/armada/.armada/transcripts/",
              copyValue: "/repos/armada/.armada/transcripts/",
              meta: "named by a drone id nothing serves"
            }
          ],
          note: "The worktree and the branch are left in place. Armada will not touch either. The log holds Fleet, the drone and Bridge in one order, keyed on this job.",
          actions: /* @__PURE__ */ jsxs(Fragment, { children: [
            /* @__PURE__ */ jsx(Button, { children: "Open the log" }),
            /* @__PURE__ */ jsx(Button, { children: "Open the worktree" })
          ] })
        }
      }
    ) });
  }
};
const StoppedAndAsked = {
  render: () => {
    const [open2, setOpen] = useState(false);
    const [instruction, setInstruction] = useState("");
    return /* @__PURE__ */ jsxs("div", { className: "armada-screen", children: [
      /* @__PURE__ */ jsx(
        AFailedJobADeadEndReadAsOne,
        {
          heading: {
            status: "escalated",
            statusIcon: OctagonAlert,
            statusLabel: "stalled",
            headline: "Cache the manifest read",
            jobId: "job_91ab",
            fields: [
              { label: "Stopped at", value: "verify" },
              { label: "step", value: "3 of 4", mono: true, continues: true },
              { label: "Model", value: "sonnet", mono: true }
            ],
            actions: /* @__PURE__ */ jsxs(Fragment, { children: [
              /* @__PURE__ */ jsx(Button, { variant: "secondary", onClick: () => setOpen(true), children: "Redirect drone" }),
              /* @__PURE__ */ jsx(
                SplitButton,
                {
                  variant: "destructive",
                  menuLabel: "What else ends this job",
                  items: [{ label: "Kill drone, the job stays open" }],
                  children: "Redispatch as a new job"
                }
              )
            ] })
          },
          why: "The job stalled. Nothing runs from here without you.",
          recourse: "Redirect the drone. Its session, its worktree and every step so far are still held, so an instruction reaches it as a new turn at the step above. Fleet refuses a restart while a drone is alive, because a restart throws that session away. A redispatch mints a new job from the approval gate and carries none of the work over.",
          steps: steps$1.map((step) => ({
            id: step.id,
            label: step.id,
            labelIsAnIdentifier: true,
            activity: step.activity,
            ungatedLabel: "Fleet serves no check result for this step",
            evidence: { label: "" }
          }))
        }
      ),
      /* @__PURE__ */ jsxs(
        Dialog,
        {
          open: open2,
          tone: "neutral",
          title: "Redirect the drone on this job?",
          confirmLabel: "Redirect drone",
          confirmDisabled: instruction.trim() === "",
          onCancel: () => setOpen(false),
          onConfirm: () => setOpen(false),
          children: [
            /* @__PURE__ */ jsx("p", { children: "The instruction is sent to the drone as a new turn. The job stays at the same step, with the same session — nothing is spawned and nothing already done is thrown away." }),
            /* @__PURE__ */ jsx(
              Textarea,
              {
                label: "Instruction",
                rows: 4,
                value: instruction,
                onChange: (event) => setInstruction(event.target.value)
              }
            )
          ]
        }
      )
    ] });
  }
};
const StoppedWithNoDrone = {
  render: () => {
    const [confirming, setConfirming] = useState(false);
    return /* @__PURE__ */ jsxs("div", { className: "armada-screen", children: [
      /* @__PURE__ */ jsx(
        AFailedJobADeadEndReadAsOne,
        {
          heading: {
            status: "escalated",
            statusIcon: OctagonAlert,
            statusLabel: "stalled",
            headline: "Cache the manifest read",
            jobId: "job_91ab",
            fields: [
              { label: "Stopped at", value: "verify" },
              { label: "step", value: "3 of 4", mono: true, continues: true },
              { label: "Model", value: "sonnet", mono: true }
            ],
            actions: /* @__PURE__ */ jsxs(Fragment, { children: [
              /* @__PURE__ */ jsx(Button, { variant: "secondary", onClick: () => setConfirming(true), children: "Restart step" }),
              /* @__PURE__ */ jsx(
                SplitButton,
                {
                  variant: "destructive",
                  menuLabel: "What else ends this job",
                  items: [{ label: "Kill job, it ends here", danger: true }],
                  children: "Redispatch as a new job"
                }
              )
            ] })
          },
          why: "The job stalled. Its drone is gone. Nothing runs from here without you.",
          recourse: "Restart the step. The drone is gone, so a fresh one takes over the worktree at the step above, resolving its toolset, model and environment again. Fleet read the worktree before offering this, so it is there to take over. A redispatch mints a new job from the approval gate and carries none of the work over.",
          steps: steps$1.map((step) => ({
            id: step.id,
            label: step.id,
            labelIsAnIdentifier: true,
            activity: step.activity,
            ungatedLabel: "Fleet serves no check result for this step",
            evidence: { label: "" }
          }))
        }
      ),
      /* @__PURE__ */ jsx(
        Dialog,
        {
          open: confirming,
          tone: "neutral",
          title: "Restart this step?",
          confirmLabel: "Restart step",
          onCancel: () => setConfirming(false),
          onConfirm: () => setConfirming(false),
          children: "A fresh drone takes over on the same worktree, at the step the last one stopped at. The toolset, model and environment are resolved again from scratch, so a widened scope can only narrow. Fleet read the worktree before offering this, so there is one to take over."
        }
      )
    ] });
  }
};
const AJudgeRefusedACriterion = {
  render: () => /* @__PURE__ */ jsx("div", { className: "armada-screen", children: /* @__PURE__ */ jsx(
    AFailedJobADeadEndReadAsOne,
    {
      heading: {
        status: "escalated",
        statusIcon: ShieldX,
        statusLabel: "failed a check",
        headline: "Sign a revoked device out on refresh failure",
        jobId: "job_2d90bb",
        fields: [
          { label: "Stopped at", value: "Implement" },
          { label: "step", value: "2 of 4", mono: true, continues: true },
          { label: "Elapsed", value: "11m 03s", mono: true },
          { label: "Model", value: "sonnet", mono: true }
        ],
        actions: /* @__PURE__ */ jsxs(Fragment, { children: [
          /* @__PURE__ */ jsx(Button, { variant: "secondary", children: "Restart step" }),
          /* @__PURE__ */ jsx(Button, { children: "Redispatch as a new job" })
        ] })
      },
      why: "failed a check · owes c2",
      recourse: "Restart the step. The drone is gone, so a fresh one takes over the worktree at the step above, resolving its toolset, model and environment again. Fleet read the worktree before offering this, so it is there to take over. A redispatch mints a new job from the approval gate and carries none of the work over.",
      ranLabel: "What ran",
      steps: [
        { id: "plan", label: "Plan the change", activity: "advanced", status: "advanced" },
        {
          id: "implement",
          label: "Implement",
          activity: "stopped",
          status: "stopped",
          current: true,
          gates: [
            {
              command: "build · cargo build --workspace",
              result: "passed",
              icon: ShieldCheck,
              iconLabel: "Passed",
              outputPath: ".armada/jobs/job_2d90bb/checks/build.log"
            }
          ],
          verdicts: [
            {
              ordinal: 1,
              criterionId: "c1",
              text: "Expired tokens refresh once rather than per request.",
              named: "met",
              verdict: "no objection",
              icon: CircleCheck
            },
            {
              ordinal: 2,
              criterionId: "c2",
              text: "A failed refresh signs the session out.",
              named: "not_met",
              verdict: "refused",
              icon: CircleX,
              expected: "A 401 from the refresh endpoint clears the session and returns the caller to sign-in.",
              produced: "The refresh error is swallowed in `session.ts:212` and the stale token is retried on the next request.",
              consequence: "A revoked device keeps a working-looking session until the next full reload, so signing a device out does not sign it out."
            }
          ]
        },
        { id: "verify", label: "Run tests", activity: "not_started", status: "not started" },
        { id: "handoff", label: "Summarise", activity: "not_started", status: "not started" }
      ],
      outputAbsent: "Each check names its output file on its own row. Nothing serves the contents."
    }
  ) })
};
const KilledWhileTheStepWasRunning = {
  render: () => /* @__PURE__ */ jsx("div", { className: "armada-screen", children: /* @__PURE__ */ jsx(
    AFailedJobADeadEndReadAsOne,
    {
      heading: {
        status: "killed",
        statusIcon: Power,
        statusLabel: "killed",
        headline: "Cache the manifest read",
        jobId: "job_91ab",
        fields: [
          { label: "Step", value: "3 of 4", mono: true },
          { label: "at", value: "verify", mono: true, continues: true },
          { label: "Elapsed", value: "22m 41s", mono: true },
          { label: "Model", value: "sonnet", mono: true }
        ]
      },
      why: /* @__PURE__ */ jsx(Fragment, { children: "stopped at Run tests" }),
      recourse: "Nothing resumes this job. Redirect and restart both take a job a person is holding, which is an escalated one, and this job is killed. A redispatch mints a new job from the approval gate and carries none of the work over.",
      steps: [
        { id: "plan", label: "Plan the change", activity: "advanced", status: "advanced", elapsed: "2m 14s" },
        { id: "implement", label: "Implement", activity: "advanced", status: "advanced", elapsed: "6m 48s" },
        {
          id: "verify",
          label: "Run tests",
          // `running` on the wire, `killed` on the rail. The Job's status is
          // read, not a state Fleet does not have.
          activity: "killed",
          status: "killed",
          current: true,
          elapsed: "4m 09s",
          gates: [
            {
              command: "test · cargo test --workspace",
              result: "not reached",
              icon: ShieldMinus,
              iconLabel: "Not reached"
            }
          ]
        },
        { id: "handoff", label: "Summarise", activity: "not_started", status: "not_started" }
      ],
      outputAbsent: "Each check names its output file on its own row. Nothing serves the contents.",
      workAbsent: "Nothing serves this Job's paths, its branch or its brief."
    }
  ) })
};
const EscalatedWithNoStepToResume = {
  render: () => /* @__PURE__ */ jsx("div", { className: "armada-screen", children: /* @__PURE__ */ jsx(
    AFailedJobADeadEndReadAsOne,
    {
      heading: {
        status: "escalated",
        statusIcon: Unplug,
        statusLabel: "interrupted",
        headline: "Cache the manifest read",
        jobId: "job_91ab",
        fields: [
          { label: "Ran", value: "3m 12s", mono: true },
          { label: "Model", value: "sonnet", mono: true }
        ],
        actions: /* @__PURE__ */ jsx(Button, { children: "Redispatch as a new job" })
      },
      why: "interrupted",
      recourse: "Nothing resumes this job. It escalated without stopping a step, so redirect and restart have no step to land on. A redispatch mints a new job from the approval gate and carries none of the work over.",
      steps: steps$1.map((step) => ({
        id: step.id,
        label: step.label,
        activity: "not_started",
        status: "not started"
      })),
      outputAbsent: "Each check names its output file on its own row. Nothing serves the contents.",
      workAbsent: "Nothing serves this Job's paths, its branch or its brief."
    }
  ) })
};
const __vite_glob_0_55 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AJudgeRefusedACriterion,
  EscalatedWithNoStepToResume,
  FailedJob,
  KilledWhileTheStepWasRunning,
  StoppedAndAsked,
  StoppedWithNoDrone,
  default: meta$8
}, Symbol.toStringTag, { value: "Module" }));
function AFinishedJobWhatItWasAndWhatItProduced({
  heading: heading2,
  brief: brief2,
  briefAbsent = "Nothing serves this Job's acceptance criteria.",
  outcome,
  outcomeAbsent = "Nothing serves a branch or a worktree yet.",
  record: record2 = [],
  recordValue,
  onRecordChange,
  recordAbsent,
  wasLabel = "What this was",
  producedLabel = "What came out",
  recordLabel = "The record",
  onCopied
}) {
  return /* @__PURE__ */ jsxs("div", { className: "armada-screen__detail", children: [
    /* @__PURE__ */ jsx(JobDetailHeaderActions, { ...heading2, onCopied }),
    /* @__PURE__ */ jsxs("div", { className: "armada-screen__col", children: [
      /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", children: wasLabel }),
      brief2 === void 0 ? /* @__PURE__ */ jsx("div", { className: "armada-screen__slot", children: /* @__PURE__ */ jsx(Absent, { name: "What this was", note: briefAbsent }) }) : /* @__PURE__ */ jsx(JobBrief, { ...brief2, only: "criteria" })
    ] }),
    /* @__PURE__ */ jsxs("div", { className: "armada-screen__col", children: [
      /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", children: producedLabel }),
      outcome === void 0 ? /* @__PURE__ */ jsx("div", { className: "armada-screen__slot", children: /* @__PURE__ */ jsx(Absent, { name: "What came out", note: outcomeAbsent }) }) : /* @__PURE__ */ jsx(JobOutcome, { ...outcome, onCopied })
    ] }),
    /* @__PURE__ */ jsxs("div", { className: "armada-screen__col", children: [
      /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", children: recordLabel }),
      /* @__PURE__ */ jsx(
        JobRecord,
        {
          sections: record2,
          value: recordValue,
          onChange: onRecordChange,
          emptyNote: recordAbsent
        }
      )
    ] })
  ] });
}
const meta$7 = {
  title: "Screens/A finished job — what it was and what it produced",
  component: AFinishedJobWhatItWasAndWhatItProduced
};
const heading$3 = {
  status: "completed-success",
  statusIcon: Check,
  statusLabel: "Done",
  headline: "Add a retry ceiling to the poke loop",
  jobId: "job_4f10",
  fields: [
    // The fact reads as a sentence around its value, which is what `suffix` is
    // for: `All 4 of 4 steps advanced`.
    { label: "All", value: "4 of 4", mono: true, suffix: "steps advanced" },
    { label: "Ran", value: "18m 22s", mono: true },
    { label: "Spend, estimated", value: "~$2.40", mono: true },
    { label: "Dispatched by you" }
  ]
};
const brief$1 = {
  criteria: [
    { text: "The poke loop stops after the configured number of attempts.", source: "check" },
    { text: "A ceiling of zero is refused at load rather than at run.", source: "check" }
  ]
};
const NOTE = "The branch is pushed and a review is open. Armada has no merge action — read the diff in your own tools and land it yourself.";
const paths = /* @__PURE__ */ jsx(
  JobLogReference,
  {
    rows: [
      {
        icon: Folder,
        iconLabel: "Worktree",
        value: "/repos/armada/.armada/worktrees/job_4f10",
        copyValue: "/repos/armada/.armada/worktrees/job_4f10"
      },
      {
        icon: File,
        iconLabel: "Log",
        value: "/repos/armada/.armada/logs/job_4f10.jsonl",
        copyValue: "/repos/armada/.armada/logs/job_4f10.jsonl",
        separated: true
      },
      {
        iconLabel: "Transcript",
        value: "/repos/armada/.armada/transcripts/",
        copyValue: "/repos/armada/.armada/transcripts/",
        meta: "named by a drone id nothing serves"
      }
    ],
    children: "The worktree, the log and the transcripts directory follow from the job id and the repository its manifest was read from."
  }
);
const record = [
  { id: "steps", label: "Steps and checks", panel: /* @__PURE__ */ jsx(Stub, { children: "The workflow rail goes here." }) },
  { id: "turns", label: "The drone's turns", panel: /* @__PURE__ */ jsx(Stub, { children: "The transcript goes here." }) },
  {
    id: "told",
    label: "What it was told",
    panel: /* @__PURE__ */ jsx(
      JobBrief,
      {
        criteria: [],
        only: "facts",
        facts: "The loop is in `fleet::poke`. The ceiling is a Machine setting, not a Kit one."
      }
    )
  },
  { id: "paths", label: "Where the work is", panel: paths }
];
const AsBridgeDrawsItToday$1 = {
  render: () => /* @__PURE__ */ jsx("div", { className: "armada-screen", children: /* @__PURE__ */ jsx(
    AFinishedJobWhatItWasAndWhatItProduced,
    {
      heading: heading$3,
      brief: brief$1,
      outcome: {
        note: NOTE,
        parts: [
          {
            name: "Branch",
            icon: GitBranch,
            iconLabel: "Branch",
            value: "fix/poke-ceiling"
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
            /* No glyph: `file` is reserved to the log row and `file-check` to
               a submission that landed, so a changed-file row has nothing in
               the registry to take. The mark column stays and renders empty. */
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
      },
      record,
      recordValue: "steps"
    }
  ) })
};
const EveryPartServed = {
  render: () => /* @__PURE__ */ jsx("div", { className: "armada-screen", children: /* @__PURE__ */ jsx(
    AFinishedJobWhatItWasAndWhatItProduced,
    {
      heading: heading$3,
      brief: brief$1,
      outcome: {
        note: NOTE,
        parts: [
          {
            name: "Branch",
            icon: GitBranch,
            iconLabel: "Branch",
            value: "fix/poke-ceiling",
            meta: "from main"
          },
          {
            name: "Commit",
            icon: GitCommitHorizontal,
            iconLabel: "Commit",
            value: "9f2c1ab"
          },
          {
            name: "Pull request",
            icon: GitPullRequest,
            iconLabel: "Pull request",
            value: "armada#42"
          },
          { name: "Files changed", value: "3 files", meta: "+214 −96" },
          {
            name: "Evidence",
            icon: FileCheck,
            iconLabel: "Evidence",
            value: "4 submissions"
          }
        ]
      },
      record,
      recordValue: "told"
    }
  ) })
};
const BeforeTheDetailArrives = {
  render: () => /* @__PURE__ */ jsx("div", { className: "armada-screen", children: /* @__PURE__ */ jsx(
    AFinishedJobWhatItWasAndWhatItProduced,
    {
      heading: {
        status: "completed-success",
        statusIcon: Check,
        statusLabel: "done",
        headline: "Add a retry ceiling to the poke loop",
        jobId: "job_4f10",
        fields: [
          { label: "All", value: "4 of 4", mono: true, suffix: "steps advanced" },
          { label: "Model", value: "sonnet", mono: true }
        ]
      },
      briefAbsent: "Reading this job.",
      outcomeAbsent: "Reading this job.",
      recordAbsent: "Reading this job, so there is no record to fold yet."
    }
  ) })
};
function Stub({ children }) {
  return /* @__PURE__ */ jsx("p", { className: "armada-record__note", children });
}
const __vite_glob_0_56 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AsBridgeDrawsItToday: AsBridgeDrawsItToday$1,
  BeforeTheDetailArrives,
  EveryPartServed,
  default: meta$7
}, Symbol.toStringTag, { value: "Module" }));
function AJobAwaitingReviewTheDiffAndTheReplyAreOneLoop({
  heading: heading2,
  brief: brief2,
  briefAbsent = "Nothing serves this job's acceptance criteria.",
  claims: claims2,
  claimsAbsent = "Nothing serves this job's work submissions.",
  diff: diff2,
  diffAbsent = "Nothing serves this job's diff.",
  decision: decision2,
  decisionAbsent = "Fleet did not answer for this job, so there is nothing to decide on.",
  work: work2,
  workAbsent = "Nothing serves this job's paths or its branch.",
  briefLabel = "What done meant",
  claimsLabel = "What the drone claimed",
  diffLabel = "What it changed",
  decisionLabel = "Your decision",
  workLabel = "Where the work is",
  onCopied
}) {
  return /* @__PURE__ */ jsxs("div", { className: "armada-screen__detail", children: [
    /* @__PURE__ */ jsx(JobDetailHeaderActions, { ...heading2, onCopied }),
    /* @__PURE__ */ jsxs("div", { className: "armada-screen__col", children: [
      /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", children: briefLabel }),
      brief2 === void 0 ? /* @__PURE__ */ jsx("div", { className: "armada-screen__slot", children: /* @__PURE__ */ jsx(Absent, { name: "What done meant", note: briefAbsent }) }) : /* @__PURE__ */ jsx(JobBrief, { ...brief2, only: "criteria" })
    ] }),
    /* @__PURE__ */ jsxs("div", { className: "armada-screen__col", children: [
      /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", "data-spaced": true, children: claimsLabel }),
      claims2 === void 0 ? /* @__PURE__ */ jsx("div", { className: "armada-screen__slot", children: /* @__PURE__ */ jsx(Absent, { name: "What the drone claimed", note: claimsAbsent }) }) : /* @__PURE__ */ jsx(EvidenceTrail, { ...claims2 })
    ] }),
    /* @__PURE__ */ jsxs("div", { className: "armada-screen__col", children: [
      /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", "data-spaced": true, children: diffLabel }),
      diff2 === void 0 ? /* @__PURE__ */ jsx("div", { className: "armada-screen__slot", children: /* @__PURE__ */ jsx(Absent, { name: "What it changed", note: diffAbsent }) }) : /* @__PURE__ */ jsx(UnifiedDiff, { ...diff2, onCopied })
    ] }),
    /* @__PURE__ */ jsxs("div", { className: "armada-screen__col", children: [
      /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", "data-spaced": true, children: decisionLabel }),
      decision2 === void 0 ? /* @__PURE__ */ jsx("div", { className: "armada-screen__slot", children: /* @__PURE__ */ jsx(Absent, { name: "Your decision", note: decisionAbsent }) }) : /* @__PURE__ */ jsx(ReviewDecision, { ...decision2 })
    ] }),
    /* @__PURE__ */ jsxs("div", { className: "armada-screen__col", children: [
      /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", "data-spaced": true, children: workLabel }),
      work2 === void 0 ? /* @__PURE__ */ jsx("div", { className: "armada-screen__slot", children: /* @__PURE__ */ jsx(Absent, { name: "Where the work is", note: workAbsent }) }) : /* @__PURE__ */ jsx("div", { className: "armada-screen__col", children: /* @__PURE__ */ jsx(JobLogReference, { rows: work2.rows, actions: work2.actions, onCopied, children: work2.note }) })
    ] })
  ] });
}
const meta$6 = {
  title: "Screens/A job awaiting review — the diff and the reply are one loop",
  component: AJobAwaitingReviewTheDiffAndTheReplyAreOneLoop
};
const heading$2 = {
  status: "awaiting-review",
  statusIcon: Eye,
  statusLabel: "Awaiting review",
  headline: "Honour human_always on a workflow's advance gate",
  jobId: "job_7c22",
  fields: [
    { label: "At", value: "4 of 4", mono: true, suffix: "steps, waiting on you" },
    { label: "Ran", value: "22m 06s", mono: true },
    { label: "Drone", value: "drn_01M4K", mono: true, suffix: "held" },
    { label: "Dispatched by you" }
  ]
};
const brief = {
  criteria: [
    { text: "A workflow declaring human_always loads rather than being refused.", source: "check" },
    { text: "A job on such a step stops at awaiting_review.", source: "check" },
    { text: "All three review acts refuse anywhere else.", source: "judge" }
  ]
};
const claims = {
  entries: [
    {
      step: "Widen the gate",
      provenance: "09:41:02 · code_change · manifest_check: cargo fmt, cargo clippy",
      icon: FileCheck,
      iconLabel: "Evidence",
      claimed: "AdvanceGate carries a HumanAlways variant and gate.rs returns Wait on it.",
      shownBy: "crates/core-model/src/workflow/gate.rs, crates/fleet/src/gate.rs",
      notClaimed: "Nothing loads a workflow declaring it yet — that is the next step."
    },
    {
      step: "Carry it through config",
      provenance: "10:03:19 · code_change · manifest_check: cargo test",
      icon: FileCheck,
      iconLabel: "Evidence",
      claimed: "A workflow declaring advance_gate: human_always loads instead of raising Fault::OutsideM1.",
      shownBy: "cargo test -p armada-config gate_ -- 6 passed",
      notClaimed: ""
    }
  ]
};
const diff = {
  emptyNote: "",
  note: "Read from this job's worktree against the branch it was cut from. Every path is inside the plan this step declared.",
  files: [
    {
      path: "crates/config/src/workflow.rs",
      lines: [
        { kind: "hunk", text: "@@ -441,7 +441,7 @@ fn gate_of(named: &str) -> Result<AdvanceGate, Fault> {" },
        { kind: "context", text: '         "auto" => Ok(AdvanceGate::Auto),' },
        { kind: "removed", text: '-        "human_always" => Err(Fault::OutsideM1("human_always")),' },
        { kind: "added", text: '+        "human_always" => Ok(AdvanceGate::HumanAlways),' },
        { kind: "context", text: "         other => Err(Fault::UnknownGate(other.into()))," }
      ]
    },
    {
      path: "crates/fleet/src/gate.rs",
      lines: [
        { kind: "hunk", text: "@@ -41,4 +41,6 @@ impl Gate {" },
        { kind: "context", text: "         match step.advance_gate() {" },
        { kind: "context", text: "             AdvanceGate::Auto => Advance::Now," },
        { kind: "added", text: "+            AdvanceGate::HumanAlways => Advance::Wait," },
        { kind: "context", text: "         }" }
      ]
    }
  ]
};
const work = {
  rows: [
    {
      icon: Folder,
      iconLabel: "Worktree",
      value: "/repos/armada/.armada/worktrees/job_7c22",
      copyValue: "/repos/armada/.armada/worktrees/job_7c22"
    },
    {
      icon: GitBranch,
      iconLabel: "Branch",
      value: "armada/job_7c22",
      copyValue: "armada/job_7c22"
    }
  ],
  note: "The worktree follows from this job's id and the repository its manifest was read from. The branch is served."
};
const decision = {
  note: "",
  onNote: () => {
  },
  onApprove: () => {
  },
  onRequestChanges: () => {
  },
  onReject: () => {
  }
};
const WorkWaitingOnADecision = {
  args: { heading: heading$2, brief, claims, diff, decision, work }
};
const ChangesBeingWritten = {
  args: {
    heading: heading$2,
    brief,
    claims,
    diff,
    work,
    decision: {
      ...decision,
      note: "The gate arm is right, but nothing refuses a workflow that declares human_always on a step with no checks. Add that and a test that loads one."
    }
  }
};
const APatchTooLongToDraw = {
  args: {
    heading: heading$2,
    brief,
    claims,
    work,
    decision,
    diff: {
      ...diff,
      cut: "This is the first 2,000 lines of a 14,318-line patch. The rest is not on screen. Read the whole diff in the worktree named under Where the work is before deciding."
    }
  }
};
const AClaimWithNothingBehindIt = {
  args: {
    heading: heading$2,
    brief,
    claims,
    work,
    decision,
    diff: {
      files: [],
      emptyNote: "This job's worktree opened and holds no change against the branch it was cut from. That is what a diff_nonempty check refuses."
    }
  }
};
const TheDiffNotReadYet = {
  args: {
    heading: heading$2,
    brief,
    claims,
    work,
    decision,
    diff: void 0,
    diffAbsent: "Reading this job's diff."
  }
};
const __vite_glob_0_57 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AClaimWithNothingBehindIt,
  APatchTooLongToDraw,
  ChangesBeingWritten,
  TheDiffNotReadYet,
  WorkWaitingOnADecision,
  default: meta$6
}, Symbol.toStringTag, { value: "Module" }));
function ARunningJob({
  heading: heading2,
  steps: steps2,
  ranLabel = "What ran",
  stepsAbsent = "Nothing serves this Job's workflow, so its steps are unknown.",
  footprint,
  footprintLabel = "Files changed",
  footprintAbsent = "Nothing has reported this drone's changed files yet.",
  evidence,
  evidenceAbsent = "Nothing serves a work submission yet.",
  log,
  logLabel = "Where the work is",
  logAbsent = "Nothing serves this Job's paths, its branch or its brief.",
  onCopied
}) {
  return /* @__PURE__ */ jsxs("div", { className: "armada-screen__detail", children: [
    /* @__PURE__ */ jsx(JobDetailHeaderActions, { ...heading2, onCopied }),
    /* @__PURE__ */ jsxs("div", { className: "armada-screen__split", children: [
      /* @__PURE__ */ jsxs("div", { className: "armada-screen__col", children: [
        /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", children: ranLabel }),
        steps2.length === 0 ? /* @__PURE__ */ jsx("div", { className: "armada-screen__slot", children: /* @__PURE__ */ jsx(Absent, { name: "What ran", note: stepsAbsent }) }) : /* @__PURE__ */ jsx(WorkflowRail, { steps: steps2, pulsing: true, onCopied }),
        /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", "data-spaced": true, children: footprintLabel }),
        footprint === void 0 ? /* @__PURE__ */ jsx("div", { className: "armada-screen__slot", children: /* @__PURE__ */ jsx(Absent, { name: "Files changed", note: footprintAbsent }) }) : (
          /* Never pulsing. The rail already carries the one animated mark
             this screen is allowed, and it is on the more specific thing. */
          /* @__PURE__ */ jsx(ChangedFiles, { ...footprint, onCopied })
        )
      ] }),
      /* @__PURE__ */ jsxs("div", { className: "armada-screen__col", children: [
        /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", children: "Evidence so far" }),
        evidence === void 0 ? /* @__PURE__ */ jsx("div", { className: "armada-screen__slot", children: /* @__PURE__ */ jsx(Absent, { name: "Evidence", note: evidenceAbsent }) }) : /* @__PURE__ */ jsx(EvidenceCard, { ...evidence }),
        /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", "data-spaced": true, children: logLabel }),
        log === void 0 ? /* @__PURE__ */ jsx("div", { className: "armada-screen__slot", children: /* @__PURE__ */ jsx(Absent, { name: "Where the work is", note: logAbsent }) }) : /* @__PURE__ */ jsxs("div", { className: "armada-screen__col", children: [
          log.brief === void 0 ? null : /* @__PURE__ */ jsx(JobBrief, { ...log.brief }),
          /* @__PURE__ */ jsx(JobLogReference, { rows: log.rows, actions: log.actions, onCopied, children: log.note })
        ] })
      ] })
    ] })
  ] });
}
const meta$5 = {
  title: "Screens/A running job",
  component: ARunningJob
};
const NO_GLYPH_IN_REGISTRY = void 0;
const steps = [
  {
    id: "plan",
    label: "Plan the change",
    activity: "advanced",
    status: "advanced",
    evidence: { icon: NO_GLYPH_IN_REGISTRY, iconLabel: "Evidence", label: "evidence · 09:14" }
  },
  {
    id: "implement",
    label: "Implement",
    activity: "running",
    status: "running · 6m 12s",
    current: true,
    gates: [
      {
        command: "build · cargo build --workspace",
        result: "not reached",
        icon: ShieldMinus,
        iconLabel: "Not reached"
      },
      {
        command: "diff_nonempty",
        result: "not reached",
        icon: ShieldMinus,
        iconLabel: "Not reached"
      }
    ],
    declarations: [
      { label: "judge · 2 criteria", result: "not reached" },
      { label: "advance_gate · auto_if_judge_passes" }
    ]
  },
  {
    id: "verify",
    label: "Run tests",
    activity: "not_started",
    status: "not started",
    gates: [
      {
        command: "test · cargo test --workspace",
        result: "not reached",
        icon: ShieldMinus,
        iconLabel: "Not reached"
      }
    ],
    declarations: [
      { label: "judge · 1 criterion · gaming check", result: "not reached" },
      { label: "advance_gate · auto_if_judge_passes" }
    ]
  },
  {
    id: "handoff",
    label: "Summarise",
    activity: "not_started",
    status: "not started",
    // The drawing draws no row under Summarise, and the rail drew "no check on
    // this step" — on the step the Job halts at. What it declares is a person,
    // and that is the row it gets.
    declarations: [{ label: "advance_gate · human_always" }]
  }
];
const WORK_ROWS = [
  {
    icon: Folder,
    iconLabel: "Worktree",
    value: "/repos/armada/.armada/worktrees/job_2d90bb",
    copyValue: "/repos/armada/.armada/worktrees/job_2d90bb"
  },
  {
    icon: GitBranch,
    iconLabel: "Branch",
    value: "fix/settings-split",
    copyValue: "fix/settings-split"
  },
  {
    icon: File,
    iconLabel: "Log",
    value: "/repos/armada/.armada/logs/job_2d90bb.jsonl",
    copyValue: "/repos/armada/.armada/logs/job_2d90bb.jsonl",
    separated: true
  },
  {
    iconLabel: "Transcript",
    value: "/repos/armada/.armada/transcripts/",
    copyValue: "/repos/armada/.armada/transcripts/",
    meta: "named by a drone id nothing serves"
  }
];
const FOOTPRINT = {
  emptyNote: "This drone has not changed anything yet.",
  note: "Read from the worktree while the drone was working. This step declared no plan, so no row is marked.",
  files: [
    { path: "src/settings.rs", change: "modified" },
    { path: "src/settings/reducer.rs", change: "added" },
    { path: "src/settings/selectors.rs", change: "added" },
    { path: "src/settings/mod.rs", change: "added" }
  ]
};
const heading$1 = {
  status: "running",
  statusIcon: CircleDot,
  statusLabel: "Running",
  headline: "Split the settings reducer",
  jobId: "job_2d90bb",
  fields: [
    { label: "Step", value: "2 of 4", mono: true },
    { label: "Branch", value: "fix/settings-split", mono: true, copyValue: "fix/settings-split" },
    { label: "Elapsed", value: "11m 03s", mono: true },
    { label: "Spend, estimated", value: "~$1.80", mono: true },
    { label: "Dispatched by you" }
  ],
  actions: /* @__PURE__ */ jsx(Button, { variant: "destructive", children: "Kill" })
};
const RunningJob = {
  render: () => /* @__PURE__ */ jsx("div", { className: "armada-screen", children: /* @__PURE__ */ jsx(
    ARunningJob,
    {
      heading: heading$1,
      steps,
      footprint: FOOTPRINT,
      evidence: {
        icon: NO_GLYPH_IN_REGISTRY,
        iconLabel: "Evidence",
        step: "Plan the change",
        time: "09:14",
        claimed: "settings.rs is split into a reducer and a selector module, with no change in behaviour.",
        shownBy: "src/settings.rs → src/settings/reducer.rs, src/settings/selectors.rs",
        notClaimed: "Nothing about the settings UI, and no new tests — the existing suite is the only cover."
      },
      log: {
        brief: {
          criteria: [
            {
              text: "settings.rs is split into a reducer and a selector module.",
              source: "check"
            },
            { text: "No change in behaviour, and the existing suite still passes.", source: "judge" }
          ],
          facts: "The reducer is the only caller of `apply_defaults`. Keep the public signature."
        },
        rows: WORK_ROWS,
        note: "The log is Fleet, the drone and Bridge in one order, keyed on this job. The transcript is named by a drone id nothing serves — the log above is the only record of it.",
        actions: /* @__PURE__ */ jsx(Button, { ground: "sunken", size: "sm", children: "Open the log" })
      }
    }
  ) })
};
const UNGATED = "No operation serves this step's checks";
const AsBridgeDrawsItToday = {
  render: () => /* @__PURE__ */ jsx("div", { className: "armada-screen", children: /* @__PURE__ */ jsx(
    ARunningJob,
    {
      heading: {
        status: "running",
        statusIcon: CircleDot,
        statusLabel: "running",
        headline: "Split the settings reducer",
        jobId: "job_2d90bb",
        fields: [
          { label: "Step", value: "2 of 4", mono: true },
          { label: "at", value: "implement", mono: true, continues: true },
          { label: "Elapsed", value: "11m 03s", mono: true },
          { label: "Branch", value: "fix/settings-split", mono: true, copyValue: "fix/settings-split" },
          { label: "Workflow", value: "bug" },
          { label: "Model", value: "sonnet", mono: true },
          { label: "Drone", value: "drn_7c21", mono: true, copyValue: "drn_7c21" },
          { label: "Writes", value: "src/settings/reducer.ts", mono: true }
        ],
        actions: /* @__PURE__ */ jsxs(Fragment, { children: [
          /* @__PURE__ */ jsx(Button, { variant: "destructive", children: "Kill drone" }),
          /* @__PURE__ */ jsx(Button, { variant: "destructive", children: "Kill job" })
        ] })
      },
      steps: [
        {
          id: "plan",
          label: "plan",
          labelIsAnIdentifier: true,
          activity: "advanced",
          elapsed: "2m 14s",
          verdict: "passed",
          verdictNamed: "passed",
          ungatedLabel: UNGATED,
          evidence: { label: "" }
        },
        {
          id: "implement",
          label: "implement",
          labelIsAnIdentifier: true,
          activity: "running",
          current: true,
          elapsed: "8m 49s",
          ungatedLabel: UNGATED,
          evidence: { label: "" }
        },
        {
          id: "verify",
          label: "verify",
          labelIsAnIdentifier: true,
          activity: "not_started",
          ungatedLabel: UNGATED,
          evidence: { label: "" }
        },
        {
          id: "handoff",
          label: "handoff",
          labelIsAnIdentifier: true,
          activity: "not_started",
          ungatedLabel: UNGATED,
          evidence: { label: "" }
        }
      ],
      footprint: FOOTPRINT,
      log: {
        brief: {
          criteria: [],
          criteriaAbsent: "This job was proposed with no acceptance criteria, so nothing states what done means for it.",
          facts: "The reducer is the only caller of `apply_defaults`. Keep the public signature."
        },
        rows: WORK_ROWS,
        note: "The worktree and the log are derived from the job id and the repository the manifest was read from. The branch is served."
      }
    }
  ) })
};
const AGateReplyWaitingForTheNextDrone = {
  render: () => /* @__PURE__ */ jsx("div", { className: "armada-screen", children: /* @__PURE__ */ jsx(
    ARunningJob,
    {
      heading: {
        ...heading$1,
        status: "queued",
        statusIcon: CircleDot,
        statusLabel: "Queued",
        fields: [
          { label: "Step", value: "2 of 4", mono: true },
          {
            label: "Branch",
            value: "fix/settings-split",
            mono: true,
            copyValue: "fix/settings-split"
          },
          { label: "Elapsed", value: "24m 11s", mono: true },
          { label: "Dispatched by you" }
        ],
        actions: /* @__PURE__ */ jsx(Button, { variant: "destructive", children: "Kill" })
      },
      steps,
      footprintAbsent: "No drone is working this job, so nothing is reporting changed files.",
      evidenceAbsent: "Submissions are read at the review gate, beside the diff they are claims about.",
      log: {
        brief: {
          criteria: [
            {
              text: "settings.rs is split into a reducer and a selector module.",
              source: "check"
            },
            {
              text: "No change in behaviour, and the existing suite still passes.",
              source: "judge"
            }
          ],
          facts: "The reducer is the only caller of `apply_defaults`. Keep the public signature.",
          waiting: "The selectors module still reaches into the reducer's private state. Name the cause, not the symptom — say why the boundary is where it is before you move anything."
        },
        rows: WORK_ROWS,
        note: "The worktree and the log are derived from the job id and the repository the manifest was read from. The branch is served."
      }
    }
  ) })
};
const __vite_glob_0_58 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AGateReplyWaitingForTheNextDrone,
  AsBridgeDrawsItToday,
  RunningJob,
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
const __vite_glob_0_59 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
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
const __vite_glob_0_60 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  FirstLaunch,
  default: meta$3
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
const meta$2 = {
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
const __vite_glob_0_61 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
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
  default: meta$2
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
const meta$1 = {
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
const __vite_glob_0_62 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  CollapsedRail,
  FleetIsNotRunning,
  Shell,
  default: meta$1
}, Symbol.toStringTag, { value: "Module" }));
function WatchingADroneWork({
  heading: heading2,
  turns: turns2,
  emptyNote,
  turnsLabel = "The drone's turns",
  readOnlyNote = "Watching only. The drone is not told, nothing about the job changes, and closing this view ends nothing.",
  live = false,
  liveNote = "A drone is writing now.",
  quietNote = "Nothing is writing. This is the whole history.",
  skipped = 0,
  missed = 0,
  closedBecause,
  failure
}) {
  return /* @__PURE__ */ jsxs("div", { className: "armada-screen__detail", children: [
    /* @__PURE__ */ jsx(JobDetailHeaderActions, { ...heading2 }),
    /* @__PURE__ */ jsxs("div", { className: "armada-screen__col", children: [
      /* @__PURE__ */ jsxs("div", { className: "armada-screen__head-row", children: [
        /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", children: turnsLabel }),
        /* @__PURE__ */ jsx("span", { className: "armada-screen__caption", children: live ? liveNote : quietNote })
      ] }),
      /* @__PURE__ */ jsx("p", { className: "armada-screen__caption", "data-note": true, children: readOnlyNote }),
      missed > 0 ? /* @__PURE__ */ jsx(Alert, { tone: "escalated", title: "Rows were dropped before this window saw them", children: `${missed} turns will never arrive. What follows is everything else, in order.` }) : null,
      skipped > 0 ? /* @__PURE__ */ jsx(Alert, { tone: "neutral", title: "Older turns are not shown", children: `${skipped} earlier turns are on disk and were left out of this history.` }) : null,
      failure === void 0 ? /* @__PURE__ */ jsx(DroneTurns, { turns: turns2, emptyNote, live }) : /* @__PURE__ */ jsx(Alert, { tone: "escalated", title: "This job's turns could not be read", children: failure }),
      closedBecause === void 0 ? null : /* @__PURE__ */ jsxs("p", { className: "armada-screen__caption", "data-note": true, children: [
        "Nothing more is coming: ",
        /* @__PURE__ */ jsx("span", { className: "armada-screen__mono", children: closedBecause })
      ] })
    ] })
  ] });
}
const meta = {
  title: "Screens/Watching a drone work",
  component: WatchingADroneWork
};
const NOTHING_YET = "This job has no turns. It was never dispatched, so no drone has written one.";
const heading = {
  status: "running",
  statusIcon: CircleDot,
  statusLabel: "Running",
  headline: "Split the settings reducer",
  jobId: "job_2d90bb",
  fields: [
    { label: "Step", value: "2 of 4", mono: true },
    { label: "Drone", value: "drone_9c41", mono: true, copyValue: "drone_9c41" },
    { label: "Elapsed", value: "11m 03s", mono: true }
  ],
  // Leaves the view. Never an act on the Drone.
  actions: /* @__PURE__ */ jsx(Button, { variant: "ghost", children: "Back to the job" })
};
const PLAN = { id: "plan", label: "Plan the change" };
const IMPLEMENT = { id: "implement", label: "Split the reducer" };
const turns = [
  {
    id: "1",
    step: PLAN,
    at: "09:14:02",
    kind: "started",
    // The model is whatever the Job named. A vendor spelling belongs in
    // `adapters` and nowhere else, so the fixture carries a placeholder.
    subject: "sess_01JB4 · the job's model · 2 mcp servers"
  },
  {
    id: "2",
    step: PLAN,
    at: "09:14:03",
    kind: "said",
    said: "Reading the settings module before I split anything, so the public signature survives."
  },
  {
    id: "3",
    step: IMPLEMENT,
    at: "09:14:04",
    kind: "called",
    subject: "Read",
    detail: "src/settings.rs",
    answer: "Answered."
  },
  // No detail: the wire had no name for this tool's arguments, so the call id
  // is what tells the row from the next one and the row leads with it.
  { id: "4", step: IMPLEMENT, at: "09:14:08", kind: "called", subject: "TodoWrite · call_7f22", answer: "Answered." },
  {
    id: "5",
    step: IMPLEMENT,
    at: "09:14:11",
    kind: "called",
    subject: "Edit",
    detail: "src/settings.rs +42 -18",
    answer: "No answer yet."
  }
];
const ADroneWriting = {
  render: () => /* @__PURE__ */ jsx("div", { className: "armada-screen", children: /* @__PURE__ */ jsx(WatchingADroneWork, { heading, turns, emptyNote: NOTHING_YET, live: true }) })
};
const AJobWithNoTranscript = {
  render: () => /* @__PURE__ */ jsx("div", { className: "armada-screen", children: /* @__PURE__ */ jsx(
    WatchingADroneWork,
    {
      heading: { ...heading, statusLabel: "Needs approval", status: "awaiting-approval" },
      turns: [],
      emptyNote: NOTHING_YET,
      closedBecause: "nothing_writing"
    }
  ) })
};
const AViewerThatMissedRows = {
  render: () => /* @__PURE__ */ jsx("div", { className: "armada-screen", children: /* @__PURE__ */ jsx(
    WatchingADroneWork,
    {
      heading,
      turns,
      emptyNote: NOTHING_YET,
      live: true,
      missed: 34
    }
  ) })
};
const ADroneThatOutlivedItsFleet = {
  render: () => /* @__PURE__ */ jsx("div", { className: "armada-screen", children: /* @__PURE__ */ jsx(
    WatchingADroneWork,
    {
      heading,
      turns,
      emptyNote: NOTHING_YET,
      skipped: 128,
      closedBecause: "drone_ended"
    }
  ) })
};
const TheTurnsCouldNotBeRead = {
  render: () => /* @__PURE__ */ jsx("div", { className: "armada-screen", children: /* @__PURE__ */ jsx(
    WatchingADroneWork,
    {
      heading,
      turns: [],
      emptyNote: NOTHING_YET,
      failure: "Fleet did not answer on this job's observe socket: connect ECONNREFUSED 127.0.0.1:7777"
    }
  ) })
};
function thinking(from, rows, at) {
  return Array.from({ length: rows }, (_, n) => ({
    id: String(from + n),
    at,
    kind: "unrecognised",
    subject: n % 4 === 3 ? "a turn with nothing in it Armada names" : "system/thinking_tokens",
    quiet: true
  }));
}
const withThinking = [
  { id: "1", at: "09:14:02", kind: "started", subject: "sess_01JB4 · the job's model · 1 mcp server" },
  ...thinking(10, 3, "09:14:03"),
  { id: "20", at: "09:14:05", kind: "said", said: "Reading the settings module before I split anything." },
  ...thinking(30, 9, "09:14:06"),
  { id: "50", at: "09:14:12", kind: "called", subject: "Read", detail: "src/settings.rs", answer: "Answered." },
  ...thinking(60, 14, "09:14:15"),
  { id: "80", at: "09:14:31", kind: "called", subject: "Edit", detail: "src/settings.rs +42 -18", answer: "Answered." },
  ...thinking(90, 6, "09:14:40")
];
const ADroneThinking = {
  render: () => /* @__PURE__ */ jsx("div", { className: "armada-screen", children: /* @__PURE__ */ jsx(WatchingADroneWork, { heading, turns: withThinking, emptyNote: NOTHING_YET, live: true }) })
};
const AFinishedRun = {
  render: () => /* @__PURE__ */ jsx("div", { className: "armada-screen", children: /* @__PURE__ */ jsx(
    WatchingADroneWork,
    {
      heading: { ...heading, statusLabel: "Awaiting review", status: "awaiting-review", statusIcon: Eye },
      turns: [
        ...withThinking,
        { id: "120", at: "09:14:47", kind: "said", said: "The public signature is unchanged. Submitting." }
      ],
      emptyNote: NOTHING_YET,
      closedBecause: "drone_ended"
    }
  ) })
};
const NothingButToolCalls = {
  render: () => /* @__PURE__ */ jsx("div", { className: "armada-screen", children: /* @__PURE__ */ jsx(
    WatchingADroneWork,
    {
      heading,
      turns: [
        { id: "1", at: "09:20:01", kind: "called", subject: "Bash", detail: "cargo xtask verify-foundations", answer: "Answered." },
        { id: "2", at: "09:20:31", kind: "called", subject: "Bash", detail: "cargo test -p ipc", answer: "Answered, and the tool itself failed." },
        { id: "3", at: "09:20:48", kind: "called", subject: "Read", detail: "crates/ipc/src/turn.rs", answer: "Answered." },
        { id: "4", at: "09:20:52", kind: "called", subject: "Write", detail: "crates/ipc/src/turn.rs, 214 lines starting //! One Drone's turns", truncated: true, answer: "No answer yet." }
      ],
      emptyNote: NOTHING_YET,
      live: true
    }
  ) })
};
const __vite_glob_0_63 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  ADroneThatOutlivedItsFleet,
  ADroneThinking,
  ADroneWriting,
  AFinishedRun,
  AJobWithNoTranscript,
  AViewerThatMissedRows,
  NothingButToolCalls,
  TheTurnsCouldNotBeRead,
  default: meta
}, Symbol.toStringTag, { value: "Module" }));
const stories = /* @__PURE__ */ Object.assign({
  "../src/compositions/ActiveJobsList/ActiveJobsList.stories.tsx": __vite_glob_0_0,
  "../src/compositions/BoardControls/BoardControls.stories.tsx": __vite_glob_0_1,
  "../src/compositions/BoardEmptyState/BoardEmptyState.stories.tsx": __vite_glob_0_2,
  "../src/compositions/ChangedFiles/ChangedFiles.stories.tsx": __vite_glob_0_3,
  "../src/compositions/CriterionVerdicts/CriterionVerdicts.stories.tsx": __vite_glob_0_4,
  "../src/compositions/DroneTurns/DroneTurns.stories.tsx": __vite_glob_0_5,
  "../src/compositions/EvidenceCard/EvidenceCard.stories.tsx": __vite_glob_0_6,
  "../src/compositions/EvidenceTrail/EvidenceTrail.stories.tsx": __vite_glob_0_7,
  "../src/compositions/FailureNotice/FailureNotice.stories.tsx": __vite_glob_0_8,
  "../src/compositions/GamingFlags/GamingFlags.stories.tsx": __vite_glob_0_9,
  "../src/compositions/JobBrief/JobBrief.stories.tsx": __vite_glob_0_10,
  "../src/compositions/JobComposer/JobComposer.stories.tsx": __vite_glob_0_11,
  "../src/compositions/JobDetailHeaderActions/JobDetailHeaderActions.stories.tsx": __vite_glob_0_12,
  "../src/compositions/JobLogReference/JobLogReference.stories.tsx": __vite_glob_0_13,
  "../src/compositions/JobOutcome/JobOutcome.stories.tsx": __vite_glob_0_14,
  "../src/compositions/JobRecord/JobRecord.stories.tsx": __vite_glob_0_15,
  "../src/compositions/JobRowStacked/JobRowStacked.stories.tsx": __vite_glob_0_16,
  "../src/compositions/ReviewDecision/ReviewDecision.stories.tsx": __vite_glob_0_17,
  "../src/compositions/Sidebar/Sidebar.stories.tsx": __vite_glob_0_18,
  "../src/compositions/StatusBar/StatusBar.stories.tsx": __vite_glob_0_19,
  "../src/compositions/StepActivityMark/StepActivityMark.stories.tsx": __vite_glob_0_20,
  "../src/compositions/StepBar/StepBar.stories.tsx": __vite_glob_0_21,
  "../src/compositions/TransitionHistory/TransitionHistory.stories.tsx": __vite_glob_0_22,
  "../src/compositions/UnifiedDiff/UnifiedDiff.stories.tsx": __vite_glob_0_23,
  "../src/compositions/WorkflowRail/WorkflowRail.stories.tsx": __vite_glob_0_24,
  "../src/errors/ErrorCode/ErrorCode.stories.tsx": __vite_glob_0_25,
  "../src/errors/ErrorNotice/ErrorNotice.stories.tsx": __vite_glob_0_26,
  "../src/errors/FileAnIssue/FileAnIssue.stories.tsx": __vite_glob_0_27,
  "../src/primitives/Alert/Alert.stories.tsx": __vite_glob_0_28,
  "../src/primitives/AttachmentChip/AttachmentChip.stories.tsx": __vite_glob_0_29,
  "../src/primitives/Badge/Badge.stories.tsx": __vite_glob_0_30,
  "../src/primitives/Button/Button.stories.tsx": __vite_glob_0_31,
  "../src/primitives/Card/Card.stories.tsx": __vite_glob_0_32,
  "../src/primitives/Checkbox/Checkbox.stories.tsx": __vite_glob_0_33,
  "../src/primitives/CommandPalette/CommandPalette.stories.tsx": __vite_glob_0_34,
  "../src/primitives/Dialog/Dialog.stories.tsx": __vite_glob_0_35,
  "../src/primitives/DropdownMenu/DropdownMenu.stories.tsx": __vite_glob_0_36,
  "../src/primitives/Input/Input.stories.tsx": __vite_glob_0_37,
  "../src/primitives/Kbd/Kbd.stories.tsx": __vite_glob_0_38,
  "../src/primitives/Popover/Popover.stories.tsx": __vite_glob_0_39,
  "../src/primitives/Prose/Prose.stories.tsx": __vite_glob_0_40,
  "../src/primitives/Radio/Radio.stories.tsx": __vite_glob_0_41,
  "../src/primitives/ScrollArea/ScrollArea.stories.tsx": __vite_glob_0_42,
  "../src/primitives/Select/Select.stories.tsx": __vite_glob_0_43,
  "../src/primitives/Separator/Separator.stories.tsx": __vite_glob_0_44,
  "../src/primitives/Sheet/Sheet.stories.tsx": __vite_glob_0_45,
  "../src/primitives/Skeleton/Skeleton.stories.tsx": __vite_glob_0_46,
  "../src/primitives/SplitButton/SplitButton.stories.tsx": __vite_glob_0_47,
  "../src/primitives/Switch/Switch.stories.tsx": __vite_glob_0_48,
  "../src/primitives/Table/Table.stories.tsx": __vite_glob_0_49,
  "../src/primitives/Tabs/Tabs.stories.tsx": __vite_glob_0_50,
  "../src/primitives/TabsWithCounts/TabsWithCounts.stories.tsx": __vite_glob_0_51,
  "../src/primitives/Textarea/Textarea.stories.tsx": __vite_glob_0_52,
  "../src/primitives/Toast/Toast.stories.tsx": __vite_glob_0_53,
  "../src/primitives/Tooltip/Tooltip.stories.tsx": __vite_glob_0_54,
  "../src/screens/AFailedJobADeadEndReadAsOne/AFailedJobADeadEndReadAsOne.stories.tsx": __vite_glob_0_55,
  "../src/screens/AFinishedJobWhatItWasAndWhatItProduced/AFinishedJobWhatItWasAndWhatItProduced.stories.tsx": __vite_glob_0_56,
  "../src/screens/AJobAwaitingReviewTheDiffAndTheReplyAreOneLoop/AJobAwaitingReviewTheDiffAndTheReplyAreOneLoop.stories.tsx": __vite_glob_0_57,
  "../src/screens/ARunningJob/ARunningJob.stories.tsx": __vite_glob_0_58,
  "../src/screens/DispatchAJobFullWithTheM1SubsetMarked/DispatchAJobFullWithTheM1SubsetMarked.stories.tsx": __vite_glob_0_59,
  "../src/screens/FirstLaunch/FirstLaunch.stories.tsx": __vite_glob_0_60,
  "../src/screens/TheListSixStatesOneRowShape/TheListSixStatesOneRowShape.stories.tsx": __vite_glob_0_61,
  "../src/screens/TheShell/TheShell.stories.tsx": __vite_glob_0_62,
  "../src/screens/WatchingADroneWork/WatchingADroneWork.stories.tsx": __vite_glob_0_63
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
