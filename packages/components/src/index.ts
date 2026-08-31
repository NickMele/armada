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

// Job detail — the three renders a Job's status chooses between. A screen was
// a story with hardcoded fixtures and nothing else could render it; these take
// props, and the stories render them with the same fixtures they held.
export * from "./screens/detail";
export * from "./screens/AFailedJobADeadEndReadAsOne/AFailedJobADeadEndReadAsOne";
export * from "./screens/AFinishedJobWhatItWasAndWhatItProduced/AFinishedJobWhatItWasAndWhatItProduced";
export * from "./screens/AJobAwaitingReviewTheDiffAndTheReplyAreOneLoop/AJobAwaitingReviewTheDiffAndTheReplyAreOneLoop";
export * from "./screens/ARunningJob/ARunningJob";

// Observe — one Job's turns, read while they are still being written. A screen
// rather than a region of job detail: it is opened on one Job deliberately and
// closed, which is the shape the turn-level detail rule already has.
export * from "./screens/WatchingADroneWork/WatchingADroneWork";

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
