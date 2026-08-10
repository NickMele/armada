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
# WHY IT DOES NOT PATTERN-MATCH THE DECISION. §2.7 requires inspecting
# `tool_input` as a whole, because the path can arrive through Read, Glob, Grep,
# or a Bash line containing rg, find, cat or python -c. A field-by-field parse
# covers the polite failure and misses the interesting one, so the decision for
# reading tools matches the raw payload.
#
# WHY THE FIELD READS ARE SCOPED. The two fields that steer the decision —
# `agent_type` and the writing tools' target — are read at a known depth rather
# than greped for anywhere, because everything under `tool_input` is attacker
# text. A command string or a nested tool payload containing
# `"agent_type":"harvester"` must not be able to buy itself the harvester's
# allowance, and a first-match grep decides that on key order alone.
#
# THE ONE NARROWING, AND ITS REASON. For tools that write, only the target path
# is matched — not the content. The corpus legitimately discusses the source
# repo by name (PLAN.md §1 cites it as the project's one piece of evidence), so
# matching content would deny an agent editing this project's own documents,
# which is ordinary work and not a clean-room breach. Reading is the vector;
# writing a document that mentions a path is not. Every other tool name — one
# renamed between versions, an MCP tool nobody listed — takes the reading path,
# so the tool dimension is an allowlist too.

set -u

GUARDED='Development/chariot'
HARVESTER='harvester'

input=$(cat)

# `field <container> <key>`: the value of a string field, at one exact place in
# the payload. `field "" tool_name` reads a top-level key; `field tool_input
# file_path` reads a key of the top-level `tool_input` object and nothing
# nested inside it. awk scans rather than matches, so a `{` or a `"` inside a
# string cannot move the scanner's idea of where it is.
field() {
	printf '%s' "$input" | awk -v container="$1" -v key="$2" '
		{ payload = payload $0 "\n" }
		END {
			want_depth = (container == "") ? 1 : 2
			depth = 0; in_string = 0; escaped = 0
			token = ""; expect_value = 0
			n = length(payload)
			for (i = 1; i <= n; i++) {
				c = substr(payload, i, 1)
				if (in_string) {
					if (escaped) { token = token c; escaped = 0 }
					else if (c == "\\") { escaped = 1 }
					else if (c == "\"") {
						in_string = 0
						if (expect_value) {
							expect_value = 0
							if (depth == want_depth && last[depth] == key &&
							    (container == "" || holder[depth] == container)) {
								print token; exit
							}
						} else { last[depth] = token }
					}
					else { token = token c }
					continue
				}
				if (c == "\"") { in_string = 1; token = "" }
				else if (c == "{" || c == "[") {
					holder[depth + 1] = (expect_value ? last[depth] : "")
					depth++; expect_value = 0; last[depth] = ""
				}
				else if (c == "}" || c == "]") { depth--; expect_value = 0 }
				else if (c == ":") expect_value = 1
				else if (c == ",") expect_value = 0
			}
		}
	'
}

deny() {
	# The documented deny shape. Exit 0: the decision *is* the output, and a
	# non-zero exit here would be reported as a broken hook rather than as a
	# refusal.
	printf '%s\n' "{\"hookSpecificOutput\":{\"hookEventName\":\"PreToolUse\",\"permissionDecision\":\"deny\",\"permissionDecisionReason\":\"$1\"}}"
	exit 0
}

tool=$(field "" tool_name)
agent=$(field "" agent_type)

# An allowlist, not a denylist: an agent type added later is denied by default
# rather than silently permitted.
if [ "$agent" = "$HARVESTER" ]; then
	exit 0
fi

REASON="Denied by the clean-room hook: only phase 3's $HARVESTER agent may read the source repo (docs/ARCHITECTURE.md section 2.7). If a phase feels like it needs to look, the plan is underspecified - fix the plan."

case "$tool" in
Write | Edit | MultiEdit | NotebookEdit)
	# Writers: the target only. See the note above. `NotebookEdit` spells its
	# target `notebook_path`, and a target this hook cannot find is a payload it
	# does not understand — which takes the reading path rather than the
	# narrowing, because the narrowing is only justified where the target is
	# known.
	target=$(field tool_input file_path)
	[ -n "$target" ] || target=$(field tool_input notebook_path)
	if [ -n "$target" ]; then
		case "$target" in
		*"$GUARDED"*) deny "$REASON" ;;
		esac
		exit 0
	fi
	;;
esac

# Everything else — Read, Glob, Grep, Bash, an MCP tool nobody listed, a tool
# renamed between versions: the whole payload, because the path can arrive
# anywhere in it.
if printf '%s' "$input" | grep -q "$GUARDED"; then
	deny "$REASON"
fi

exit 0
