import { jsx, jsxs, Fragment } from "react/jsx-runtime";
import { ChevronDown, UserCheck, Cpu, GitBranch, CircleDot, X, Check, Power, Folder, OctagonAlert, Clock, MessageSquare, ClipboardList, Activity, Bell, Eye, ScrollText, Stethoscope, FileCog, Flag, RotateCw, ShieldCheck, ShieldX, ShieldMinus, Lock, TriangleAlert, Stamp, Terminal, Link, Ban, Archive, RefreshCw, FileQuestionMark, Split, Unplug, ArrowUpToLine, Send, CornerUpRight, Settings } from "lucide-react";
import { useState, useCallback, useRef, useEffect, useId, useMemo, createElement } from "react";
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
        onClick: item.onSelect,
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
  pulsing = false,
  focused,
  selected,
  dimmed,
  onOpen,
  onCopied
}) {
  return /* @__PURE__ */ jsxs(
    "div",
    {
      className: "armada-job-row",
      role: "listitem",
      "data-focused": focused || void 0,
      "data-selected": selected || void 0,
      "data-dimmed": dimmed || void 0,
      onClick: onOpen,
      children: [
        /* @__PURE__ */ jsx("div", { className: "armada-job-row__badge", children: /* @__PURE__ */ jsx(Badge, { status, icon: statusIcon, pulsing, children: statusLabel }) }),
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
              style: { gridTemplateColumns: tracks ?? trackList(fields.length) },
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
          /* @__PURE__ */ jsx("div", { className: "armada-job-row__action", onClick: (e) => e.stopPropagation(), children: action })
        ) : null
      ]
    }
  );
}
const DRAWN_TRACKS = [
  "var(--armada-track-origin)",
  "var(--armada-track-bar)",
  "var(--armada-track-step)",
  "var(--armada-track-time)",
  "var(--armada-track-spend)"
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
function ActiveJobsList({ heading, summary, action, children, empty }) {
  const rows = Array.isArray(children) ? children.filter(Boolean) : children;
  const isEmpty = rows === void 0 || rows === null || Array.isArray(rows) && rows.length === 0;
  return /* @__PURE__ */ jsxs("section", { className: "armada-active-jobs", children: [
    heading || summary || action ? /* @__PURE__ */ jsxs("header", { className: "armada-active-jobs__header", children: [
      /* @__PURE__ */ jsxs("div", { className: "armada-active-jobs__titles", children: [
        heading ? /* @__PURE__ */ jsx("h2", { className: "armada-active-jobs__heading", children: heading }) : null,
        summary ? /* @__PURE__ */ jsx("p", { className: "armada-active-jobs__summary", children: summary }) : null
      ] }),
      action ? /* @__PURE__ */ jsx("div", { className: "armada-active-jobs__action", children: action }) : null
    ] }) : null,
    /* @__PURE__ */ jsx("div", { className: "armada-active-jobs__frame", role: "list", children: isEmpty ? empty : rows })
  ] });
}
const meta$E = {
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
          statusLabel: "Queued",
          headline: "Retire the legacy poke path",
          jobId: "job_8b42",
          fields: [
            { value: WORKFLOW },
            { value: /* @__PURE__ */ jsx(StepBar, { total: 4, current: 0, label: "Not started, 4 steps" }) },
            { value: "Waiting on a drone", emphasis: true },
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
const __vite_glob_0_0 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AtTheWidthFloor: AtTheWidthFloor$1,
  EmptyWithNoEmptyState,
  SixStates,
  default: meta$E
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
const meta$D = {
  title: "Compositions/Evidence trail",
  component: EvidenceTrail
};
const NO_GLYPH_IN_REGISTRY$6 = void 0;
const AFinishedJob = {
  args: {
    entries: [
      {
        icon: NO_GLYPH_IN_REGISTRY$6,
        iconLabel: "Evidence",
        step: "Plan the change",
        provenance: "14:02 · facts_note · no check",
        claimed: "The poke loop stops after 3 attempts and the job records how many it spent.",
        shownBy: "core/fleet/src/lease.rs · the loop has no ceiling today",
        notClaimed: "Does not change the poke interval, and does not decide what happens at the third failure."
      },
      {
        icon: NO_GLYPH_IN_REGISTRY$6,
        iconLabel: "Evidence",
        step: "Implement",
        provenance: "14:11 · diff · build exit 0 · diff_nonempty passed",
        claimed: "A drone that stops answering is poked at most 3 times, and the count is on the job record.",
        shownBy: "core/fleet/src/lease.rs +38 −7 · core/model/src/job.rs +14 −0",
        notClaimed: "The count is not surfaced in Bridge. Nothing acts on reaching the ceiling yet — the loop exits and the job keeps its status."
      },
      {
        icon: NO_GLYPH_IN_REGISTRY$6,
        iconLabel: "Evidence",
        step: "Run tests",
        provenance: "14:16 · test_suite_run · test exit 0",
        claimed: "The ceiling holds at 3 and the counter increments once per poke.",
        shownBy: "cargo test --workspace · 86 passed 0 failed 5.1s · lease::poke_ceiling_holds, lease::poke_count_increments",
        notClaimed: "No test covers a drone that answers on the third poke. The suite was green before this change and is green after, so it does not prove the ceiling is reached in practice."
      },
      {
        icon: NO_GLYPH_IN_REGISTRY$6,
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
        icon: NO_GLYPH_IN_REGISTRY$6,
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
        icon: NO_GLYPH_IN_REGISTRY$6,
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
const __vite_glob_0_1 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AFinishedJob,
  NotClaimedEmpty,
  OneEntry,
  default: meta$D
}, Symbol.toStringTag, { value: "Module" }));
const ROW_ICON = 12;
const ROW_STROKE = 2;
function JobLogReference({ rows, children, actions, onCopied }) {
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
  return /* @__PURE__ */ jsxs("div", { className: "armada-log-ref", children: [
    rows.map((row, i) => /* @__PURE__ */ jsxs("div", { className: "armada-log-ref__row", "data-separated": row.separated || void 0, children: [
      /* @__PURE__ */ jsxs("span", { className: "armada-log-ref__mark", children: [
        row.icon ? /* @__PURE__ */ jsx(row.icon, { size: ROW_ICON, strokeWidth: ROW_STROKE, "aria-hidden": true }) : null,
        row.iconLabel ? /* @__PURE__ */ jsx("span", { className: "armada-log-ref__sr", children: row.iconLabel }) : null
      ] }),
      /* @__PURE__ */ jsx(
        "span",
        {
          className: "armada-log-ref__value",
          "data-copies": row.copyValue !== void 0 || void 0,
          onClick: row.copyValue !== void 0 ? (e) => copy(e, row.copyValue) : void 0,
          children: row.value
        }
      ),
      row.meta ? /* @__PURE__ */ jsx("span", { className: "armada-log-ref__meta", children: row.meta }) : null
    ] }, i)),
    children || actions ? /* @__PURE__ */ jsxs("div", { className: "armada-log-ref__foot", children: [
      children ? /* @__PURE__ */ jsx("p", { className: "armada-log-ref__note", children }) : null,
      actions ? /* @__PURE__ */ jsx("div", { className: "armada-log-ref__actions", children: actions }) : null
    ] }) : null
  ] });
}
const meta$C = {
  title: "Compositions/Job log reference",
  component: JobLogReference
};
const NO_GLYPH_IN_REGISTRY$5 = void 0;
const OnARunningJob = {
  args: {
    rows: [
      {
        icon: NO_GLYPH_IN_REGISTRY$5,
        iconLabel: "Log",
        value: ".armada/logs/job_2d90bb.jsonl",
        copyValue: ".armada/logs/job_2d90bb.jsonl",
        meta: "142 lines · 0 error"
      }
    ],
    children: "Fleet, the drone and Bridge in one order, keyed on this job. It is being written now.",
    actions: /* @__PURE__ */ jsx(Button, { ground: "sunken", children: "Open the log" })
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
      { icon: Folder, iconLabel: "Worktree", value: "~/.armada/worktrees/job_91ab" },
      {
        icon: NO_GLYPH_IN_REGISTRY$5,
        iconLabel: "Log",
        value: ".armada/logs/job_91ab.jsonl",
        copyValue: ".armada/logs/job_91ab.jsonl",
        meta: "318 lines · 4 error",
        separated: true
      }
    ],
    children: "The worktree and the branch are left in place. Armada will not touch either. The log holds Fleet, the drone and Bridge in one order, keyed on this job.",
    actions: /* @__PURE__ */ jsxs(Fragment, { children: [
      /* @__PURE__ */ jsx(Button, { ground: "sunken", children: "Open the log" }),
      /* @__PURE__ */ jsx(Button, { ground: "sunken", children: "Open the worktree" })
    ] })
  }
};
const OnAFinishedJob = {
  args: {
    rows: [
      {
        icon: NO_GLYPH_IN_REGISTRY$5,
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
        icon: NO_GLYPH_IN_REGISTRY$5,
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
const __vite_glob_0_2 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  OnAFailedJob,
  OnAFinishedJob,
  OnARunningJob,
  WithErrors,
  default: meta$C
}, Symbol.toStringTag, { value: "Module" }));
const meta$B = {
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
const NeedsApproval = {
  args: {
    status: "awaiting-approval",
    statusIcon: UserCheck,
    statusLabel: "Needs approval",
    headline: "Coalesce concurrent token refreshes",
    jobId: "job_7c31",
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
const Queued$1 = {
  args: {
    status: "not-started",
    statusIcon: Clock,
    statusLabel: "Queued",
    headline: "Retire the legacy poke path",
    jobId: "job_8b42",
    fields: [
      { label: "Workflow", value: "bug, 4 steps", quiet: true },
      { value: /* @__PURE__ */ jsx(StepBar, { total: 4, current: 0, label: "Not started, 4 steps" }) },
      { value: "Waiting on a drone", emphasis: true },
      { value: "approved 09:20", quiet: true },
      { value: "Dispatched by you" }
    ],
    action: open$1
  }
};
const Running$4 = {
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
      { value: "~$1.80", mono: true }
    ],
    action: open$1
  }
};
const RunningFocused = {
  args: { ...Running$4.args, focused: true }
};
const Selected = {
  args: { ...Running$4.args, selected: true }
};
const Dimmed$2 = {
  args: { ...Running$4.args, dimmed: true }
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
      { value: "~$1.80", mono: true }
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
const Failed$3 = {
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
      { value: "~$2.10", mono: true }
    ],
    action: open$1
  }
};
const Killed$5 = {
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
      { value: "~$0.60", mono: true }
    ],
    action: open$1
  }
};
const Done = {
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
      { value: "~$2.40", mono: true }
    ],
    action: open$1
  }
};
const SpendAsQuota = {
  args: {
    ...Running$4.args,
    fields: [
      { value: "fix/settings-split", mono: true, icon: GitBranch, copyValue: "fix/settings-split" },
      { value: /* @__PURE__ */ jsx(StepBar, { total: 4, current: 2, activity: "running", label: "Step 2 of 4" }) },
      { value: "Implement", emphasis: true },
      { value: "11m 03s", mono: true },
      { value: "68% quota", mono: true }
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
        { value: "~$1.80", mono: true }
      ],
      action: open$1
    }
  ) })
};
const Convoy = {
  args: {
    ...Running$4.args,
    headline: "Retire the poke path across the fleet",
    fields: [
      { value: "3 workspaces" },
      { value: /* @__PURE__ */ jsx(StepBar, { total: 4, current: 2, activity: "running", label: "Step 2 of 4" }) },
      { value: "Implement", emphasis: true },
      { value: "11m 03s", mono: true },
      { value: "~$4.20", mono: true }
    ]
  }
};
const __vite_glob_0_3 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AtTheWidthFloor,
  Convoy,
  Dimmed: Dimmed$2,
  Done,
  EscalatedSecondTime,
  EscalatedStalled,
  Failed: Failed$3,
  Killed: Killed$5,
  NeedsApproval,
  Queued: Queued$1,
  Running: Running$4,
  RunningFocused,
  Selected,
  SpendAsQuota,
  default: meta$B
}, Symbol.toStringTag, { value: "Module" }));
function Select({ label: label2, invalid = false, message, id, children, ...rest }) {
  const generated = useId();
  const selectId = id ?? generated;
  const messageId = `${selectId}-message`;
  const showMessage = invalid && message !== void 0;
  return /* @__PURE__ */ jsxs("div", { className: "armada-select-field", children: [
    label2 !== void 0 && /* @__PURE__ */ jsx("label", { className: "armada-select-field__label", htmlFor: selectId, children: label2 }),
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
    showMessage && /* @__PURE__ */ jsx("span", { className: "armada-select-field__message", id: messageId, children: message })
  ] });
}
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
const meta$A = {
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
const CollapsedRail = {
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
const __vite_glob_0_4 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AtMaximumWidth,
  AtMinimumWidth,
  CollapsedRail,
  Expanded,
  FlatForContrast,
  HelmActive,
  M1OneSurface,
  default: meta$A
}, Symbol.toStringTag, { value: "Module" }));
function count(n, one, many) {
  return `${n} ${n === 1 ? one : many}`;
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
const meta$z = {
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
const __vite_glob_0_5 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
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
  default: meta$z
}, Symbol.toStringTag, { value: "Module" }));
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
const MARK_STROKE = 2;
function StepActivityMark({ activity, label: label2, ordinal, pulsing = false }) {
  const Icon = GLYPH[activity];
  const animates = pulsing && activity === "running";
  return /* @__PURE__ */ jsxs("span", { className: "armada-step-mark", "data-activity": activity, "data-pulsing": animates || void 0, children: [
    Icon ? /* @__PURE__ */ jsx(Icon, { size: MARK_ICON, strokeWidth: MARK_STROKE, "aria-hidden": true }) : ordinal !== void 0 ? /* @__PURE__ */ jsx("span", { className: "armada-step-mark__ordinal", "aria-hidden": true, children: ordinal }) : null,
    /* @__PURE__ */ jsx("span", { className: "armada-step-mark__name", children: label2 })
  ] });
}
const meta$y = {
  title: "Compositions/Step activity mark",
  component: StepActivityMark
};
const NotStarted$2 = {
  args: { activity: "not_started", label: "Not started", ordinal: 3 }
};
const NotStartedWithNoOrdinal = {
  args: { activity: "not_started", label: "Not started" }
};
const Running$3 = {
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
const Killed$4 = {
  args: { activity: "killed", label: "Killed" }
};
const Failed$2 = {
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
const __vite_glob_0_6 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Advanced,
  AwaitingHuman: AwaitingHuman$1,
  EveryValue,
  Failed: Failed$2,
  Killed: Killed$4,
  NotStarted: NotStarted$2,
  NotStartedWithNoOrdinal,
  Retrying,
  Running: Running$3,
  RunningPulsing: RunningPulsing$1,
  Stopped: Stopped$1,
  default: meta$y
}, Symbol.toStringTag, { value: "Module" }));
const meta$x = {
  title: "Compositions/Step bar",
  component: StepBar
};
const NotStarted$1 = {
  args: { total: 4, current: 0, label: "Not started, 4 steps" }
};
const Running$2 = {
  args: { total: 4, current: 2, activity: "running", label: "Step 2 of 4" }
};
const RunningLongWorkflow = {
  args: { total: 7, current: 5, activity: "running", label: "Step 5 of 7" }
};
const AwaitingHuman = {
  args: { total: 4, current: 3, activity: "awaiting_human", label: "Step 3 of 4" }
};
const Failed$1 = {
  args: { total: 4, current: 3, activity: "failed", label: "Step 3 of 4" }
};
const Killed$3 = {
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
const __vite_glob_0_7 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AllAdvanced,
  AwaitingHuman,
  Failed: Failed$1,
  Killed: Killed$3,
  NotStarted: NotStarted$1,
  Running: Running$2,
  RunningLongWorkflow,
  RunningNeverPulses,
  default: meta$x
}, Symbol.toStringTag, { value: "Module" }));
const GATE_ICON = 12;
const GATE_STROKE = 2;
function WorkflowRail({ steps: steps2, pulsing = false }) {
  return /* @__PURE__ */ jsx("ol", { className: "armada-rail", children: steps2.map((step, i) => {
    const gates = step.gates ?? [];
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
            step.status ? /* @__PURE__ */ jsx("span", { className: "armada-rail__status", children: step.status }) : null
          ]
        }
      ),
      gates.length > 0 ? /* @__PURE__ */ jsx("ul", { className: "armada-rail__gates", children: gates.map((gate, g) => /* @__PURE__ */ jsxs("li", { className: "armada-rail__gate", children: [
        /* @__PURE__ */ jsxs("span", { className: "armada-rail__gate-mark", children: [
          gate.icon ? /* @__PURE__ */ jsx(gate.icon, { size: GATE_ICON, strokeWidth: GATE_STROKE, "aria-hidden": true }) : null,
          gate.iconLabel ? /* @__PURE__ */ jsx("span", { className: "armada-rail__sr", children: gate.iconLabel }) : null
        ] }),
        /* @__PURE__ */ jsx("span", { className: "armada-rail__gate-command", children: gate.command }),
        gate.result ? /* @__PURE__ */ jsx("span", { className: "armada-rail__gate-result", children: gate.result }) : null
      ] }, g)) }) : (
        // An ungated step says so in words. A step carrying no Check is
        // ordinary rather than exceptional, and a blank would read as a
        // gate that failed to render.
        /* @__PURE__ */ jsx("ul", { className: "armada-rail__gates", children: /* @__PURE__ */ jsxs("li", { className: "armada-rail__gate", "data-ungated": true, children: [
          /* @__PURE__ */ jsxs("span", { className: "armada-rail__gate-mark", children: [
            step.evidence?.icon ? /* @__PURE__ */ jsx(step.evidence.icon, { size: GATE_ICON, strokeWidth: GATE_STROKE, "aria-hidden": true }) : null,
            step.evidence?.iconLabel ? /* @__PURE__ */ jsx("span", { className: "armada-rail__sr", children: step.evidence.iconLabel }) : null
          ] }),
          /* @__PURE__ */ jsx("span", { className: "armada-rail__gate-command", children: step.evidence?.label }),
          /* @__PURE__ */ jsx("span", { className: "armada-rail__gate-ungated", children: step.ungatedLabel ?? "no check on this step" })
        ] }) })
      )
    ] }, step.id);
  }) });
}
const meta$w = {
  title: "Compositions/Workflow rail",
  component: WorkflowRail
};
const NO_GLYPH_IN_REGISTRY$4 = void 0;
const running = [
  {
    id: "plan",
    label: "Plan the change",
    activity: "advanced",
    status: "advanced",
    evidence: { icon: NO_GLYPH_IN_REGISTRY$4, iconLabel: "Evidence", label: "evidence · 09:14" }
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
    ]
  },
  {
    id: "verify",
    label: "Run tests",
    activity: "not_started",
    status: "not started",
    gates: [
      { command: "test · cargo test --workspace", result: "not reached", icon: ShieldMinus, iconLabel: "Not reached" }
    ]
  },
  {
    id: "handoff",
    label: "Summarise",
    activity: "not_started",
    status: "not started",
    evidence: { icon: NO_GLYPH_IN_REGISTRY$4, iconLabel: "Evidence", label: "" }
  }
];
const Running$1 = {
  args: { steps: running, pulsing: true }
};
const RunningPulseElsewhere = {
  args: { steps: running, pulsing: false }
};
const Failed = {
  args: {
    steps: [
      { id: "plan", label: "Plan the change", activity: "advanced", status: "advanced", evidence: { icon: NO_GLYPH_IN_REGISTRY$4, label: "evidence · 13:58" } },
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
const WaitingAndRetrying = {
  args: {
    steps: [
      { id: "plan", label: "Plan the change", activity: "advanced", status: "advanced" },
      { id: "review", label: "Review the diff", activity: "awaiting_human", status: "waiting on you", current: true },
      { id: "fix", label: "Fix", activity: "retrying", status: "retrying · attempt 2" }
    ]
  }
};
const Killed$2 = {
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
const __vite_glob_0_8 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Failed,
  HardPrerequisite,
  Killed: Killed$2,
  LabelsMissing,
  OneStep,
  Running: Running$1,
  RunningPulseElsewhere,
  Stopped,
  WaitingAndRetrying,
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
const __vite_glob_0_9 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Escalated: Escalated$1,
  Neutral,
  default: meta$v
}, Symbol.toStringTag, { value: "Module" }));
const meta$u = {
  title: "Primitives/Badge",
  component: Badge
};
const NO_GLYPH_IN_REGISTRY$3 = void 0;
const NotStarted = {
  args: { status: "not-started", icon: NO_GLYPH_IN_REGISTRY$3, children: "Not started" }
};
const Queued = {
  args: { status: "not-started", icon: Clock, children: "Queued" }
};
const QueuedOutOfHeadroom = {
  args: { status: "not-started", icon: Cpu, children: "Waiting on resources" }
};
const QueuedBlockedByDependency = {
  args: { status: "not-started", icon: Link, children: "Blocked by a dependency" }
};
const AwaitingApproval = {
  args: { status: "awaiting-approval", icon: UserCheck, children: "Awaiting approval" }
};
const Running = {
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
  args: { status: "escalated", icon: NO_GLYPH_IN_REGISTRY$3, children: "Escalated" }
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
const Killed$1 = {
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
const __vite_glob_0_10 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AwaitingApproval,
  AwaitingAttestation,
  AwaitingReview,
  CompletedFailed,
  CompletedSuccess,
  Escalated,
  EscalationReasons,
  Killed: Killed$1,
  NotStarted,
  Piloted,
  Queued,
  QueuedBlockedByDependency,
  QueuedOutOfHeadroom,
  Rejected,
  Running,
  RunningPulsing,
  Superseded,
  default: meta$u
}, Symbol.toStringTag, { value: "Module" }));
const meta$t = {
  title: "Primitives/Button",
  component: Button
};
function Card$7({ children }) {
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
  render: (args) => /* @__PURE__ */ jsx(Card$7, { children: /* @__PURE__ */ jsx(Button, { ...args }) })
};
const Secondary = {
  args: { variant: "secondary", children: "Cancel" },
  render: (args) => /* @__PURE__ */ jsx(Card$7, { children: /* @__PURE__ */ jsx(Button, { ...args }) })
};
const Ghost = {
  args: { variant: "ghost", children: "Ghost" },
  render: (args) => /* @__PURE__ */ jsx(Card$7, { children: /* @__PURE__ */ jsx(Button, { ...args }) })
};
const Destructive = {
  args: { variant: "destructive", children: "Kill job" },
  render: (args) => /* @__PURE__ */ jsx(Card$7, { children: /* @__PURE__ */ jsx(Button, { ...args }) })
};
const Hover = {
  render: () => /* @__PURE__ */ jsxs(Card$7, { children: [
    /* @__PURE__ */ jsx(Button, { variant: "primary", "data-preview-hover": "", children: "Dispatch job" }),
    /* @__PURE__ */ jsx(Button, { variant: "secondary", "data-preview-hover": "", children: "Cancel" }),
    /* @__PURE__ */ jsx(Button, { variant: "ghost", "data-preview-hover": "", children: "Ghost" }),
    /* @__PURE__ */ jsx(Button, { variant: "destructive", "data-preview-hover": "", children: "Kill job" })
  ] })
};
const Focused$7 = {
  render: () => /* @__PURE__ */ jsx(Card$7, { children: /* @__PURE__ */ jsx(Button, { variant: "secondary", "data-preview-focus": "", children: "Focused" }) })
};
const Disabled$7 = {
  render: () => /* @__PURE__ */ jsxs(Card$7, { children: [
    /* @__PURE__ */ jsx(Button, { variant: "primary", disabled: true, children: "Dispatch job" }),
    /* @__PURE__ */ jsx(Button, { variant: "secondary", disabled: true, children: "Cancel" }),
    /* @__PURE__ */ jsx(Button, { variant: "destructive", disabled: true, children: "Kill job" })
  ] })
};
const Small = {
  render: () => /* @__PURE__ */ jsxs(Card$7, { children: [
    /* @__PURE__ */ jsx(Button, { size: "sm", children: "Review" }),
    /* @__PURE__ */ jsx(Button, { size: "sm", children: "Open diff" }),
    /* @__PURE__ */ jsx(Button, { size: "sm", variant: "ghost", iconOnly: true, "aria-label": "Retry", children: /* @__PURE__ */ jsx(RotateCw, { size: 16, strokeWidth: 2, "aria-hidden": "true" }) })
  ] })
};
const Group = {
  render: () => /* @__PURE__ */ jsxs(Card$7, { children: [
    /* @__PURE__ */ jsx(Button, { variant: "primary", children: "Approve" }),
    /* @__PURE__ */ jsx(Button, { variant: "secondary", children: "Open diff" }),
    /* @__PURE__ */ jsx(Button, { variant: "ghost", children: "Redirect" }),
    /* @__PURE__ */ jsx(Button, { variant: "destructive", children: "Kill" })
  ] })
};
const SecondaryOnASunkenGround = {
  render: () => /* @__PURE__ */ jsxs("div", { style: { display: "flex", flexDirection: "column", gap: "var(--space-4)" }, children: [
    /* @__PURE__ */ jsx(Card$7, { children: /* @__PURE__ */ jsx(Button, { variant: "secondary", ground: "card", children: "On a card" }) }),
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
  render: () => /* @__PURE__ */ jsx("div", { "data-theme": "light", children: /* @__PURE__ */ jsxs(Card$7, { children: [
    /* @__PURE__ */ jsx(Button, { variant: "primary", children: "Dispatch job" }),
    /* @__PURE__ */ jsx(Button, { variant: "secondary", children: "Cancel" })
  ] }) })
};
const __vite_glob_0_11 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
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
  default: meta$t
}, Symbol.toStringTag, { value: "Module" }));
function joined$1(base, extra) {
  return extra ? `${base} ${extra}` : base;
}
function Card$6({ className, ...rest }) {
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
const meta$s = {
  title: "Primitives/Card",
  component: Card$6,
  decorators: [
    (Story) => /* @__PURE__ */ jsx("div", { style: { maxWidth: "56ch" }, children: /* @__PURE__ */ jsx(Story, {}) })
  ]
};
const Default$6 = {
  render: () => /* @__PURE__ */ jsxs(Card$6, { children: [
    /* @__PURE__ */ jsx(CardTitle, { children: "Evidence accepted at step 4 of 5" }),
    /* @__PURE__ */ jsx(CardDescription, { children: "Four criteria resolved. One test added, none removed." })
  ] })
};
const WithHeader = {
  render: () => /* @__PURE__ */ jsxs(Card$6, { children: [
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
  render: () => /* @__PURE__ */ jsxs(Card$6, { "data-dimmed": true, children: [
    /* @__PURE__ */ jsx(CardTitle, { children: "Superseded at step 2 of 5" }),
    /* @__PURE__ */ jsx(CardDescription, { children: "The work landed outside this job." })
  ] })
};
const __vite_glob_0_12 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Default: Default$6,
  Dimmed: Dimmed$1,
  WithHeader,
  default: meta$s
}, Symbol.toStringTag, { value: "Module" }));
function Checkbox({ children, ...rest }) {
  return /* @__PURE__ */ jsxs("label", { className: "armada-checkbox", children: [
    /* @__PURE__ */ jsx("input", { ...rest, type: "checkbox", className: "armada-checkbox__input" }),
    /* @__PURE__ */ jsx("span", { className: "armada-checkbox__box", "aria-hidden": "true", children: /* @__PURE__ */ jsx(Check, { size: 12, strokeWidth: 2 }) }),
    /* @__PURE__ */ jsx("span", { children })
  ] });
}
const meta$r = {
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
const __vite_glob_0_13 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Checked,
  Disabled: Disabled$6,
  Focused: Focused$6,
  Light: Light$6,
  Unchecked,
  default: meta$r
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
        const heading = entry.section !== lastSection ? entry.section : "";
        lastSection = entry.section;
        return /* @__PURE__ */ jsxs("div", { children: [
          heading ? /* @__PURE__ */ jsx("div", { className: "armada-palette__section", children: heading }) : null,
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
function Dialog({
  open: open2,
  title,
  children,
  tone = "destructive",
  confirmLabel,
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
        onConfirm?.();
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open2, onCancel, onConfirm]);
  if (!open2) return null;
  const destructive = tone === "destructive";
  return /* @__PURE__ */ jsx("div", { className: "armada-dialog-scrim", children: /* @__PURE__ */ jsxs("div", { className: "armada-dialog", role: "dialog", "aria-modal": "true", "aria-label": title, children: [
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
      /* @__PURE__ */ jsxs("div", { className: "armada-dialog__copy", children: [
        /* @__PURE__ */ jsx("h2", { className: "armada-dialog__title", children: title }),
        /* @__PURE__ */ jsx("div", { className: "armada-dialog__body", children })
      ] })
    ] }),
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
          onClick: onConfirm,
          children: confirmLabel
        }
      )
    ] })
  ] }) });
}
const meta$q = {
  title: "Primitives/CommandPalette",
  component: CommandPalette
};
const glyph = { size: 12, strokeWidth: 2, "aria-hidden": true };
const entries$1 = [
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
  args: { open: true, entries: entries$1 }
};
const AliasFindsTheLexiconTerm = {
  args: { open: true, entries: entries$1, defaultQuery: "terminate" }
};
const NoMatch = {
  args: { open: true, entries: entries$1, defaultQuery: "zzz" }
};
const DestructiveEntryConfirms = {
  render: () => {
    const [pending, setPending] = useState(void 0);
    return /* @__PURE__ */ jsxs(Fragment, { children: [
      /* @__PURE__ */ jsx(
        CommandPalette,
        {
          open: true,
          entries: entries$1,
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
const __vite_glob_0_14 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AliasFindsTheLexiconTerm,
  DestructiveEntryConfirms,
  NoMatch,
  Resting: Resting$1,
  default: meta$q
}, Symbol.toStringTag, { value: "Module" }));
const meta$p = {
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
const __vite_glob_0_15 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Confirmation,
  NeutralConfirm,
  default: meta$p
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
const meta$o = {
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
const __vite_glob_0_16 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AtTheLeftEdge: AtTheLeftEdge$2,
  AtTheRightEdge: AtTheRightEdge$2,
  RowMenu,
  WithNoRoomBelow: WithNoRoomBelow$2,
  WithSectionLabels,
  default: meta$o
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
const meta$n = {
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
const __vite_glob_0_17 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Default: Default$5,
  Disabled: Disabled$5,
  Focused: Focused$5,
  Invalid: Invalid$2,
  Light: Light$5,
  Mono,
  Placeholder: Placeholder$1,
  default: meta$n
}, Symbol.toStringTag, { value: "Module" }));
function Kbd({ className, ...rest }) {
  return /* @__PURE__ */ jsx("kbd", { className: className ? `armada-kbd ${className}` : "armada-kbd", ...rest });
}
function KbdChord({ className, ...rest }) {
  return /* @__PURE__ */ jsx("span", { className: className ? `armada-kbd-chord ${className}` : "armada-kbd-chord", ...rest });
}
const meta$m = {
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
const __vite_glob_0_18 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Chord,
  ContextualKeys,
  Default: Default$4,
  default: meta$m
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
const meta$l = {
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
const __vite_glob_0_19 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AlignedToTheEnd,
  AtTheLeftEdge: AtTheLeftEdge$1,
  AtTheRightEdge: AtTheRightEdge$1,
  Open: Open$1,
  WithNoRoomBelow: WithNoRoomBelow$1,
  default: meta$l
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
const meta$k = {
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
const __vite_glob_0_20 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Default: Default$3,
  Disabled: Disabled$4,
  Focused: Focused$4,
  Light: Light$4,
  default: meta$k
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
const meta$j = {
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
const __vite_glob_0_21 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Scrolling,
  WithinBounds,
  default: meta$j
}, Symbol.toStringTag, { value: "Module" }));
const meta$i = {
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
const __vite_glob_0_22 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Default: Default$2,
  Disabled: Disabled$3,
  Focused: Focused$3,
  Invalid: Invalid$1,
  Light: Light$3,
  default: meta$i
}, Symbol.toStringTag, { value: "Module" }));
const meta$h = {
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
const __vite_glob_0_23 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Announced,
  Horizontal,
  Vertical,
  default: meta$h
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
const meta$g = {
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
const __vite_glob_0_24 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Left,
  Right,
  default: meta$g
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
const meta$f = {
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
  render: () => /* @__PURE__ */ jsxs(Card$6, { children: [
    /* @__PURE__ */ jsx(CardTitle, { children: "Evidence" }),
    /* @__PURE__ */ jsx(SkeletonText, { label: "Loading evidence" })
  ] })
};
const __vite_glob_0_25 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  InACard,
  Single,
  Text,
  default: meta$f
}, Symbol.toStringTag, { value: "Module" }));
const meta$e = {
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
const __vite_glob_0_26 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Closed,
  Disabled: Disabled$2,
  EscalatedRow,
  Focused: Focused$2,
  FocusedOnPrimary,
  Light: Light$2,
  Open,
  PrimaryOnJobDetail,
  default: meta$e
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
const meta$d = {
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
const __vite_glob_0_27 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Disabled: Disabled$1,
  Focused: Focused$1,
  Light: Light$1,
  Off,
  On,
  WithADescription,
  default: meta$d
}, Symbol.toStringTag, { value: "Module" }));
const meta$c = {
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
const __vite_glob_0_28 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Default: Default$1,
  Dimmed,
  FocusedAndSelected,
  MonoValuesCopy,
  RowsGrowWithContent,
  default: meta$c
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
const meta$b = {
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
const __vite_glob_0_29 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  LastActive,
  SectionsOfOneObject,
  default: meta$b
}, Symbol.toStringTag, { value: "Module" }));
function TabsWithCounts({ items, value, defaultValue, onChange }) {
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
  return /* @__PURE__ */ jsx("div", { className: "armada-tabs-counts", role: "tablist", onKeyDown: onKey, children: items.map((item) => /* @__PURE__ */ jsxs(
    "button",
    {
      type: "button",
      role: "tab",
      "aria-selected": item.id === active,
      tabIndex: item.id === active ? 0 : -1,
      className: item.id === active ? "armada-tabs-counts__tab armada-tabs-counts__tab--active" : "armada-tabs-counts__tab",
      onClick: () => select(item.id),
      children: [
        item.label,
        item.count ? /* @__PURE__ */ jsx("span", { className: "armada-tabs-counts__count", children: item.count }) : null
      ]
    },
    item.id
  )) });
}
const meta$a = {
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
const __vite_glob_0_30 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Queues,
  Zero,
  default: meta$a
}, Symbol.toStringTag, { value: "Module" }));
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
const meta$9 = {
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
const __vite_glob_0_31 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Default,
  Disabled,
  Focused,
  Invalid,
  Light,
  Overflowing,
  Placeholder,
  Rows,
  default: meta$9
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
const meta$8 = {
  title: "Primitives/Toast",
  component: Toast
};
const Copied = {
  args: {
    children: "Copied job_8f2a1c."
  }
};
const Killed = {
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
const __vite_glob_0_32 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Copied,
  Killed,
  Landed,
  default: meta$8
}, Symbol.toStringTag, { value: "Module" }));
const meta$7 = {
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
const __vite_glob_0_33 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AtTheLeftEdge,
  AtTheRightEdge,
  Resting,
  TruncatedValue,
  WithNoRoomBelow,
  WithShortcut,
  default: meta$7
}, Symbol.toStringTag, { value: "Module" }));
const meta$6 = {
  title: "Screens/A failed job — a dead end, read as one"
};
const NO_GLYPH_IN_REGISTRY$2 = void 0;
const steps$1 = [
  {
    id: "plan",
    label: "Plan the change",
    activity: "advanced",
    status: "advanced",
    // The drawing draws no row under Plan the change here. The rail always
    // draws one. Reported.
    evidence: { icon: NO_GLYPH_IN_REGISTRY$2, iconLabel: "Evidence", label: "" }
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
    evidence: { icon: NO_GLYPH_IN_REGISTRY$2, iconLabel: "Evidence", label: "" }
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
const FailedJob = {
  render: () => /* @__PURE__ */ jsxs("div", { className: "armada-screen", children: [
    /* @__PURE__ */ jsxs("div", { className: "armada-screen__detail", children: [
      /* @__PURE__ */ jsxs("div", { className: "armada-screen__ident", children: [
        /* @__PURE__ */ jsxs("div", { className: "armada-screen__ident-line", children: [
          /* @__PURE__ */ jsx(Badge, { status: "completed-failed", icon: X, children: "Failed" }),
          /* @__PURE__ */ jsx("span", { className: "armada-screen__title", children: "Cache the manifest read" }),
          /* @__PURE__ */ jsx("span", { className: "armada-screen__job-id", children: "job_91ab" })
        ] }),
        /* @__PURE__ */ jsxs("div", { className: "armada-screen__meta", children: [
          /* @__PURE__ */ jsxs("span", { children: [
            "Stopped at ",
            /* @__PURE__ */ jsx("span", { className: "armada-screen__value", "data-sans": true, children: "Run tests" }),
            ", step ",
            /* @__PURE__ */ jsx("span", { className: "armada-screen__value", children: "3 of 4" })
          ] }),
          /* @__PURE__ */ jsxs("span", { children: [
            "Ran ",
            /* @__PURE__ */ jsx("span", { className: "armada-screen__value", children: "22m 41s" })
          ] }),
          /* @__PURE__ */ jsxs("span", { children: [
            "Spend, estimated ",
            /* @__PURE__ */ jsx("span", { className: "armada-screen__value", children: "~$2.10" })
          ] }),
          /* @__PURE__ */ jsx("span", { children: "Dispatched by you" })
        ] })
      ] }),
      /* @__PURE__ */ jsxs("div", { className: "armada-screen__sunken", children: [
        /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", children: "Why this stopped" }),
        /* @__PURE__ */ jsxs("p", { className: "armada-screen__why", children: [
          "The test check exited 1 at Run tests, on 2 assertions in",
          " ",
          /* @__PURE__ */ jsx("span", { className: "armada-screen__mono", children: "core/manifest" }),
          ". The job is over. Nothing runs from here without you."
        ] })
      ] }),
      /* @__PURE__ */ jsxs("div", { className: "armada-screen__split", "data-wide": true, children: [
        /* @__PURE__ */ jsxs("div", { className: "armada-screen__col", children: [
          /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", children: "What ran" }),
          /* @__PURE__ */ jsx(WorkflowRail, { steps: steps$1 }),
          /* @__PURE__ */ jsxs("span", { className: "armada-screen__caption", "data-muted": true, children: [
            "The failed step is hued and carries a surface, so the row that ended the Job stays findable while the Check output is read beside it. The gate rows stay neutral — the step’s state is hued, a Check’s exit code is measured. ",
            /* @__PURE__ */ jsx("span", { className: "armada-screen__mono", children: "Implement" }),
            " carries two checks and both had to pass:",
            " ",
            /* @__PURE__ */ jsx("span", { className: "armada-screen__mono", children: "cargo build" }),
            " succeeds on an empty diff, so the build alone would advance a drone that did nothing."
          ] })
        ] }),
        /* @__PURE__ */ jsxs("div", { className: "armada-screen__col", "data-loose": true, children: [
          /* @__PURE__ */ jsxs("div", { className: "armada-screen__col", children: [
            /* @__PURE__ */ jsxs("div", { className: "armada-screen__head-row", children: [
              /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", children: "Check output" }),
              /* @__PURE__ */ jsx("span", { className: "armada-screen__tag", children: "exit 1 · 4.2s · tail 12 lines" })
            ] }),
            /* @__PURE__ */ jsx("pre", { className: "armada-screen__output", children: tail }),
            /* @__PURE__ */ jsx("span", { className: "armada-screen__caption", "data-muted": true, children: "Readable without opening a log file. Mono throughout, because every line of it is machine output, and the tail is bounded with the full capture one click away." })
          ] }),
          /* @__PURE__ */ jsxs("div", { className: "armada-screen__col", children: [
            /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", children: "Where the work is" }),
            /* @__PURE__ */ jsx(
              JobLogReference,
              {
                rows: [
                  {
                    icon: GitBranch,
                    iconLabel: "Branch",
                    value: "feat/manifest-cache",
                    copyValue: "feat/manifest-cache",
                    meta: "2 files +48 −11"
                  },
                  // `folder` means "workspace" in the registry. A worktree is
                  // not a workspace, and the registry has no row for one.
                  // Reported.
                  {
                    icon: Folder,
                    iconLabel: "Worktree",
                    value: "~/.armada/worktrees/job_91ab"
                  },
                  {
                    icon: NO_GLYPH_IN_REGISTRY$2,
                    iconLabel: "Log",
                    value: ".armada/logs/job_91ab.jsonl",
                    copyValue: ".armada/logs/job_91ab.jsonl",
                    meta: "318 lines · 4 error",
                    separated: true
                  }
                ],
                children: "The worktree and the branch are left in place. Armada will not touch either. The log holds Fleet, the drone and Bridge in one order, keyed on this job."
              }
            ),
            /* @__PURE__ */ jsxs("div", { className: "armada-screen__actions", children: [
              /* @__PURE__ */ jsx(Button, { children: "Open the log" }),
              /* @__PURE__ */ jsx(Button, { children: "Open the worktree" })
            ] })
          ] })
        ] })
      ] })
    ] }),
    /* @__PURE__ */ jsxs("p", { className: "armada-screen__note", children: [
      /* @__PURE__ */ jsx("span", { className: "armada-screen__strong", children: "No retry, no category, no suggestion." }),
      " ",
      "All three controls take you to the work; none offers to do anything about it. The screen states four things in order — what failed, that the job is over, where the branch is, and where the log is — and the sentence saying nothing happens automatically is written out rather than left to be inferred from an absence of buttons.",
      " ",
      /* @__PURE__ */ jsx("span", { className: "armada-screen__strong", children: "The per-job log is named here because M1 is when everything is broken." }),
      " ",
      "The Check output pane answers what the suite said; the log answers what Fleet, the drone and Bridge each did, joined on",
      " ",
      /* @__PURE__ */ jsx("span", { className: "armada-screen__mono", children: "job_id" }),
      ". Showing the path and the error count means a person can reach it without knowing the sink layout, and it costs one row and one button rather than a viewer."
    ] })
  ] })
};
const __vite_glob_0_34 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  FailedJob,
  default: meta$6
}, Symbol.toStringTag, { value: "Module" }));
const meta$5 = {
  title: "Screens/A finished job — a branch and an evidence trail"
};
const NO_GLYPH_IN_REGISTRY$1 = void 0;
const BRANCH_ICON = 16;
const BRANCH_STROKE = 2;
const PX$1 = "px";
const entries = [
  {
    step: "Plan the change",
    provenance: "14:02 · facts_note · no check",
    icon: NO_GLYPH_IN_REGISTRY$1,
    iconLabel: "Evidence",
    claimed: "The poke loop stops after 3 attempts and the job records how many it spent.",
    shownBy: "core/fleet/src/lease.rs · the loop has no ceiling today",
    notClaimed: "Does not change the poke interval, and does not decide what happens at the third failure."
  },
  {
    step: "Implement",
    provenance: "14:11 · diff · build exit 0 · diff_nonempty passed",
    icon: NO_GLYPH_IN_REGISTRY$1,
    iconLabel: "Evidence",
    claimed: "A drone that stops answering is poked at most 3 times, and the count is on the job record.",
    shownBy: "core/fleet/src/lease.rs +38 −7 · core/model/src/job.rs +14 −0",
    notClaimed: "The count is not surfaced in Bridge. Nothing acts on reaching the ceiling yet — the loop exits and the job keeps its status."
  },
  {
    step: "Run tests",
    provenance: "14:16 · test_suite_run · test exit 0",
    icon: NO_GLYPH_IN_REGISTRY$1,
    iconLabel: "Evidence",
    claimed: "The ceiling holds at 3 and the counter increments once per poke.",
    shownBy: "cargo test --workspace · 86 passed 0 failed 5.1s · lease::poke_ceiling_holds, lease::poke_count_increments",
    notClaimed: "No test covers a drone that answers on the third poke. The suite was green before this change and is green after, so it does not prove the ceiling is reached in practice."
  },
  {
    step: "Summarise",
    provenance: "14:20 · facts_note · no check",
    icon: NO_GLYPH_IN_REGISTRY$1,
    iconLabel: "Evidence",
    claimed: "The change is on fix/poke-ceiling and ready to read.",
    shownBy: "3 files +214 −96 · branch fix/poke-ceiling",
    notClaimed: "The value 3 is a constant rather than config. Whether it is the right number is not established by anything here."
  }
];
const FinishedJob = {
  render: () => /* @__PURE__ */ jsxs("div", { className: "armada-screen", children: [
    /* @__PURE__ */ jsxs("div", { className: "armada-screen__detail", children: [
      /* @__PURE__ */ jsxs("div", { className: "armada-screen__ident", children: [
        /* @__PURE__ */ jsxs("div", { className: "armada-screen__ident-line", children: [
          /* @__PURE__ */ jsx(Badge, { status: "completed-success", icon: Check, children: "Done" }),
          /* @__PURE__ */ jsx("span", { className: "armada-screen__title", children: "Add a retry ceiling to the poke loop" }),
          /* @__PURE__ */ jsx("span", { className: "armada-screen__job-id", children: "job_4f10" })
        ] }),
        /* @__PURE__ */ jsxs("div", { className: "armada-screen__meta", children: [
          /* @__PURE__ */ jsxs("span", { children: [
            "All ",
            /* @__PURE__ */ jsx("span", { className: "armada-screen__value", children: "4 of 4" }),
            " steps advanced"
          ] }),
          /* @__PURE__ */ jsxs("span", { children: [
            "Ran ",
            /* @__PURE__ */ jsx("span", { className: "armada-screen__value", children: "18m 22s" })
          ] }),
          /* @__PURE__ */ jsxs("span", { children: [
            "Spend, estimated ",
            /* @__PURE__ */ jsx("span", { className: "armada-screen__value", children: "~$2.40" })
          ] }),
          /* @__PURE__ */ jsx("span", { children: "Dispatched by you" })
        ] })
      ] }),
      /* @__PURE__ */ jsxs("div", { className: "armada-screen__sunken", children: [
        /* @__PURE__ */ jsxs("div", { className: "armada-screen__branch-line", children: [
          /* @__PURE__ */ jsx("span", { className: "armada-screen__mark", children: /* @__PURE__ */ jsx(GitBranch, { size: BRANCH_ICON, strokeWidth: BRANCH_STROKE, "aria-hidden": true }) }),
          /* @__PURE__ */ jsx("span", { className: "armada-screen__branch", children: "fix/poke-ceiling" }),
          /* @__PURE__ */ jsx("span", { className: "armada-screen__tag", children: "from main · 3 files +214 −96" }),
          /* @__PURE__ */ jsx("div", { className: "armada-screen__push-right", children: /* @__PURE__ */ jsx(Button, { ground: "sunken", children: "Open the worktree" }) })
        ] }),
        /* @__PURE__ */ jsxs("div", { className: "armada-screen__log-line", children: [
          /* @__PURE__ */ jsx("span", { className: "armada-screen__mark", "aria-hidden": true }),
          /* @__PURE__ */ jsx("span", { className: "armada-screen__log-path", children: ".armada/logs/job_4f10.jsonl" }),
          /* @__PURE__ */ jsx("span", { className: "armada-screen__tag", children: "204 lines · 0 error" }),
          /* @__PURE__ */ jsx("div", { className: "armada-screen__push-right", children: /* @__PURE__ */ jsx(Button, { ground: "sunken", children: "Open the log" }) })
        ] }),
        /* @__PURE__ */ jsx("span", { className: "armada-screen__caption", "data-muted": true, children: "The branch is unpushed and unmerged. Armada does not push and has no merge action — read the diff in your own tools and land it yourself." })
      ] }),
      /* @__PURE__ */ jsxs("div", { className: "armada-screen__col", children: [
        /* @__PURE__ */ jsxs("div", { className: "armada-screen__head-row", children: [
          /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", children: "Evidence" }),
          /* @__PURE__ */ jsx("span", { className: "armada-screen__tag", children: "4 submissions · in order" })
        ] }),
        /* @__PURE__ */ jsx(EvidenceTrail, { entries }),
        /* @__PURE__ */ jsxs("span", { className: "armada-screen__caption", "data-muted": true, children: [
          "One entry per step in submission order, each with its",
          " ",
          /* @__PURE__ */ jsx("span", { className: "armada-screen__mono", children: "evidence_type" }),
          " and the Checks that let it pass.",
          " ",
          /* @__PURE__ */ jsx("span", { className: "armada-screen__strong", children: "The three fields are the schema’s, not a layout choice." }),
          " ",
          "A work submission carries ",
          /* @__PURE__ */ jsx("span", { className: "armada-screen__mono", children: "claimed" }),
          " — what the work now does, as an observable —",
          " ",
          /* @__PURE__ */ jsx("span", { className: "armada-screen__mono", children: "shown_by" }),
          ", the artifact demonstrating it, and ",
          /* @__PURE__ */ jsx("span", { className: "armada-screen__mono", children: "not_claimed" }),
          ", which is required and may be empty. It always renders, and an empty one reads",
          " ",
          /* @__PURE__ */ jsx("em", { children: "Nothing" }),
          " — a dash would read as no answer, which is the reading the field exists to rule out. Rendering them as a paragraph would let a drone report in prose, which is the failure this milestone is watching for. The trail is the reason to open this screen, so it is the largest element rather than a panel to expand."
        ] })
      ] })
    ] }),
    /* @__PURE__ */ jsxs("p", { className: "armada-screen__note", children: [
      /* @__PURE__ */ jsx("span", { className: "armada-screen__strong", children: "The screen hands over a branch name and gets out of the way." }),
      " ",
      "No approve, no reject, no merge, and no in-app diff — the two controls copy a value and open a directory. Both are ",
      `36${PX$1}`,
      " and neither is filled, because the accent belongs to Approve and dispatch and there is no decision to make here that Armada participates in."
    ] })
  ] })
};
const __vite_glob_0_35 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  FinishedJob,
  default: meta$5
}, Symbol.toStringTag, { value: "Module" }));
function Absent({ name, note }) {
  return /* @__PURE__ */ jsxs("div", { className: "armada-screen-absent", role: "note", children: [
    /* @__PURE__ */ jsx("span", { className: "armada-screen-absent__name", children: name }),
    /* @__PURE__ */ jsx("span", { className: "armada-screen-absent__why", children: note })
  ] });
}
const meta$4 = {
  title: "Screens/A running job"
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
    ]
  },
  {
    id: "handoff",
    label: "Summarise",
    activity: "not_started",
    status: "not started",
    // The drawing draws no row under Summarise. The rail always draws one,
    // because a blank would read as a gate that failed to render. Reported.
    evidence: { icon: NO_GLYPH_IN_REGISTRY, iconLabel: "Evidence", label: "" }
  }
];
const labelled = [
  { id: "plan", label: "Plan the change", activity: "advanced" },
  { id: "implement", label: "Implement", activity: "running", current: true },
  { id: "verify", label: "Run tests", activity: "not_started" },
  { id: "handoff", label: "Summarise", activity: "not_started" }
];
const identifiers = [
  { id: "plan", label: "plan", labelIsAnIdentifier: true, activity: "advanced" },
  { id: "implement", label: "implement", labelIsAnIdentifier: true, activity: "running", current: true },
  { id: "verify", label: "verify", labelIsAnIdentifier: true, activity: "not_started" },
  { id: "handoff", label: "handoff", labelIsAnIdentifier: true, activity: "not_started" }
];
const RunningJob = {
  render: () => /* @__PURE__ */ jsxs("div", { className: "armada-screen", children: [
    /* @__PURE__ */ jsxs("div", { className: "armada-screen__detail", children: [
      /* @__PURE__ */ jsx(
        Absent,
        {
          name: "Job detail header actions",
          note: "Holds the Running badge, static, beside the title “Split the settings reducer” and job_2d90bb; then Step 2 of 4 · Branch fix/settings-split · Elapsed 11m 03s · Spend, estimated ~$1.80 · Dispatched by you; and Kill at the trailing edge, outlined in --status-completed-failed and never filled."
        }
      ),
      /* @__PURE__ */ jsxs("div", { className: "armada-screen__split", children: [
        /* @__PURE__ */ jsxs("div", { className: "armada-screen__col", children: [
          /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", children: "What ran" }),
          /* @__PURE__ */ jsx(WorkflowRail, { steps, pulsing: true }),
          /* @__PURE__ */ jsxs("span", { className: "armada-screen__caption", "data-muted": true, children: [
            "Step names are sans nouns; the Check under each is mono, because a Check is a command. A step carries",
            " ",
            /* @__PURE__ */ jsx("span", { className: "armada-screen__mono", children: "mechanical_checks[]" }),
            " and every entry must pass, which is why ",
            /* @__PURE__ */ jsx("span", { className: "armada-screen__mono", children: "Implement" }),
            " ",
            "shows two rows. An ungated step says so in words rather than leaving a gap where a gate row would be — two of the four have no check at all, so a blank would read as a gate that failed to render."
          ] })
        ] }),
        /* @__PURE__ */ jsxs("div", { className: "armada-screen__col", children: [
          /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", children: "Evidence so far" }),
          /* @__PURE__ */ jsx("div", { className: "armada-screen__slot", children: /* @__PURE__ */ jsx(
            Absent,
            {
              name: "Evidence card",
              note: "Holds one submission — Plan the change, 09:14 — on the three fields the Evidence MCP tool requires: Claimed, Shown by, Not claimed. Plan the change is facts_note, so shown_by points at files rather than a command."
            }
          ) }),
          /* @__PURE__ */ jsxs("span", { className: "armada-screen__caption", "data-muted": true, children: [
            "One entry per step, in order, on the three fields the Evidence MCP tool requires. It is the only record of what the drone claimed, and whether it is worth reading is the finding this milestone is for.",
            " ",
            /* @__PURE__ */ jsx("span", { className: "armada-screen__mono", children: "Plan the change" }),
            " is",
            " ",
            /* @__PURE__ */ jsx("span", { className: "armada-screen__mono", children: "facts_note" }),
            ", so",
            " ",
            /* @__PURE__ */ jsx("span", { className: "armada-screen__mono", children: "shown_by" }),
            " points at files rather than a command."
          ] }),
          /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", "data-spaced": true, children: "Log" }),
          /* @__PURE__ */ jsx(
            JobLogReference,
            {
              rows: [
                {
                  icon: NO_GLYPH_IN_REGISTRY,
                  iconLabel: "Log",
                  value: ".armada/logs/job_2d90bb.jsonl",
                  copyValue: ".armada/logs/job_2d90bb.jsonl",
                  meta: "142 lines · 0 error"
                }
              ],
              actions: /* @__PURE__ */ jsx(Button, { ground: "sunken", size: "sm", children: "Open the log" }),
              children: "Fleet, the drone and Bridge in one order, keyed on this job. It is being written now."
            }
          )
        ] })
      ] })
    ] }),
    /* @__PURE__ */ jsxs("div", { className: "armada-screen__row", children: [
      /* @__PURE__ */ jsxs("div", { className: "armada-screen__card", "data-width": "rail", children: [
        /* @__PURE__ */ jsx("span", { className: "armada-screen__caption", children: "The rail with labels, as drawn above" }),
        /* @__PURE__ */ jsx(WorkflowRail, { steps: labelled }),
        /* @__PURE__ */ jsxs("span", { className: "armada-screen__caption", "data-muted": true, children: [
          "Nouns naming the artifact, matching the settled rule that step names are labels in sans on every surface.",
          " ",
          /* @__PURE__ */ jsx("span", { className: "armada-screen__mono", children: "WorkflowDef.steps[].label" }),
          " is a required field, so the schema is not the gap — these four values are, and I proposed them."
        ] })
      ] }),
      /* @__PURE__ */ jsxs("div", { className: "armada-screen__card", "data-dim": true, "data-width": "rail", children: [
        /* @__PURE__ */ jsx("span", { className: "armada-screen__caption", children: "The same rail with the ids alone, for contrast" }),
        /* @__PURE__ */ jsx(WorkflowRail, { steps: identifiers }),
        /* @__PURE__ */ jsxs("span", { className: "armada-screen__caption", children: [
          "Honest, and useless to scan: four schema identifiers on the surface a person watches. This is what M1 renders if the four",
          " ",
          /* @__PURE__ */ jsx("span", { className: "armada-screen__mono", children: "label" }),
          " values are not written, which is why the field being required is not enough on its own."
        ] })
      ] })
    ] })
  ] })
};
const __vite_glob_0_36 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  RunningJob,
  default: meta$4
}, Symbol.toStringTag, { value: "Module" }));
const meta$3 = {
  title: "Screens/Dispatch a job — full, with the M1 subset marked"
};
const Dispatch = {
  render: () => /* @__PURE__ */ jsxs("div", { className: "armada-screen", children: [
    /* @__PURE__ */ jsxs("div", { className: "armada-screen__legend", children: [
      /* @__PURE__ */ jsxs("div", { className: "armada-screen__legend-line", children: [
        /* @__PURE__ */ jsx("span", { className: "armada-screen__swatch" }),
        /* @__PURE__ */ jsx("span", { className: "armada-screen__strong", children: "M1 renders this" })
      ] }),
      /* @__PURE__ */ jsxs("div", { className: "armada-screen__legend-line", children: [
        /* @__PURE__ */ jsx("span", { className: "armada-screen__swatch", "data-dim": true }),
        /* @__PURE__ */ jsxs("span", { children: [
          "Designed, not built at M1 — dimmed to",
          " ",
          /* @__PURE__ */ jsx("span", { className: "armada-screen__mono", children: "--border-subtle" }),
          " and",
          " ",
          /* @__PURE__ */ jsx("span", { className: "armada-screen__mono", children: "--fg-subtle" }),
          ", the same dimming a de-emphasised row takes"
        ] })
      ] })
    ] }),
    /* @__PURE__ */ jsxs("div", { className: "armada-screen__row", children: [
      /* @__PURE__ */ jsxs("div", { className: "armada-screen__card", "data-dim": true, "data-width": "narrow", children: [
        /* @__PURE__ */ jsxs("div", { className: "armada-screen__card-head", children: [
          /* @__PURE__ */ jsx("span", { className: "armada-screen__caption", children: "1. Browse the Job Board" }),
          /* @__PURE__ */ jsx("span", { className: "armada-screen__tag", "data-dim": true, children: "not at M1" })
        ] }),
        /* @__PURE__ */ jsxs("div", { className: "armada-screen__queue", children: [
          /* @__PURE__ */ jsxs("div", { className: "armada-screen__queue-head", children: [
            /* @__PURE__ */ jsx("span", { children: "Ready" }),
            /* @__PURE__ */ jsx("span", { className: "armada-screen__mono", children: "3" })
          ] }),
          /* @__PURE__ */ jsxs("div", { className: "armada-screen__queue-item", children: [
            /* @__PURE__ */ jsx("span", { children: "Coalesce concurrent token refreshes" }),
            /* @__PURE__ */ jsx("span", { children: "api/auth · found by Fleet" })
          ] }),
          /* @__PURE__ */ jsxs("div", { className: "armada-screen__queue-item", children: [
            /* @__PURE__ */ jsx("span", { children: "Retire the legacy poke path" }),
            /* @__PURE__ */ jsx("span", { children: "core/fleet · drafted in Helm" })
          ] }),
          /* @__PURE__ */ jsxs("div", { className: "armada-screen__queue-item", children: [
            /* @__PURE__ */ jsx("span", { children: "Cache the manifest read" }),
            /* @__PURE__ */ jsx("span", { children: "core/manifest · dispatched by you" })
          ] })
        ] }),
        /* @__PURE__ */ jsx("span", { className: "armada-screen__caption", children: "The queue, its origin tags, the list-and-graph toggle and the scope picker. M1 has no queue: a job is created and immediately waiting on you, so there is nothing to browse." })
      ] }),
      /* @__PURE__ */ jsxs("div", { className: "armada-screen__card", "data-bright": true, "data-width": "card", children: [
        /* @__PURE__ */ jsxs("div", { className: "armada-screen__card-head", children: [
          /* @__PURE__ */ jsx("span", { className: "armada-screen__caption", children: "2. The approval card — the full design" }),
          /* @__PURE__ */ jsx("span", { className: "armada-screen__tag", children: "reduced at M1" })
        ] }),
        /* @__PURE__ */ jsx("div", { className: "armada-screen__slot", children: /* @__PURE__ */ jsx(
          Absent,
          {
            name: "Approval card",
            note: "Holds the title “Coalesce concurrent token refreshes”, its brief, then the three glance fields the card exists for — Diff size ~4 files, Job type feature, Cost, estimated ~$3.20 of $20 — then Workflow bug, 4 steps · Workspace armada · Criteria 4, then Cancel beside Approve and dispatch."
          }
        ) }),
        /* @__PURE__ */ jsx("span", { className: "armada-screen__caption", "data-muted": true, children: "The three glance fields are the whole point of the card: diff size, job type and estimated cost have to be read before the tap registers. Criteria is the one row M1 drops, because there is no Judge to hold them." })
      ] }),
      /* @__PURE__ */ jsxs("div", { className: "armada-screen__card", "data-bright": true, "data-width": "card", children: [
        /* @__PURE__ */ jsxs("div", { className: "armada-screen__card-head", children: [
          /* @__PURE__ */ jsx("span", { className: "armada-screen__caption", children: "3. What M1 renders" }),
          /* @__PURE__ */ jsx("span", { className: "armada-screen__tag", children: "M1" })
        ] }),
        /* @__PURE__ */ jsx("div", { className: "armada-screen__slot", children: /* @__PURE__ */ jsx(
          Absent,
          {
            name: "Job composer",
            note: "Holds a Title input, a Brief textarea, a Workflow select reading “bug — 4 steps” beside a read-only Project armada, then the two-up glance strip Steps 4 · 2 gated and Checks build, test, then Cancel beside Approve and dispatch — the one accent fill in the whole milestone."
          }
        ) }),
        /* @__PURE__ */ jsxs("span", { className: "armada-screen__caption", "data-muted": true, children: [
          /* @__PURE__ */ jsxs("span", { className: "armada-screen__strong", children: [
            "Approve lands the job in ",
            /* @__PURE__ */ jsx("span", { className: "armada-screen__mono", children: "queued" }),
            ", not ",
            /* @__PURE__ */ jsx("span", { className: "armada-screen__mono", children: "running" }),
            "."
          ] }),
          " ",
          "A drone spawning is what starts it, and at M1 Fleet runs one at a time, so a job approved while another is working sits queued for as long as that one takes. Its badge carries ",
          /* @__PURE__ */ jsx("span", { className: "armada-screen__mono", children: "cpu" }),
          " rather than",
          " ",
          /* @__PURE__ */ jsx("span", { className: "armada-screen__mono", children: "clock" }),
          ", because a reason’s glyph replaces ",
          /* @__PURE__ */ jsx("span", { className: "armada-screen__mono", children: "clock" }),
          " where one is present and M1’s only reason is",
          " ",
          /* @__PURE__ */ jsx("span", { className: "armada-screen__mono", children: "waiting_on_resources" }),
          " — there are no dependencies to be blocked by. Same card, same order, same button, one field set smaller. The glance strip survives with the two values M1 can measure before dispatch — how long the workflow is and which Checks gate it — because a card whose whole design is a forced glance cannot ship with nothing to glance at.",
          " ",
          /* @__PURE__ */ jsxs("span", { className: "armada-screen__strong", children: [
            "Cancel writes ",
            /* @__PURE__ */ jsx("span", { className: "armada-screen__mono", children: "killed" }),
            "."
          ] }),
          " ",
          "A job you never dispatched was not stopped, it was abandoned, so the copy names what the person is doing while the record names what happened.",
          " ",
          /* @__PURE__ */ jsx("span", { className: "armada-screen__mono", children: "rejected" }),
          " is the verdict exit and it is out of M1, so ",
          /* @__PURE__ */ jsx("span", { className: "armada-screen__mono", children: "killed" }),
          " is the only destination — an operator act carrying no verdict, which is the honest reading of closing a card."
        ] })
      ] }),
      /* @__PURE__ */ jsxs("div", { className: "armada-screen__card", "data-dim": true, "data-width": "narrow", children: [
        /* @__PURE__ */ jsxs("div", { className: "armada-screen__card-head", children: [
          /* @__PURE__ */ jsx("span", { className: "armada-screen__caption", children: "Forks off the main line" }),
          /* @__PURE__ */ jsx("span", { className: "armada-screen__tag", "data-dim": true, children: "not at M1" })
        ] }),
        /* @__PURE__ */ jsxs("div", { className: "armada-screen__stack", children: [
          /* @__PURE__ */ jsxs("div", { className: "armada-screen__fork", children: [
            /* @__PURE__ */ jsx("span", { children: "Pre-approved before you step away" }),
            /* @__PURE__ */ jsx("span", { children: "Specific queued jobs marked to dispatch in your absence, indefinite until run or revoked." })
          ] }),
          /* @__PURE__ */ jsxs("div", { className: "armada-screen__fork", children: [
            /* @__PURE__ */ jsx("span", { children: "Pattern learning" }),
            /* @__PURE__ */ jsx("span", { children: "After the same command trips the allowlist N times, Armada proposes a Manifest change. You confirm or decline." })
          ] }),
          /* @__PURE__ */ jsxs("div", { className: "armada-screen__fork", children: [
            /* @__PURE__ */ jsx("span", { children: "Criteria editor" }),
            /* @__PURE__ */ jsx("span", { children: "The only place acceptance criteria are authored. Nothing reads them until the Judge exists." })
          ] })
        ] }),
        /* @__PURE__ */ jsx("span", { className: "armada-screen__caption", children: "All three sit outside the card and none is on the path through it, which is why the reduced version is a subset rather than a redraw." })
      ] })
    ] }),
    /* @__PURE__ */ jsxs("p", { className: "armada-screen__note", children: [
      /* @__PURE__ */ jsx("span", { className: "armada-screen__strong", children: "Approval stays one-by-one, and the accent is spent here." }),
      " ",
      "Approve and dispatch is the only accent fill in the whole milestone: it is the single primary action M1 has, and every other screen’s controls are secondary or ghost. The keyboard path is the same shape as the mouse path — the form is a tab order ending on the primary, and Enter from any field commits it."
    ] })
  ] })
};
const __vite_glob_0_37 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Dispatch,
  default: meta$3
}, Symbol.toStringTag, { value: "Module" }));
const meta$2 = {
  title: "Screens/First launch"
};
const FirstLaunch = {
  render: () => /* @__PURE__ */ jsx("div", { className: "armada-screen", children: /* @__PURE__ */ jsxs("div", { className: "armada-screen__row", children: [
    /* @__PURE__ */ jsxs("div", { className: "armada-screen__card", "data-width": "half", children: [
      /* @__PURE__ */ jsx("span", { className: "armada-screen__caption", children: "Fleet running, no jobs" }),
      /* @__PURE__ */ jsx("div", { className: "armada-screen__empty-slot", children: /* @__PURE__ */ jsx(
        Absent,
        {
          name: "Board empty state",
          note: "Holds one line — “No jobs. Fleet has been up 6 days.” — and the New job button beneath it. No centred glyph and no illustration."
        }
      ) }),
      /* @__PURE__ */ jsx("span", { className: "armada-screen__caption", "data-muted": true, children: "One line and the action. No centred glyph, no illustration — the empty state points at the work available and nothing else." })
    ] }),
    /* @__PURE__ */ jsxs("div", { className: "armada-screen__card", "data-width": "half", children: [
      /* @__PURE__ */ jsx("span", { className: "armada-screen__caption", children: "Fleet is not running" }),
      /* @__PURE__ */ jsx("div", { className: "armada-screen__empty-slot", children: /* @__PURE__ */ jsx(
        Absent,
        {
          name: "Board empty state",
          note: "Holds “Fleet is not running. Bridge has nothing to read.”, then armada-fleet start as a mono value to copy rather than a button, then “Run that in a terminal. Bridge connects on its own once the runtime file appears.”"
        }
      ) }),
      /* @__PURE__ */ jsx("span", { className: "armada-screen__caption", "data-muted": true, children: "The state a person will actually meet in M1, since Fleet is started by hand. The command is machine-derived, so it is mono, and it is a value to copy rather than a button — Bridge does not start Fleet at this milestone." })
    ] })
  ] }) })
};
const __vite_glob_0_38 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  FirstLaunch,
  default: meta$2
}, Symbol.toStringTag, { value: "Module" }));
const meta$1 = {
  title: "Screens/The list — six states, one row shape"
};
const PX = "px";
const menu = [
  { label: "Copy job id", shortcut: "⌘C" },
  { label: "Kill", shortcut: "x", danger: true }
];
const open = /* @__PURE__ */ jsx(SplitButton, { ground: "card", items: menu, children: "Open" });
const workflow = /* @__PURE__ */ jsxs(Fragment, { children: [
  /* @__PURE__ */ jsx("span", { className: "armada-screen__mono", children: "bug" }),
  ", 4 steps"
] });
const APPROVAL_TRACKS = [
  "calc(var(--space-12) * 3 + var(--space-6))",
  "calc(var(--space-12) + var(--space-6))",
  "calc(var(--space-12) * 2 + var(--space-3))",
  "calc(var(--space-12) * 2 + var(--space-1))",
  "calc(var(--space-12) * 2 + var(--space-8))"
].join(" ");
const TheList = {
  render: () => /* @__PURE__ */ jsxs("div", { className: "armada-screen", children: [
    /* @__PURE__ */ jsxs(
      ActiveJobsList,
      {
        heading: "Active jobs",
        summary: "6 jobs. 1 awaiting approval.",
        action: /* @__PURE__ */ jsx(Button, { variant: "primary", children: "New job" }),
        children: [
          /* @__PURE__ */ jsx(
            JobRowStacked,
            {
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
              action: /* @__PURE__ */ jsx(SplitButton, { ground: "card", items: [{ label: "Reject", danger: true }], children: "Approve" })
            },
            "a"
          ),
          /* @__PURE__ */ jsx(
            JobRowStacked,
            {
              status: "not-started",
              statusIcon: Cpu,
              statusLabel: "Queued",
              headline: "Retire the legacy poke path",
              jobId: "job_8b42",
              fields: [
                { value: workflow },
                { value: /* @__PURE__ */ jsx(StepBar, { total: 4, current: 0, label: "Not started, 4 steps" }) },
                { value: "Waiting on a drone", emphasis: true },
                { value: "approved 09:20", quiet: true },
                { value: "Dispatched by you" }
              ],
              action: open
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
                {
                  value: "fix/settings-split",
                  mono: true,
                  icon: GitBranch,
                  copyValue: "fix/settings-split"
                },
                { value: /* @__PURE__ */ jsx(StepBar, { total: 4, current: 2, activity: "running", label: "Step 2 of 4" }) },
                { value: "Implement", emphasis: true },
                { value: "11m 03s", mono: true },
                { value: "~$1.80", mono: true }
              ],
              action: open
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
                {
                  value: "feat/manifest-cache",
                  mono: true,
                  icon: GitBranch,
                  copyValue: "feat/manifest-cache"
                },
                { value: /* @__PURE__ */ jsx(StepBar, { total: 4, current: 3, activity: "failed", label: "Step 3 of 4" }) },
                { value: "Run tests", emphasis: true },
                { value: "22m 41s", mono: true },
                { value: "~$2.10", mono: true }
              ],
              action: open
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
                { value: "~$2.40", mono: true }
              ],
              action: open
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
                {
                  value: "feat/session-rename",
                  mono: true,
                  icon: GitBranch,
                  copyValue: "feat/session-rename"
                },
                { value: /* @__PURE__ */ jsx(StepBar, { total: 4, current: 2, activity: "killed", label: "Step 2 of 4" }) },
                { value: "Implement", emphasis: true },
                { value: "4m 09s", mono: true },
                { value: "~$0.60", mono: true }
              ],
              action: open
            },
            "f"
          )
        ]
      }
    ),
    /* @__PURE__ */ jsxs("div", { className: "armada-screen__notes", children: [
      /* @__PURE__ */ jsxs("p", { className: "armada-screen__note", children: [
        /* @__PURE__ */ jsx("span", { className: "armada-screen__strong", children: "Ordering carries the trigger, not a control." }),
        " ",
        "The one row that needs a person sorts first; the rest are newest work first. Every row carries one secondary split button and no accent, and the row itself opens the job. M1’s field run is workflow or branch, step bar, step, elapsed and spend — five fixed tracks, so the list reads down as well as across. The needs-approval row swaps the first four tracks, because it has no branch, no step and no elapsed yet: a job that has not run has different facts, and the track list belongs to the field set."
      ] }),
      /* @__PURE__ */ jsxs("p", { className: "armada-screen__note", children: [
        /* @__PURE__ */ jsx("span", { className: "armada-screen__strong", children: "A failed segment is loud; a killed one is not." }),
        " ",
        /* @__PURE__ */ jsx("span", { className: "armada-screen__mono", children: "--step-failed" }),
        " was added to the token set on 2026-08-23 and aliases its Job counterpart. The first pass gave a failed step no hue, on the grounds that a Check result is measured and measured facts render flatly — rejected, because at M1 a failed Check ends the Job and that row is the entire reason a person opened the screen. Killed keeps",
        " ",
        /* @__PURE__ */ jsx("span", { className: "armada-screen__mono", children: "--fg-default" }),
        " and no hue: it is a human decision rather than a system failure and must not read as an error. That distinction is what the two treatments now carry."
      ] })
    ] }),
    /* @__PURE__ */ jsxs("p", { className: "armada-screen__note", children: [
      /* @__PURE__ */ jsx("span", { className: "armada-screen__strong", children: "On a list row the badge carries the pulse; the bar never does." }),
      " ",
      "Decided 2026-08-23 after both were drawn: the badge is where",
      " ",
      /* @__PURE__ */ jsx("span", { className: "armada-screen__mono", children: "circle-dot" }),
      "’s inner dot is documented to pulse, and it sits in the same fixed",
      " ",
      `132${PX}`,
      " column on every row, so the motion appears in one predictable place rather than moving with the workflow’s length. The bar’s job is where the work got to, which is a static fact. One pulse per screen, on the most specific mark present — so on job detail the rail takes it and this badge goes static."
    ] })
  ] })
};
const __vite_glob_0_39 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  TheList,
  default: meta$1
}, Symbol.toStringTag, { value: "Module" }));
const meta = {
  title: "Screens/The shell"
};
const Shell = {
  render: () => /* @__PURE__ */ jsxs("div", { className: "armada-screen", children: [
    /* @__PURE__ */ jsxs("div", { className: "armada-screen__window", children: [
      /* @__PURE__ */ jsx(
        Sidebar,
        {
          appName: "Armada",
          sectionLabel: "Bridge",
          activeId: "active",
          header: /* @__PURE__ */ jsx(Select, { "aria-label": "Project", children: /* @__PURE__ */ jsx("option", { children: "armada" }) }),
          surfaces: [
            // The drawing's rail row carries a label and a count and no glyph.
            // Sidebar requires one, and `activity` is what the registry assigns
            // to Active jobs, so that is the glyph. Reported.
            { id: "active", label: "Active jobs", icon: Activity, count: 6 }
          ]
        }
      ),
      /* @__PURE__ */ jsxs("div", { className: "armada-screen__panel", children: [
        /* @__PURE__ */ jsxs("div", { className: "armada-screen__panel-head", children: [
          /* @__PURE__ */ jsxs("div", { className: "armada-screen__titles", children: [
            /* @__PURE__ */ jsx("span", { className: "armada-screen__title", children: "Active jobs" }),
            /* @__PURE__ */ jsx("span", { className: "armada-screen__summary", children: "6 jobs. 1 awaiting approval." })
          ] }),
          /* @__PURE__ */ jsx(Button, { variant: "primary", children: "New job" })
        ] }),
        /* @__PURE__ */ jsx("div", { className: "armada-screen__mount", children: "The list mounts here — 1d" }),
        /* @__PURE__ */ jsx(
          StatusBar,
          {
            fleet: "running",
            fleetLabel: "Fleet running",
            detail: "pid 4417 · port 7411 · 1 drone",
            spend: "today ~$4.80"
          }
        )
      ] })
    ] }),
    /* @__PURE__ */ jsx("span", { className: "armada-screen__eyebrow", children: "The status bar says Fleet out loud — three states" }),
    /* @__PURE__ */ jsxs("div", { className: "armada-screen__col", children: [
      /* @__PURE__ */ jsx("div", { className: "armada-screen__bar-frame", children: /* @__PURE__ */ jsx(
        StatusBar,
        {
          fleet: "running",
          fleetLabel: "Fleet running",
          detail: "pid 4417 · port 7411 · 1 drone",
          spend: "today ~$4.80"
        }
      ) }),
      /* @__PURE__ */ jsx("div", { className: "armada-screen__bar-frame", children: /* @__PURE__ */ jsx(
        StatusBar,
        {
          fleet: "not-running",
          fleetLabel: "Fleet is not running",
          detail: "no runtime file at ~/.armada/fleet.json",
          advice: "Start it from the terminal."
        }
      ) }),
      /* @__PURE__ */ jsx("div", { className: "armada-screen__bar-frame", children: /* @__PURE__ */ jsx(
        StatusBar,
        {
          fleet: "unreachable",
          fleetLabel: "Fleet unreachable",
          detail: "pid 4417 alive on port 7411 · no response for 20s",
          advice: "The last job state read is 20s old."
        }
      ) })
    ] }),
    /* @__PURE__ */ jsxs("p", { className: "armada-screen__note", children: [
      /* @__PURE__ */ jsx("span", { className: "armada-screen__strong", children: "Not running and unreachable differ on the runtime file." }),
      " ",
      "Fleet writes port, pid and protocol version on startup and removes them on a clean exit, so a missing file is a Fleet that is not there and a live pid with no answer is a Fleet that is wedged — two different things to do about it, so two sentences. The dot takes a status hue on the same grounds Doctor’s pass, warn and fail do: a health claim reuses the Job values rather than inventing a third set. Colour beyond that stays out of the bar — there are no escalation or approval counts in M1 to carry it."
    ] })
  ] })
};
const __vite_glob_0_40 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Shell,
  default: meta
}, Symbol.toStringTag, { value: "Module" }));
const stories = /* @__PURE__ */ Object.assign({
  "../src/compositions/ActiveJobsList/ActiveJobsList.stories.tsx": __vite_glob_0_0,
  "../src/compositions/EvidenceTrail/EvidenceTrail.stories.tsx": __vite_glob_0_1,
  "../src/compositions/JobLogReference/JobLogReference.stories.tsx": __vite_glob_0_2,
  "../src/compositions/JobRowStacked/JobRowStacked.stories.tsx": __vite_glob_0_3,
  "../src/compositions/Sidebar/Sidebar.stories.tsx": __vite_glob_0_4,
  "../src/compositions/StatusBar/StatusBar.stories.tsx": __vite_glob_0_5,
  "../src/compositions/StepActivityMark/StepActivityMark.stories.tsx": __vite_glob_0_6,
  "../src/compositions/StepBar/StepBar.stories.tsx": __vite_glob_0_7,
  "../src/compositions/WorkflowRail/WorkflowRail.stories.tsx": __vite_glob_0_8,
  "../src/primitives/Alert/Alert.stories.tsx": __vite_glob_0_9,
  "../src/primitives/Badge/Badge.stories.tsx": __vite_glob_0_10,
  "../src/primitives/Button/Button.stories.tsx": __vite_glob_0_11,
  "../src/primitives/Card/Card.stories.tsx": __vite_glob_0_12,
  "../src/primitives/Checkbox/Checkbox.stories.tsx": __vite_glob_0_13,
  "../src/primitives/CommandPalette/CommandPalette.stories.tsx": __vite_glob_0_14,
  "../src/primitives/Dialog/Dialog.stories.tsx": __vite_glob_0_15,
  "../src/primitives/DropdownMenu/DropdownMenu.stories.tsx": __vite_glob_0_16,
  "../src/primitives/Input/Input.stories.tsx": __vite_glob_0_17,
  "../src/primitives/Kbd/Kbd.stories.tsx": __vite_glob_0_18,
  "../src/primitives/Popover/Popover.stories.tsx": __vite_glob_0_19,
  "../src/primitives/Radio/Radio.stories.tsx": __vite_glob_0_20,
  "../src/primitives/ScrollArea/ScrollArea.stories.tsx": __vite_glob_0_21,
  "../src/primitives/Select/Select.stories.tsx": __vite_glob_0_22,
  "../src/primitives/Separator/Separator.stories.tsx": __vite_glob_0_23,
  "../src/primitives/Sheet/Sheet.stories.tsx": __vite_glob_0_24,
  "../src/primitives/Skeleton/Skeleton.stories.tsx": __vite_glob_0_25,
  "../src/primitives/SplitButton/SplitButton.stories.tsx": __vite_glob_0_26,
  "../src/primitives/Switch/Switch.stories.tsx": __vite_glob_0_27,
  "../src/primitives/Table/Table.stories.tsx": __vite_glob_0_28,
  "../src/primitives/Tabs/Tabs.stories.tsx": __vite_glob_0_29,
  "../src/primitives/TabsWithCounts/TabsWithCounts.stories.tsx": __vite_glob_0_30,
  "../src/primitives/Textarea/Textarea.stories.tsx": __vite_glob_0_31,
  "../src/primitives/Toast/Toast.stories.tsx": __vite_glob_0_32,
  "../src/primitives/Tooltip/Tooltip.stories.tsx": __vite_glob_0_33,
  "../src/screens/AFailedJobADeadEndReadAsOne/AFailedJobADeadEndReadAsOne.stories.tsx": __vite_glob_0_34,
  "../src/screens/AFinishedJobABranchAndAnEvidenceTrail/AFinishedJobABranchAndAnEvidenceTrail.stories.tsx": __vite_glob_0_35,
  "../src/screens/ARunningJob/ARunningJob.stories.tsx": __vite_glob_0_36,
  "../src/screens/DispatchAJobFullWithTheM1SubsetMarked/DispatchAJobFullWithTheM1SubsetMarked.stories.tsx": __vite_glob_0_37,
  "../src/screens/FirstLaunch/FirstLaunch.stories.tsx": __vite_glob_0_38,
  "../src/screens/TheListSixStatesOneRowShape/TheListSixStatesOneRowShape.stories.tsx": __vite_glob_0_39,
  "../src/screens/TheShell/TheShell.stories.tsx": __vite_glob_0_40
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
