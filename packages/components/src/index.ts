// Every primitive, exported once. Generated from the directories under
// src/primitives — a component exists here because it has a story, and
// Storybook is what says whether it does.

export * from "./primitives/Alert/Alert";
export * from "./primitives/AttachmentChip/AttachmentChip";
export * from "./primitives/Badge/Badge";
export * from "./primitives/Button/Button";
export * from "./primitives/Card/Card";
export * from "./primitives/Checkbox/Checkbox";
export * from "./primitives/CommandPalette/CommandPalette";
export * from "./primitives/Dialog/Dialog";
export * from "./primitives/DropdownMenu/DropdownMenu";
export * from "./primitives/HoldButton/HoldButton";
export * from "./primitives/Input/Input";
export * from "./primitives/Kbd/Kbd";
export * from "./primitives/MentionPopover/MentionPopover";
export * from "./primitives/Popover/Popover";
export * from "./primitives/Prose/Prose";
export * from "./primitives/Radio/Radio";
export * from "./primitives/ScrollArea/ScrollArea";
export * from "./primitives/Select/Select";
export * from "./primitives/Separator/Separator";
export * from "./primitives/Sheet/Sheet";
export * from "./primitives/Skeleton/Skeleton";
export * from "./primitives/SplitButton/SplitButton";
export * from "./primitives/Switch/Switch";
export * from "./primitives/Table/Table";
export * from "./primitives/Tabs/Tabs";
export * from "./primitives/TabsWithCounts/TabsWithCounts";
export * from "./primitives/Textarea/Textarea";
export * from "./primitives/Toast/Toast";
export * from "./primitives/Tooltip/Tooltip";

// Compositions — what M1 composes its screens from.
export * from "./compositions/ActiveJobsList/ActiveJobsList";
export * from "./compositions/ActivityLogSheet/ActivityLogSheet";
export * from "./compositions/BoardControls/BoardControls";
export * from "./compositions/BoardEmptyState/BoardEmptyState";
export * from "./compositions/ChangedFiles/ChangedFiles";
export * from "./compositions/CriterionVerdicts/CriterionVerdicts";
export * from "./compositions/DroneQuestion/DroneQuestion";
export * from "./compositions/DockQuestions/DockQuestions";
export * from "./compositions/HelmRecord/HelmRecord";
export * from "./compositions/HelmThread/HelmThread";
export * from "./compositions/HelmComposer/HelmComposer";
export * from "./compositions/JudgeQuestion/JudgeQuestion";
export * from "./compositions/EvidenceCard/EvidenceCard";
export * from "./compositions/FleetPanel/FleetPanel";
export * from "./compositions/DroneTurns/DroneTurns";
export * from "./compositions/EvidenceTrail/EvidenceTrail";
export * from "./compositions/FailureNotice/FailureNotice";
export * from "./compositions/FramesPaired/FramesPaired";
export * from "./compositions/FramesShown/FramesShown";
export * from "./compositions/GamingFlags/GamingFlags";
export * from "./compositions/JobComposer/JobComposer";
export * from "./compositions/JobDiffSheet/JobDiffSheet";
export * from "./compositions/JobBrief/JobBrief";
export * from "./compositions/JobDetailHeaderActions/JobDetailHeaderActions";
export * from "./compositions/JobLogReference/JobLogReference";
export * from "./compositions/JobOutcome/JobOutcome";
export * from "./compositions/JobRecord/JobRecord";
export * from "./compositions/JobRowStacked/JobRowStacked";
export * from "./compositions/Panel/Panel";
export * from "./compositions/ReviewComments/ReviewComments";
export * from "./compositions/ReviewDecision/ReviewDecision";
export * from "./compositions/VerdictSheet/VerdictSheet";
export * from "./compositions/ConfidenceSheet/ConfidenceSheet";
export * from "./compositions/ViewSheet/ViewSheet";
export * from "./compositions/Sidebar/Sidebar";
export * from "./compositions/StatsPanel/StatsPanel";
export * from "./compositions/StepActivityMark/StepActivityMark";
export * from "./compositions/StepBar/StepBar";
export * from "./compositions/TaskMark/TaskMark";
export * from "./compositions/TransitionHistory/TransitionHistory";
export * from "./compositions/UnifiedDiff/UnifiedDiff";
export * from "./compositions/WorkflowDiagram/WorkflowDiagram";
export * from "./compositions/WorkflowRail/WorkflowRail";
export * from "./compositions/ActivityInstrument/ActivityInstrument";
export * from "./compositions/FootprintInstrument/FootprintInstrument";

// What every job detail render shares. The five screens that took it are gone:
// job detail is one arrangement, and the five were the defect.
export * from "./screens/detail";
export * from "./screens/absent";

// The shell. Rail, panel and dock — the frame every screen mounts inside. A
// composition, so its story renders it from props like any other; the screens
// it frames are drawn by the app on a mock Fleet, #1225.
export * from "./compositions/TheShell/TheShell";

// The title row `TheShell` draws above the rail and the panel. #1087.
export * from "./compositions/TitleBar/TitleBar";

// The error treatment. Its own group, because an error is Armada failing and a
// failed Job is Armada working — the vocabulary that keeps the two apart is
// not a primitive and is not a composition of one.
export * from "./errors/ErrorCode/ErrorCode";
// The namespace Bridge mints its own codes in, beside the chip that draws
// them. Bridge's faults never cross the wire, so nothing hands them a code —
// and the code is one of the two channels separating an error from a status.
export * from "./errors/ErrorCode/codes";
export * from "./errors/ErrorNotice/ErrorNotice";
export * from "./errors/FileAnIssue/FileAnIssue";

// The run — the workflow as a tree on job detail. Not the rail: a rail drew
// every step's gate rows inline, and a step's gates are the phase strip's now.
export * from "./compositions/RunTree/RunTree";

// Where this step is. Each stage is a control, and Checks and the Judge are
// drawn as the different things they are.

// The activity log — the Drone's turns, Armada's injected turns and Fleet's
// own events, in one stream, every entry naming who.
export * from "./compositions/ActivityLog/ActivityLog";

// The step's story — Drone instructions, Activity log, Produced. Opening one
// collapses the others to their header line.
export * from "./compositions/StepStory/StepStory";

// The two chips a step's facts are made of. A fact is a value; a path is the
// one value that keeps its filename at every width.
export * from "./compositions/FactChip/FactChip";
export * from "./compositions/PathChip/PathChip";

// One row of the run — the step, its mark, its elapsed figure, and the short
// facts the chevron opens. The tree composes these; it does not draw a row.
export * from "./compositions/StepRow/StepRow";

// One row of Where things are. A path opens where it lives; an identifier
// copies; and the label column says which is which before the value is read.
export * from "./compositions/WhereRow/WhereRow";

// The step's story. A chapter collapses to its header line and never to
// nothing; a log entry opens in place to its payload, and every line opens.
export * from "./compositions/Chapter/Chapter";
export * from "./compositions/LogEntry/LogEntry";

// What a stage of the phase strip opens to — what that tier is, what it is
// waiting on and where it stands. The standing sentence for each tier lives
// here, written once, because it is the same on every Job.
export * from "./compositions/PhaseCard/PhaseCard";

// The vocabulary: the verb, the glyph and the status token each variant of the
// domain renders as, generated from `crates/core-model/domain/`.
//
// **It lives here and not in `@armada/protocol` because it is a rendering.** A
// glyph is a React component, so the vocabulary imports `lucide-react`, and the
// wire package is the one thing every other package depends on and imports
// nothing. The wire says `escalated`; this says how to draw it.
//
// Emitted once. It was written twice — here and into the app — because the
// dependency only ran one way and a story could not reach the app's copy. Now
// both depend on this.
export * from "./generated/vocabulary";

// How the domain is counted in words.
export * from "./lexicon";

// What a word naming an Armada concept means, in one sentence each. Exported
// because Bridge draws its own hovers off the same vocabulary the components
// do, and a second copy of the sentence explaining a Check is the defect this
// module exists to prevent.
export * from "./concepts";

// Every act, its verb, its glyph and its binding, generated from
// `crates/core-model/domain/actions.toml`. Here rather than a layer up for
// `vocabulary.ts`'s reason: the glyph is a React component, so the file
// belongs with the components that draw it, and a story has to be able to
// reach the real map rather than a fixture of one.
//
// One line and not two: `actions.ts` re-exports the generated map beside the
// aliases and the context filter, which are Bridge's readings of it and not
// the registry's.
export * from "./actions";

// Whether Cmd is held, for a control that carries a Global-tier binding to
// show its own `Kbd` badge. `TheShell` is the one provider; any control
// beneath it reads `useShortcutReveal()` rather than taking a prop for it.
export * from "./shortcut-reveal";

// `⌘Enter` sends, in every box a message is typed into. The registry holds the
// binding; this is the keystroke test and the badge the two composers share.
export * from "./send-message";

// The trackpad's answer to a press, a no-op until Bridge's renderer provides one.
export * from "./haptics";

// The token specimens: what a value looks like, read off the running sheet.
export * from "./foundations/Tokens/Tokens";

// What Armada told the Drone, in the blocks Fleet wrote it in. The chapter
// preview drew it as one paragraph and every newline collapsed to a space —
// #306.
export * from "./compositions/DroneBrief/DroneBrief";

// Dispatch a job by describing the work. The Job proposer answers the title,
// the workflow and the split, so the form behind `Enter by hand` is the
// override rather than the path.
export * from "./compositions/DispatchRequest/DispatchRequest";

// One worktree Fleet is holding, and the test it did not pass. The reasons are
// the component: not-provably-safe is one word for four situations a person
// answers differently, and each wants different facts in front of the decision.
export * from "./compositions/HeldWorktree/HeldWorktree";

// What one Job holds on the machine, and the act that goes and looks. Not a
// debug panel: the first thing on it is a sentence answering *is this working*,
// because the moment it is for is a person worried about a Job.
export * from "./compositions/JobResources/JobResources";
export * from "./compositions/ManifestNotice/ManifestNotice";
export * from "./compositions/ManifestForm/ManifestForm";
// The calls a stopped job was refused, with the command each one was on. The
// evidence for `blocked_by_policy`, which named a policy and nothing it stopped.
export * from "./compositions/Refusals/Refusals";

// What a Check's suite actually asserted. `exit 0 · 315 passed` cannot be
// audited: a suite can go green by deleting the assertion that was failing,
// and the list of what ran is the only place that is visible.
export * from "./compositions/AssertionSet/AssertionSet";

// A Check's own stdout, read where the Check is. Bridge reads the run log and
// keeps nothing — no new store, no retention policy, and it goes when the Job
// is cleaned up.
export * from "./compositions/ConsoleOutput/ConsoleOutput";

// What each Check came to, and the output behind it. A Check result is an
// evidence record: it has a kind and a file, and the only difference from a
// Drone's evidence is who produced it.
export * from "./compositions/CheckRuns/CheckRuns";

// The panel's answer, as criteria against judges. The measured band is the
// veto-only contract drawn: it says which parts of a verdict rest on a machine
// and which on a model.
export * from "./compositions/JudgeVerdicts/JudgeVerdicts";

// A refusal, whole. The overlap line is why it exists — one veto out of three
// means two opposite things, and comparing what each judge cited is what tells
// them apart.
export * from "./compositions/JudgeRefusal/JudgeRefusal";

// Every pointer the panel made. A judgment is mostly a set of pointers into
// other artifacts, which makes this the densest navigation surface on the
// screen.
export * from "./compositions/JudgeCitations/JudgeCitations";

// What the panel was shown. A panel is only a panel if the judges ran
// independently on identical inputs, and the digest is the evidence for it.
export * from "./compositions/JudgeInputs/JudgeInputs";

// Everything else the step produced, as selectors into the viewer. Not
// FactChip: a fact chip is a value being read, and these are controls.
export * from "./compositions/EvidenceStrip/EvidenceStrip";

// One artifact at full size, on the layer that can hold it. A check's output,
// a patch, a judgment: one viewer, and everything on the screen points into
// it. The sheet for the reason the activity log is one.
export * from "./compositions/EvidenceSheet/EvidenceSheet";

// A passage held to a few lines. The control exists only where the text
// actually overflows — measured, because the same words clamp at one width and
// not at another, and a *View more* that reveals nothing is a surface lying.
export * from "./compositions/Clamped/Clamped";

// The Job's pulse, in a few lines. The full reading was the largest thing
// in the run column and answered a question nobody had asked yet; this says
// whether anything is wrong, and opens the reading when the answer is yes.
export * from "./compositions/JobHoldsSummary/JobHoldsSummary";

// That reading, on a trailing sheet. `JobResources` unchanged inside it — a
// new home rather than an edit.
export * from "./compositions/JobHoldsSheet/JobHoldsSheet";
export * from "./compositions/PlanTaskSheet/PlanTaskSheet";

// The plan read as the groups it will run in, and the boundary each ends at.
export * from "./compositions/PlanBoard/PlanBoard";

// Every setting a person can change on a running Job, on the same layer, and
// the header's way into it.
export * from "./compositions/JobSettings/JobSettings";
export * from "./compositions/FleetSettings/FleetSettings";
export * from "./compositions/KitServers/KitServers";
export * from "./compositions/KitSetup/KitSetup";
export * from "./compositions/MachineSettings/MachineSettings";

// Asking a Job to show its work again, and every set a press kept beside the
// step's own frames. Each set is an unchanged `FramesShown`.
export * from "./compositions/ShownAgain/ShownAgain";
// Journey 9's rehearsal, run in the Job's own worktree. Writes no Evidence and
// moves nothing on the Job — a rehearsal, never a verdict.
export * from "./compositions/RunSheet/RunSheet";
export * from "./compositions/RunPage/RunPage";
export * from "./compositions/RunDiffSheet/RunDiffSheet";
// A step's work with runs of one tool folded to a line. The rows inside a group
// are the caller's own log, drawn unfolded everywhere else.
export * from "./compositions/WorkGroups/WorkGroups";
// A step as one timeline: the phases in the order they happened, repeated for
// each attempt, with the earlier ones folded. Replaces the strip and the story.
export * from "./compositions/StepTimeline/StepTimeline";
export * from "./compositions/ManifestFile/ManifestFile";
// Drift and Verify on the Manifest surface — Journey 9's *Verify*, as two
// panels of their own rather than two halves of one.
export * from "./compositions/DriftPanel/DriftPanel";
export * from "./compositions/VerifyPanel/VerifyPanel";
// Setup — Journey 3: the picker, a line's source, and the port add form both surfaces mount.
export * from "./compositions/PortAddForm/PortAddForm";
export * from "./compositions/Provenance/Provenance";
export * from "./compositions/SetupPicker/SetupPicker";
// Locate — Journey 3's *Getting in*: a repository added by folder or cloned from a URL.
export * from "./compositions/LocateForm/LocateForm";
export * from "./compositions/ProposalSheet/ProposalSheet";
export * from "./compositions/ValuePopover/ValuePopover";
// Overview's summary strip — every panel's count above the fold.
export * from "./compositions/OverviewSummaryStrip/OverviewSummaryStrip";
// The card a gaming flag holds a step with, and the two answers to it. #1079.
export * from "./compositions/HeldFlag/HeldFlag";
// The message box fixed under an activity log — sending a redirect without
// leaving the log to reach the step header's button. #1154.
export * from "./compositions/DroneMessageBox/DroneMessageBox";
// A step's work as the Drone told it, under the plan task it served. #1185.
export * from "./compositions/WorkNarration/WorkNarration";
// A Studio's nodes, and the whiteboard React Flow draws them on. #1286.
export * from "./compositions/StudioCapture/StudioCapture";
export * from "./compositions/CaptureBar/CaptureBar";
export * from "./compositions/StudioAddNode/StudioAddNode";
export * from "./compositions/StudioName/StudioName";
export * from "./compositions/StudioPicked/StudioPicked";
export * from "./compositions/StudioNode/StudioNode";
export * from "./compositions/StudioWhiteboard/StudioWhiteboard";
// A label and its figure in one aligned column — Pulse and the Fleet panel.
export * from "./compositions/FigureList/FigureList";
// What the Job has changed, as its own panel beside the run. #1187.
export * from "./compositions/ProducedPanel/ProducedPanel";
// A tool's name, in the colour of what the call does. #1196.
export * from "./compositions/ToolName/ToolName";
// A Note's frame, opened over the Studios surface. #1352.
export * from "./compositions/StudioFrameSheet/StudioFrameSheet";
// Drift's reading and Verify's run, each on its own layer instead of resident
// above the runner. #1383.
export * from "./compositions/DriftSheet/DriftSheet";
export * from "./compositions/VerifySheet/VerifySheet";
// The dispatch form's optional settings, and what else is writing where a
// request would. #1540.
export * from "./compositions/DispatchSettings/DispatchSettings";
export * from "./compositions/WhatElseIsRunning/WhatElseIsRunning";
