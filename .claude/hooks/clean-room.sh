#!/bin/sh
# PreToolUse: default-deny the source repo, for every agent but the harvester.
#
# Why this exists (ARCHITECTURE.md §2.7, PHASES.md §8.1): charkit is built
# greenfield so it cannot inherit the source repo's assumptions. "The
# implementer never opens that repo" is a sentence in a prompt, and a sentence
# in a prompt is policed rather than structural. This is the structural half.
#
# It ships in phase 1 rather than phase 3, where it is first needed, because a
# guard added at the moment it is needed has already been unenforced for every
# commit before that. Note .gitignore deliberately does NOT ignore .claude/ —
# a hook that is not committed enforces nothing.
#
# WHY SHELL, AND WHY IT FAILS CLOSED. A hook that cannot run must not silently
# permit: a non-zero exit that is not 2 is a non-blocking error, and the tool
# call proceeds. /bin/sh is always present, so there is no interpreter to be
# missing. Every exit below is deliberate.
#
# WHY IT DOES NOT PARSE JSON. §2.7 requires inspecting `tool_input` as a whole,
# because the path can arrive through Read, Glob, Grep, or a Bash line
# containing rg, find, cat or python -c. A field-by-field parse covers the
# polite failure and misses the interesting one, so for reading tools this
# matches the raw payload.
#
# THE ONE NARROWING, AND ITS REASON. For tools that write, only the target path
# is matched — not the content. The corpus legitimately discusses the source
# repo by name (PLAN.md §1 cites it as the project's one piece of evidence), so
# matching content would deny an agent editing this project's own documents,
# which is ordinary work and not a clean-room breach. Reading is the vector;
# writing a document that mentions a path is not.

set -u

GUARDED='Development/chariot'
HARVESTER='harvester'

input=$(cat)

field() {
	printf '%s' "$input" |
		grep -o "\"$1\"[[:space:]]*:[[:space:]]*\"[^\"]*\"" |
		head -n 1 |
		sed 's/.*"\([^"]*\)"$/\1/'
}

deny() {
	# The documented deny shape. Exit 0: the decision *is* the output, and a
	# non-zero exit here would be reported as a broken hook rather than as a
	# refusal.
	printf '%s\n' "{\"hookSpecificOutput\":{\"hookEventName\":\"PreToolUse\",\"permissionDecision\":\"deny\",\"permissionDecisionReason\":\"$1\"}}"
	exit 0
}

tool=$(field tool_name)
agent=$(field agent_type)

# An allowlist, not a denylist: an agent type added later is denied by default
# rather than silently permitted.
if [ "$agent" = "$HARVESTER" ]; then
	exit 0
fi

REASON="Denied by the clean-room hook: only phase 3's $HARVESTER agent may read the source repo (docs/ARCHITECTURE.md section 2.7). If a phase feels like it needs to look, the plan is underspecified - fix the plan."

case "$tool" in
Read | Glob | Grep | Bash | LS | NotebookRead | Task | WebFetch)
	# Readers: the whole payload, because the path can arrive anywhere in it.
	if printf '%s' "$input" | grep -q "$GUARDED"; then
		deny "$REASON"
	fi
	;;
Write | Edit | MultiEdit | NotebookEdit)
	# Writers: the target only. See the note above.
	target=$(field file_path)
	case "$target" in
	*"$GUARDED"*) deny "$REASON" ;;
	esac
	;;
esac

exit 0
