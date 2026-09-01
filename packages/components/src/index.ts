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
export * from "./primitives/Input/Input";
export * from "./primitives/Kbd/Kbd";
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
export * from "./compositions/BoardControls/BoardControls";
export * from "./compositions/BoardEmptyState/BoardEmptyState";
export * from "./compositions/ChangedFiles/ChangedFiles";
export * from "./compositions/CriterionVerdicts/CriterionVerdicts";
export * from "./compositions/DroneQuestion/DroneQuestion";
export * from "./compositions/EvidenceCard/EvidenceCard";
export * from "./compositions/DroneTurns/DroneTurns";
export * from "./compositions/EvidenceTrail/EvidenceTrail";
export * from "./compositions/FailureNotice/FailureNotice";
export * from "./compositions/GamingFlags/GamingFlags";
export * from "./compositions/JobComposer/JobComposer";
export * from "./compositions/JobBrief/JobBrief";
export * from "./compositions/JobDetailHeaderActions/JobDetailHeaderActions";
export * from "./compositions/JobLogReference/JobLogReference";
export * from "./compositions/JobOutcome/JobOutcome";
export * from "./compositions/JobRecord/JobRecord";
export * from "./compositions/JobRowStacked/JobRowStacked";
export * from "./compositions/ReviewDecision/ReviewDecision";
export * from "./compositions/Sidebar/Sidebar";
export * from "./compositions/StatusBar/StatusBar";
export * from "./compositions/StepActivityMark/StepActivityMark";
export * from "./compositions/StepBar/StepBar";
export * from "./compositions/TransitionHistory/TransitionHistory";
export * from "./compositions/UnifiedDiff/UnifiedDiff";
export * from "./compositions/WorkflowRail/WorkflowRail";

// What every job detail render shares. The five screens that took it are gone:
// job detail is one arrangement, and the five were the defect.
export * from "./screens/detail";

// The three journey screens, lifted for the reason the job detail three were:
// a story with hardcoded fixtures is a screen nothing outside Storybook can
// render. Each takes the regions it composes as props.
export * from "./screens/DispatchAJobFullWithTheM1SubsetMarked/DispatchAJobFullWithTheM1SubsetMarked";
export * from "./screens/FirstLaunch/FirstLaunch";
export * from "./screens/TheListSixStatesOneRowShape/TheListSixStatesOneRowShape";

// The shell. Rail, panel and status bar — the frame the three above mount
// inside. A screen like them, and lifted for the same reason: Bridge needs the
// frame as a component, and the story renders it from the drawing's fixture.
export * from "./screens/TheShell/TheShell";

// The error treatment. Its own group, because an error is Armada failing and a
// failed Job is Armada working — the vocabulary that keeps the two apart is
// not a primitive and is not a composition of one.
export * from "./errors/ErrorCode/ErrorCode";
export * from "./errors/ErrorNotice/ErrorNotice";
export * from "./errors/FileAnIssue/FileAnIssue";

// The run — the workflow as a tree on job detail. Not the rail: a rail drew
// every step's gate rows inline, and a step's gates are the phase strip's now.
export * from "./compositions/RunTree/RunTree";

// Where this step is. Each stage is a control, and Checks and the Judge are
// drawn as the different things they are.
export * from "./compositions/PhaseStrip/PhaseStrip";

// The activity log — the Drone's turns, Armada's injected turns and Fleet's
// own events, in one stream, every entry naming who.
export * from "./compositions/ActivityLog/ActivityLog";

// The step's story — Drone instructions, Activity log, Produced. Opening one
// collapses the others to their header line.
export * from "./compositions/StepStory/StepStory";

// Inside a job — the one arrangement, at every state. The screen #186 built:
// the run as a tree, the selected step in the panel, its story in order.
export * from "./screens/InsideAJobOneArrangementAtEveryState/InsideAJobOneArrangementAtEveryState";

// The two chips a step's facts are made of. A fact is a value; a path is the
// one value that keeps its filename at every width.
export * from "./compositions/FactChip/FactChip";
export * from "./compositions/PathChip/PathChip";
