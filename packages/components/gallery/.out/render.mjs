import { jsxs, jsx, Fragment } from "react/jsx-runtime";
import { TriangleAlert, Stethoscope, UserCheck, Stamp, Eye, X, Check, Power, Terminal, Clock, Link, Cpu, Ban, CircleDot, Archive, OctagonAlert, RefreshCw, FileQuestion, ShieldX, Split, Unplug, ArrowUpToLine, RotateCw, Send, CornerUpRight, ClipboardList, Activity, Bell, MessageSquare, Settings, ChevronDown } from "lucide-react";
import { useState, useRef, useMemo, useEffect, useId, useCallback, createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
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
const meta$n = {
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
const __vite_glob_0_0 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Escalated: Escalated$1,
  Neutral,
  default: meta$n
}, Symbol.toStringTag, { value: "Module" }));
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
const meta$m = {
  title: "Primitives/Badge",
  component: Badge
};
const NO_GLYPH_IN_REGISTRY = void 0;
const NotStarted = {
  args: { status: "not-started", icon: NO_GLYPH_IN_REGISTRY, children: "Not started" }
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
    /* @__PURE__ */ jsx(Badge, { status: "escalated", icon: FileQuestion, children: "Evidence disputed" }),
    /* @__PURE__ */ jsx(Badge, { status: "escalated", icon: ShieldX, children: "Check failed" }),
    /* @__PURE__ */ jsx(Badge, { status: "escalated", icon: Split, children: "Fanned out" }),
    /* @__PURE__ */ jsx(Badge, { status: "escalated", icon: Unplug, children: "Connection lost" }),
    /* @__PURE__ */ jsx(Badge, { status: "escalated", icon: ArrowUpToLine, children: "Reached its ceiling" })
  ] })
};
const __vite_glob_0_1 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
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
  default: meta$m
}, Symbol.toStringTag, { value: "Module" }));
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
const meta$l = {
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
const Focused$6 = {
  render: () => /* @__PURE__ */ jsx(Card$6, { children: /* @__PURE__ */ jsx(Button, { variant: "secondary", "data-preview-focus": "", children: "Focused" }) })
};
const Disabled$6 = {
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
const Light$6 = {
  render: () => /* @__PURE__ */ jsx("div", { "data-theme": "light", children: /* @__PURE__ */ jsxs(Card$6, { children: [
    /* @__PURE__ */ jsx(Button, { variant: "primary", children: "Dispatch job" }),
    /* @__PURE__ */ jsx(Button, { variant: "secondary", children: "Cancel" })
  ] }) })
};
const __vite_glob_0_2 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Destructive,
  Disabled: Disabled$6,
  Focused: Focused$6,
  Ghost,
  Group,
  Hover,
  Light: Light$6,
  Primary,
  Secondary,
  SecondaryOnASunkenGround,
  Small,
  default: meta$l
}, Symbol.toStringTag, { value: "Module" }));
function joined$1(base, extra) {
  return extra ? `${base} ${extra}` : base;
}
function Card$5({ className, ...rest }) {
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
const meta$k = {
  title: "Primitives/Card",
  component: Card$5,
  decorators: [
    (Story) => /* @__PURE__ */ jsx("div", { style: { maxWidth: "56ch" }, children: /* @__PURE__ */ jsx(Story, {}) })
  ]
};
const Default$5 = {
  render: () => /* @__PURE__ */ jsxs(Card$5, { children: [
    /* @__PURE__ */ jsx(CardTitle, { children: "Evidence accepted at step 4 of 5" }),
    /* @__PURE__ */ jsx(CardDescription, { children: "Four criteria resolved. One test added, none removed." })
  ] })
};
const WithHeader = {
  render: () => /* @__PURE__ */ jsxs(Card$5, { children: [
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
  render: () => /* @__PURE__ */ jsxs(Card$5, { "data-dimmed": true, children: [
    /* @__PURE__ */ jsx(CardTitle, { children: "Superseded at step 2 of 5" }),
    /* @__PURE__ */ jsx(CardDescription, { children: "The work landed outside this job." })
  ] })
};
const __vite_glob_0_3 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Default: Default$5,
  Dimmed: Dimmed$1,
  WithHeader,
  default: meta$k
}, Symbol.toStringTag, { value: "Module" }));
function Checkbox({ children, ...rest }) {
  return /* @__PURE__ */ jsxs("label", { className: "armada-checkbox", children: [
    /* @__PURE__ */ jsx("input", { ...rest, type: "checkbox", className: "armada-checkbox__input" }),
    /* @__PURE__ */ jsx("span", { className: "armada-checkbox__box", "aria-hidden": "true", children: /* @__PURE__ */ jsx(Check, { size: 12, strokeWidth: 2 }) }),
    /* @__PURE__ */ jsx("span", { children })
  ] });
}
const meta$j = {
  title: "Primitives/Checkbox",
  component: Checkbox
};
function Card$4({ children }) {
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
  render: () => /* @__PURE__ */ jsx(Card$4, { children: /* @__PURE__ */ jsx(Checkbox, { children: "Land as a convoy" }) })
};
const Checked = {
  render: () => /* @__PURE__ */ jsx(Card$4, { children: /* @__PURE__ */ jsx(Checkbox, { defaultChecked: true, children: "Run Doctor before dispatch" }) })
};
const Focused$5 = {
  render: () => /* @__PURE__ */ jsx(Card$4, { children: /* @__PURE__ */ jsx(Checkbox, { defaultChecked: true, "data-preview-focus": "", children: "Run Doctor before dispatch" }) })
};
const Disabled$5 = {
  render: () => /* @__PURE__ */ jsxs(Card$4, { children: [
    /* @__PURE__ */ jsx(Checkbox, { disabled: true, children: "Land as a convoy" }),
    /* @__PURE__ */ jsx(Checkbox, { disabled: true, defaultChecked: true, children: "Run Doctor before dispatch" })
  ] })
};
const Light$5 = {
  render: () => /* @__PURE__ */ jsx("div", { "data-theme": "light", children: /* @__PURE__ */ jsx(Card$4, { children: /* @__PURE__ */ jsx(Checkbox, { defaultChecked: true, children: "Run Doctor before dispatch" }) }) })
};
const __vite_glob_0_4 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Checked,
  Disabled: Disabled$5,
  Focused: Focused$5,
  Light: Light$5,
  Unchecked,
  default: meta$j
}, Symbol.toStringTag, { value: "Module" }));
const DEFAULT_SECTIONS = ["Actions", "Navigation", "Jobs", "Settings"];
function matches(entry, query) {
  if (!query) return true;
  const q = query.toLowerCase();
  if (entry.label.toLowerCase().includes(q)) return true;
  return (entry.aliases ?? []).some((alias) => alias.toLowerCase().includes(q));
}
function CommandPalette({
  open,
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
    if (open) input.current?.focus();
  }, [open]);
  if (!open) return null;
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
  open,
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
    if (open) cancelRef.current?.focus();
  }, [open]);
  useEffect(() => {
    if (!open) return;
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
  }, [open, onCancel, onConfirm]);
  if (!open) return null;
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
const meta$i = {
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
const __vite_glob_0_5 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AliasFindsTheLexiconTerm,
  DestructiveEntryConfirms,
  NoMatch,
  Resting: Resting$1,
  default: meta$i
}, Symbol.toStringTag, { value: "Module" }));
const meta$h = {
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
const __vite_glob_0_6 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Confirmation,
  NeutralConfirm,
  default: meta$h
}, Symbol.toStringTag, { value: "Module" }));
function DropdownMenu({
  triggerLabel,
  entries: entries2,
  defaultOpen = false,
  onSelect
}) {
  const [open, setOpen] = useState(defaultOpen);
  const root = useRef(null);
  useEffect(() => {
    if (!open) return;
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
  }, [open]);
  return /* @__PURE__ */ jsxs("div", { className: "armada-dropdown-menu", ref: root, children: [
    /* @__PURE__ */ jsx(
      "button",
      {
        type: "button",
        className: "armada-dropdown-menu__trigger",
        "aria-haspopup": "menu",
        "aria-expanded": open,
        onClick: () => setOpen((v) => !v),
        children: triggerLabel
      }
    ),
    open ? /* @__PURE__ */ jsx("div", { className: "armada-dropdown-menu__panel", role: "menu", children: entries2.map((entry) => {
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
const meta$g = {
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
const __vite_glob_0_7 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AtTheLeftEdge: AtTheLeftEdge$2,
  AtTheRightEdge: AtTheRightEdge$2,
  RowMenu,
  WithNoRoomBelow: WithNoRoomBelow$2,
  WithSectionLabels,
  default: meta$g
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
const meta$f = {
  title: "Primitives/Input",
  component: Input
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
const Default$4 = {
  args: { label: "Job title", defaultValue: "Refresh the auth token flow" },
  render: (args) => /* @__PURE__ */ jsx(Card$3, { children: /* @__PURE__ */ jsx(Input, { ...args }) })
};
const Placeholder = {
  args: { label: "Job title", placeholder: "Refresh the auth token flow" },
  render: (args) => /* @__PURE__ */ jsx(Card$3, { children: /* @__PURE__ */ jsx(Input, { ...args }) })
};
const Mono = {
  args: { label: "Project location", defaultValue: "~/code/armada", mono: true },
  render: (args) => /* @__PURE__ */ jsx(Card$3, { children: /* @__PURE__ */ jsx(Input, { ...args }) })
};
const Focused$4 = {
  render: () => /* @__PURE__ */ jsx(Card$3, { children: /* @__PURE__ */ jsx(Input, { label: "Job title", defaultValue: "Refresh the auth token flow", "data-preview-focus": "" }) })
};
const Invalid$1 = {
  render: () => /* @__PURE__ */ jsx(Card$3, { children: /* @__PURE__ */ jsx(
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
const Disabled$4 = {
  render: () => /* @__PURE__ */ jsx(Card$3, { children: /* @__PURE__ */ jsx(Input, { label: "Job title", defaultValue: "Refresh the auth token flow", disabled: true }) })
};
const Light$4 = {
  render: () => /* @__PURE__ */ jsx("div", { "data-theme": "light", children: /* @__PURE__ */ jsx(Card$3, { children: /* @__PURE__ */ jsx(Input, { label: "Job title", defaultValue: "Refresh the auth token flow" }) }) })
};
const __vite_glob_0_8 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Default: Default$4,
  Disabled: Disabled$4,
  Focused: Focused$4,
  Invalid: Invalid$1,
  Light: Light$4,
  Mono,
  Placeholder,
  default: meta$f
}, Symbol.toStringTag, { value: "Module" }));
function Kbd({ className, ...rest }) {
  return /* @__PURE__ */ jsx("kbd", { className: className ? `armada-kbd ${className}` : "armada-kbd", ...rest });
}
function KbdChord({ className, ...rest }) {
  return /* @__PURE__ */ jsx("span", { className: className ? `armada-kbd-chord ${className}` : "armada-kbd-chord", ...rest });
}
const meta$e = {
  title: "Primitives/kbd",
  component: Kbd
};
const Default$3 = {
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
const __vite_glob_0_9 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Chord,
  ContextualKeys,
  Default: Default$3,
  default: meta$e
}, Symbol.toStringTag, { value: "Module" }));
function Popover({ trigger: trigger2, children, align = "start", defaultOpen = false }) {
  const [open, setOpen] = useState(defaultOpen);
  const root = useRef(null);
  useEffect(() => {
    if (!open) return;
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
  }, [open]);
  return /* @__PURE__ */ jsxs("div", { className: "armada-popover", ref: root, children: [
    /* @__PURE__ */ jsx("span", { className: "armada-popover__trigger", onClick: () => setOpen((v) => !v), children: trigger2 }),
    open ? /* @__PURE__ */ jsx(
      "div",
      {
        className: align === "end" ? "armada-popover__panel armada-popover__panel--end" : "armada-popover__panel armada-popover__panel--start",
        role: "dialog",
        children
      }
    ) : null
  ] });
}
const meta$d = {
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
const __vite_glob_0_10 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AlignedToTheEnd,
  AtTheLeftEdge: AtTheLeftEdge$1,
  AtTheRightEdge: AtTheRightEdge$1,
  Open: Open$1,
  WithNoRoomBelow: WithNoRoomBelow$1,
  default: meta$d
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
const meta$c = {
  title: "Primitives/Radio",
  component: Radio
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
const Default$2 = {
  render: () => /* @__PURE__ */ jsx(Card$2, { children: /* @__PURE__ */ jsxs(RadioGroup, { label: "Kit source", children: [
    /* @__PURE__ */ jsx(Radio, { name: "kit", defaultChecked: true, children: "Import an existing Kit" }),
    /* @__PURE__ */ jsx(Radio, { name: "kit", children: "Start fresh" })
  ] }) })
};
const Focused$3 = {
  render: () => /* @__PURE__ */ jsx(Card$2, { children: /* @__PURE__ */ jsxs(RadioGroup, { label: "Kit source", children: [
    /* @__PURE__ */ jsx(Radio, { name: "kit-focus", defaultChecked: true, "data-preview-focus": "", children: "Import an existing Kit" }),
    /* @__PURE__ */ jsx(Radio, { name: "kit-focus", children: "Start fresh" })
  ] }) })
};
const Disabled$3 = {
  render: () => /* @__PURE__ */ jsx(Card$2, { children: /* @__PURE__ */ jsxs(RadioGroup, { label: "Kit source", children: [
    /* @__PURE__ */ jsx(Radio, { name: "kit-disabled", defaultChecked: true, disabled: true, children: "Import an existing Kit" }),
    /* @__PURE__ */ jsx(Radio, { name: "kit-disabled", disabled: true, children: "Start fresh" })
  ] }) })
};
const Light$3 = {
  render: () => /* @__PURE__ */ jsx("div", { "data-theme": "light", children: /* @__PURE__ */ jsx(Card$2, { children: /* @__PURE__ */ jsxs(RadioGroup, { label: "Kit source", children: [
    /* @__PURE__ */ jsx(Radio, { name: "kit-light", defaultChecked: true, children: "Import an existing Kit" }),
    /* @__PURE__ */ jsx(Radio, { name: "kit-light", children: "Start fresh" })
  ] }) }) })
};
const __vite_glob_0_11 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Default: Default$2,
  Disabled: Disabled$3,
  Focused: Focused$3,
  Light: Light$3,
  default: meta$c
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
const meta$b = {
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
const __vite_glob_0_12 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Scrolling,
  WithinBounds,
  default: meta$b
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
const meta$a = {
  title: "Primitives/Select",
  component: Select
};
function Card$1({ children }) {
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
const Default$1 = {
  render: () => /* @__PURE__ */ jsx(Card$1, { children: /* @__PURE__ */ jsx(Select, { label: "Concurrency ceiling", children: ceilings }) })
};
const Focused$2 = {
  render: () => /* @__PURE__ */ jsx(Card$1, { children: /* @__PURE__ */ jsx(Select, { label: "Concurrency ceiling", "data-preview-focus": "", children: ceilings }) })
};
const Invalid = {
  render: () => /* @__PURE__ */ jsx(Card$1, { children: /* @__PURE__ */ jsx(
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
const Disabled$2 = {
  render: () => /* @__PURE__ */ jsx(Card$1, { children: /* @__PURE__ */ jsx(Select, { label: "Concurrency ceiling", disabled: true, children: ceilings }) })
};
const Light$2 = {
  render: () => /* @__PURE__ */ jsx("div", { "data-theme": "light", children: /* @__PURE__ */ jsx(Card$1, { children: /* @__PURE__ */ jsx(Select, { label: "Concurrency ceiling", children: ceilings }) }) })
};
const __vite_glob_0_13 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Default: Default$1,
  Disabled: Disabled$2,
  Focused: Focused$2,
  Invalid,
  Light: Light$2,
  default: meta$a
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
const meta$9 = {
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
const __vite_glob_0_14 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Announced,
  Horizontal,
  Vertical,
  default: meta$9
}, Symbol.toStringTag, { value: "Module" }));
function Sheet({ open, title, children, side = "right", footer, onClose }) {
  const closeRef = useRef(null);
  useEffect(() => {
    if (open) closeRef.current?.focus();
  }, [open]);
  useEffect(() => {
    if (!open) return;
    function onKey(event) {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose?.();
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);
  if (!open) return null;
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
const meta$8 = {
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
const __vite_glob_0_15 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Left,
  Right,
  default: meta$8
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
const meta$7 = {
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
  render: () => /* @__PURE__ */ jsxs(Card$5, { children: [
    /* @__PURE__ */ jsx(CardTitle, { children: "Evidence" }),
    /* @__PURE__ */ jsx(SkeletonText, { label: "Loading evidence" })
  ] })
};
const __vite_glob_0_16 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  InACard,
  Single,
  Text,
  default: meta$7
}, Symbol.toStringTag, { value: "Module" }));
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
  const [open, setOpen] = useState(defaultOpen);
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
          "aria-expanded": open,
          "aria-label": menuLabel,
          disabled,
          onClick: () => setOpen((was) => !was),
          children: /* @__PURE__ */ jsx(ChevronDown, { size: 16, strokeWidth: 2, "aria-hidden": "true" })
        }
      )
    ] }),
    open && /* @__PURE__ */ jsx("div", { className: "armada-split-button__menu", role: "menu", "aria-label": menuLabel, children: items.map((item) => /* @__PURE__ */ jsxs(
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
const meta$6 = {
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
const Focused$1 = {
  render: () => /* @__PURE__ */ jsxs("div", { style: { display: "flex", gap: "var(--space-4)" }, children: [
    /* @__PURE__ */ jsx("div", { "data-preview-focus": "action", children: /* @__PURE__ */ jsx(Row, { children: /* @__PURE__ */ jsx(SplitButton, { items: reviewActions, ground: "sunken", children: "Approve" }) }) }),
    /* @__PURE__ */ jsx("div", { "data-preview-focus": "caret", children: /* @__PURE__ */ jsx(Row, { children: /* @__PURE__ */ jsx(SplitButton, { items: reviewActions, ground: "sunken", children: "Approve" }) }) })
  ] })
};
const Disabled$1 = {
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
const Light$1 = {
  render: () => /* @__PURE__ */ jsx("div", { "data-theme": "light", children: /* @__PURE__ */ jsx(Row, { children: /* @__PURE__ */ jsx(SplitButton, { items: reviewActions, ground: "sunken", children: "Approve" }) }) })
};
const FocusedOnPrimary = {
  render: () => /* @__PURE__ */ jsxs("div", { className: "armada-split-button-focus-row", children: [
    /* @__PURE__ */ jsx("div", { "data-preview-focus": "action", children: /* @__PURE__ */ jsx(SplitButton, { items: reviewActions, variant: "primary", children: "Approve" }) }),
    /* @__PURE__ */ jsx("div", { "data-preview-focus": "caret", children: /* @__PURE__ */ jsx(SplitButton, { items: reviewActions, variant: "primary", children: "Approve" }) })
  ] })
};
const __vite_glob_0_17 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Closed,
  Disabled: Disabled$1,
  EscalatedRow,
  Focused: Focused$1,
  FocusedOnPrimary,
  Light: Light$1,
  Open,
  PrimaryOnJobDetail,
  default: meta$6
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
const meta$5 = {
  title: "Primitives/Switch",
  component: Switch
};
function Card({ children }) {
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
  render: () => /* @__PURE__ */ jsx(Card, { children: /* @__PURE__ */ jsx(Switch, { defaultChecked: true, children: "Escalate on stall" }) })
};
const Off = {
  render: () => /* @__PURE__ */ jsx(Card, { children: /* @__PURE__ */ jsx(Switch, { children: "Auto-approve small diffs" }) })
};
const Focused = {
  render: () => /* @__PURE__ */ jsx(Card, { children: /* @__PURE__ */ jsx(Switch, { defaultChecked: true, "data-preview-focus": "", children: "Escalate on stall" }) })
};
const Disabled = {
  render: () => /* @__PURE__ */ jsxs(Card, { children: [
    /* @__PURE__ */ jsx(Switch, { defaultChecked: true, disabled: true, children: "Escalate on stall" }),
    /* @__PURE__ */ jsx(Switch, { disabled: true, children: "Auto-approve small diffs" })
  ] })
};
const WithADescription = {
  render: () => /* @__PURE__ */ jsxs(Card, { children: [
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
const Light = {
  render: () => /* @__PURE__ */ jsx("div", { "data-theme": "light", children: /* @__PURE__ */ jsx(Card, { children: /* @__PURE__ */ jsx(Switch, { defaultChecked: true, children: "Escalate on stall" }) }) })
};
const __vite_glob_0_18 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Disabled,
  Focused,
  Light,
  Off,
  On,
  WithADescription,
  default: meta$5
}, Symbol.toStringTag, { value: "Module" }));
const meta$4 = {
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
const Default = {
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
const __vite_glob_0_19 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Default,
  Dimmed,
  FocusedAndSelected,
  MonoValuesCopy,
  RowsGrowWithContent,
  default: meta$4
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
const meta$3 = {
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
const __vite_glob_0_20 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  LastActive,
  SectionsOfOneObject,
  default: meta$3
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
const meta$2 = {
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
const __vite_glob_0_21 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Queues,
  Zero,
  default: meta$2
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
const meta$1 = {
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
const __vite_glob_0_22 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  Copied,
  Killed,
  Landed,
  default: meta$1
}, Symbol.toStringTag, { value: "Module" }));
function Tooltip({ label: label2, shortcut, children, defaultOpen = false }) {
  const [open, setOpen] = useState(defaultOpen);
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
        open ? /* @__PURE__ */ jsxs("span", { className: "armada-tooltip__bubble", role: "tooltip", children: [
          /* @__PURE__ */ jsx("span", { className: "armada-tooltip__label", children: label2 }),
          shortcut ? /* @__PURE__ */ jsx("kbd", { className: "armada-tooltip__kbd", children: shortcut }) : null
        ] }) : null
      ]
    }
  );
}
const meta = {
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
const __vite_glob_0_23 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AtTheLeftEdge,
  AtTheRightEdge,
  Resting,
  TruncatedValue,
  WithNoRoomBelow,
  WithShortcut,
  default: meta
}, Symbol.toStringTag, { value: "Module" }));
const stories = /* @__PURE__ */ Object.assign({
  "../src/primitives/Alert/Alert.stories.tsx": __vite_glob_0_0,
  "../src/primitives/Badge/Badge.stories.tsx": __vite_glob_0_1,
  "../src/primitives/Button/Button.stories.tsx": __vite_glob_0_2,
  "../src/primitives/Card/Card.stories.tsx": __vite_glob_0_3,
  "../src/primitives/Checkbox/Checkbox.stories.tsx": __vite_glob_0_4,
  "../src/primitives/CommandPalette/CommandPalette.stories.tsx": __vite_glob_0_5,
  "../src/primitives/Dialog/Dialog.stories.tsx": __vite_glob_0_6,
  "../src/primitives/DropdownMenu/DropdownMenu.stories.tsx": __vite_glob_0_7,
  "../src/primitives/Input/Input.stories.tsx": __vite_glob_0_8,
  "../src/primitives/Kbd/Kbd.stories.tsx": __vite_glob_0_9,
  "../src/primitives/Popover/Popover.stories.tsx": __vite_glob_0_10,
  "../src/primitives/Radio/Radio.stories.tsx": __vite_glob_0_11,
  "../src/primitives/ScrollArea/ScrollArea.stories.tsx": __vite_glob_0_12,
  "../src/primitives/Select/Select.stories.tsx": __vite_glob_0_13,
  "../src/primitives/Separator/Separator.stories.tsx": __vite_glob_0_14,
  "../src/primitives/Sheet/Sheet.stories.tsx": __vite_glob_0_15,
  "../src/primitives/Skeleton/Skeleton.stories.tsx": __vite_glob_0_16,
  "../src/primitives/SplitButton/SplitButton.stories.tsx": __vite_glob_0_17,
  "../src/primitives/Switch/Switch.stories.tsx": __vite_glob_0_18,
  "../src/primitives/Table/Table.stories.tsx": __vite_glob_0_19,
  "../src/primitives/Tabs/Tabs.stories.tsx": __vite_glob_0_20,
  "../src/primitives/TabsWithCounts/TabsWithCounts.stories.tsx": __vite_glob_0_21,
  "../src/primitives/Toast/Toast.stories.tsx": __vite_glob_0_22,
  "../src/primitives/Tooltip/Tooltip.stories.tsx": __vite_glob_0_23
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
