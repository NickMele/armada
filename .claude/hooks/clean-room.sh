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
# repo (PLAN.md §1 cites it as the project's one piece of evidence), so matching
# content would deny an agent editing this project's own documents, which is
# ordinary work and not a clean-room breach. Reading is the vector; writing a
# document that mentions a path is not. Every other tool name — one renamed
# between versions, an MCP tool nobody listed — takes the reading path, so the
# tool dimension is an allowlist too.
#
# WHERE THE GUARDED PATH COMES FROM, AND WHY NOT FROM HERE. charkit is public.
# A guard that names the repo it is guarding publishes the one thing it exists
# to keep out, and every clone inherits a path that is nobody else's. So the
# path is configuration, in one of two places:
#
#   CHARKIT_CLEAN_ROOM_PATH   a path fragment — `Development/acme-app` — which
#                             wins whenever it is exported, empty included.
#   .claude/clean-room.local  the same fragment on a line of its own, `#`
#                             comments and blanks skipped, first line used.
#                             Untracked (see .gitignore), and unlike the
#                             variable it survives a hook run that inherits no
#                             shell — an editor launched from the desktop.
#
# Neither set is not a failure to fail closed: it means no source repo has been
# named, so there is nothing to be outside of and the hook permits. Treating it
# as a failure is not even available — the empty fragment matched with
# `grep -F` is in every payload, so "deny when unconfigured" denies all work.

set -u

# Exported wins over the file, and exported-empty is a deliberate off switch —
# hence `+set` rather than `:-`, which cannot tell the two apart.
if [ -n "${CHARKIT_CLEAN_ROOM_PATH+set}" ]; then
	GUARDED=$CHARKIT_CLEAN_ROOM_PATH
else
	CONFIG="$(dirname "$0")/../clean-room.local"
	GUARDED=$(
		[ -r "$CONFIG" ] && grep -v '^[[:space:]]*#' "$CONFIG" |
			sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//' |
			grep -v '^$' | head -n 1
	)
fi

HARVESTER='harvester'

# Read first, exit second: closing stdin on a writer that is still sending gets
# reported as a broken hook, which is the shape this file spends its length
# avoiding.
input=$(cat)

# Nothing named, nothing to guard — and no payload scanned, on every tool call
# in every clone that never configures this.
[ -n "$GUARDED" ] || exit 0

# `field <container> <key>`: the value of a string field, at one exact place in
# the payload. `field "" tool_name` reads a top-level key; `field tool_input
# file_path` reads a key of the top-level `tool_input` object and nothing
# nested inside it. awk scans rather than matches, so a `{` or a `"` inside a
# string cannot move the scanner's idea of where it is.
#
# WHY IT SPLITS ON THE QUOTE RATHER THAN WALKING CHARACTERS. `content:` and
# `new_string:` carry whole files, and a character-at-a-time scan that builds
# its token by concatenation is quadratic — measured here at 21 s for a 512 KB
# `Write`, which is on its way to the 60 s hook timeout, and a hook that times
# out is the silent permit this file exists to prevent. `RS` does the splitting
# in C, so a string char nobody is looking for costs nothing: only structural
# text is walked, only a sought value is accumulated, and both are short.
field() {
	printf '%s' "$input" | awk -v container="$1" -v key="$2" '
		BEGIN {
			RS = "\""
			want_depth = (container == "") ? 1 : 2
			depth = 0; in_string = 0; expect_value = 0
			pending = ""; keep = 0; is_value = 0
		}
		in_string {
			if (keep && length(pending) < 4096) pending = pending $0
			# A closing quote preceded by an odd run of backslashes is escaped,
			# so the string continues into the next record.
			slashes = 0; end = length($0)
			while (slashes < end && substr($0, end - slashes, 1) == "\\") slashes++
			if (slashes % 2 == 1) {
				if (keep && length(pending) < 4096) pending = pending "\""
				next
			}
			in_string = 0
			if (is_value) {
				expect_value = 0
				if (keep) { print pending; exit }
			} else if (keep) last[depth] = pending
			next
		}
		{
			# Structural text only: everything the scanner does not branch on is
			# dropped in one pass, so what is walked is a handful of characters.
			structure = $0
			gsub(/[^]{},:[]/, "", structure)
			n = length(structure)
			for (i = 1; i <= n; i++) {
				c = substr(structure, i, 1)
				if (c == "{" || c == "[") {
					holder[depth + 1] = (expect_value ? last[depth] : "")
					depth++; expect_value = 0; last[depth] = ""
				}
				else if (c == "}" || c == "]") { depth--; expect_value = 0 }
				else if (c == ":") expect_value = 1
				else if (c == ",") expect_value = 0
			}
			in_string = 1; pending = ""; is_value = expect_value
			if (is_value) {
				keep = (depth == want_depth && last[depth] == key &&
					(container == "" || holder[depth] == container))
			} else {
				keep = (depth == want_depth || depth == want_depth - 1)
			}
		}
	'
}

# The one place the guarded path is matched, so a spelling that gets past one
# call site cannot get past another.
#
# The text handed here is raw JSON bytes — `field` returns what is between the
# quotes and the reading path matches the payload itself — so JSON's own escapes
# are still in it, and a backslash before the separator is the guarded fragment
# written a second way. `find -regex`, `sed s///` and `grep -E` all put one there
# in ordinary use. Dropping every backslash before matching can only make a path
# easier to see, never harder, so the normalisation errs towards denying. `tr`
# and `grep` are each one pass, which is what keeps this linear on a payload
# carrying a whole file.
#
# The empty-fragment check is redundant with the exit above and stays anyway: it
# is the difference between reordering this file and disabling it.
reaches_the_source_repo() {
	[ -n "$GUARDED" ] || return 1
	printf '%s' "$1" | tr -d '\\' | grep -q -F "$GUARDED"
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
		if reaches_the_source_repo "$target"; then
			deny "$REASON"
		fi
		exit 0
	fi
	;;
esac

# Everything else — Read, Glob, Grep, Bash, an MCP tool nobody listed, a tool
# renamed between versions: the whole payload, because the path can arrive
# anywhere in it.
if reaches_the_source_repo "$input"; then
	deny "$REASON"
fi

exit 0
