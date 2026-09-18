#!/usr/bin/env python3
"""Scrub one spike-19 case into the shape spike 18's transcripts keep: one
stream line per line, machine paths written `<spike>` and `~`, each `init`
inventory collapsed to a count, hook events reduced to their name, and every id,
signature and cost dropped.

usage: 019-scrub.py CASE_DIR OUT_JSONL SPIKE_ROOT
"""
import json, os, sys

case, out, root = sys.argv[1], sys.argv[2], sys.argv[3]
home = os.path.expanduser("~")
flat = root.replace("/", "-")


def scrub(value):
    if isinstance(value, str):
        return (value.replace(root, "<spike>").replace(flat, "<spike-flattened>")
                     .replace(home, "~"))
    if isinstance(value, list):
        return [scrub(item) for item in value]
    if isinstance(value, dict):
        return {k: scrub(v) for k, v in value.items()
                if k not in ("uuid", "session_id", "signature", "parent_tool_use_id")}
    return value


COUNTED = ("tools", "mcp_servers", "slash_commands", "skills", "plugins", "commands")

with open(out, "w") as f:
    for line in open(os.path.join(case, "session.jsonl")):
        entry = json.loads(line)
        event = scrub(json.loads(entry["line"]))
        if event.get("subtype") == "init":
            for key in COUNTED:
                if isinstance(event.get(key), list):
                    event[key] = len(event[key])
        if event.get("subtype") in ("hook_started", "hook_response"):
            event = {k: v for k, v in event.items()
                     if k in ("type", "subtype", "hook_name")}
        if event.get("type") == "result":
            for key in ("total_cost_usd", "usage", "modelUsage", "duration_api_ms"):
                event.pop(key, None)
        event["t"] = entry["t"]
        f.write(json.dumps(event) + "\n")
print(out)
