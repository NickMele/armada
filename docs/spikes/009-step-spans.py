#!/usr/bin/env python3
"""Reproduce every table in 009-how-long-does-a-step-take.md.

Two sources, both machine state rather than repository content, so neither is
committed beside this script:

  * the store, whose `job_events` rows carry a step id and an instant. A
    `step_transition` from `not_started -> running` to `running -> advanced` is
    the interval `StepNorms::wall_clock` is compared against.
  * `.armada/transcripts/*.jsonl`, one file per Drone, every event with the
    instant Fleet saw it.

Usage: 009-step-spans.py [repository] [store]
Defaults to the current directory and the store under the machine directory.
"""

import collections
import datetime
import glob
import json
import os
import sqlite3
import statistics
import sys

REPO = sys.argv[1] if len(sys.argv) > 1 else os.getcwd()
STORE = (
    sys.argv[2]
    if len(sys.argv) > 2
    else os.path.expanduser("~/Library/Application Support/Armada/armada.db")
)
CALL_NORM = 60


def at(text):
    return datetime.datetime.fromisoformat(text.replace("Z", "+00:00"))


def pct(values, p):
    values = sorted(values)
    k = (len(values) - 1) * p
    low = int(k)
    high = min(low + 1, len(values) - 1)
    return values[low] + (values[high] - values[low]) * (k - low)


def summary(label, values):
    return (
        f"{label:44} n={len(values):3} median={statistics.median(values):8.0f}"
        f" p90={pct(values, .9):8.0f} p95={pct(values, .95):8.0f} max={max(values):8.0f}"
    )


con = sqlite3.connect(f"file:{STORE}?mode=ro", uri=True)
status = dict(con.execute("select job_id, status from jobs"))
workflow = dict(con.execute("select job_id, workflow_id from jobs"))
last_seen = {
    job: at(when) for job, when in con.execute("select job_id, max(at) from job_events group by job_id")
}

# One span per time a step was entered. A step can be entered more than once.
spans, entered = [], {}
for job, step, moved_to, when in con.execute(
    "select job_id, step_id, state_to, at from job_events where kind='step_transition' order by seq"
):
    if moved_to == "running":
        entered[(job, step)] = at(when)
    elif (job, step) in entered:
        spans.append((job, step, entered.pop((job, step)), at(when), True))
# A step still running when its Job ended never advanced: it ran at least this long.
for (job, step), began in entered.items():
    spans.append((job, step, began, last_seen[job], False))

# Which Drone belonged to which Job, from Fleet's own log envelopes.
drone_of = {}
for path in glob.glob(os.path.join(REPO, ".armada/logs/*.jsonl")):
    for line in open(path):
        line = json.loads(line)
        if line.get("drone_id"):
            drone_of[line["drone_id"]] = line["job_id"]

# Rows written before the transcript carried a step label have none, so a call
# is attributed to the step whose interval it falls in — the same subtraction
# `Working::calls_this_step` does against a baseline taken at step start.
heard = collections.defaultdict(list)
for path in sorted(glob.glob(os.path.join(REPO, ".armada/transcripts/*.jsonl"))):
    job = drone_of.get(os.path.basename(path).split(".")[0])
    if not job:
        continue
    for line in open(path):
        line = json.loads(line)
        heard[job].append((at(line["ts"]), line["event"]))

rows = []
for job, step, began, ended, finished in sorted(spans, key=lambda span: span[2]):
    said = sorted(when for when, _ in heard.get(job, []) if began <= when <= ended)
    calls = sum(1 for when, event in heard.get(job, []) if began <= when <= ended and event == "called")
    marks = [began] + said + [ended]
    rows.append(
        {
            "job": job,
            "step": step,
            "workflow": workflow[job],
            "status": status[job],
            "seconds": (ended - began).total_seconds(),
            "calls": calls,
            "quiet": max((marks[i + 1] - marks[i]).total_seconds() for i in range(len(marks) - 1)),
            "tail": (ended - said[-1]).total_seconds() if said else (ended - began).total_seconds(),
            "finished": finished,
        }
    )

done = [row for row in rows if row["finished"]]
stuck = [row for row in rows if not row["finished"]]

print("== every step, in the order it was entered ==")
print(f"{'job':8} {'workflow':9} {'step':11} {'secs':>7} {'calls':>6} {'quiet':>7} {'tail':>6}  finished")
for row in rows:
    print(
        f"{row['job'][-6:]:8} {row['workflow']:9} {row['step']:11} {row['seconds']:7.0f}"
        f" {row['calls']:6} {row['quiet']:7.0f} {row['tail']:6.0f}  {row['finished']}"
    )

print("\n== completed steps ==")
print(summary("wall clock, whole step", [row["seconds"] for row in done]))
print(summary("tool calls", [row["calls"] for row in done]))
print(summary("longest silence inside the step", [row["quiet"] for row in done]))
print(summary("silence before the step ended", [row["tail"] for row in done]))

print("\nby step id:")
by_step = collections.defaultdict(list)
for row in done:
    by_step[row["step"]].append(row["seconds"])
for step, seconds in sorted(by_step.items(), key=lambda pair: -len(pair[1])):
    print(summary(f"  {step}", seconds))

print("\n== what a ceiling would have cost ==")
for ceiling in (600, 900, 1200, 1500, 1800):
    over = [row for row in done if row["seconds"] > ceiling]
    talkative = [row for row in over if row["calls"] > CALL_NORM]
    print(f"  {ceiling:5}s -> {len(over)} of {len(done)} completed steps, {len(talkative)} already over the call norm")

print("\n== steps that never advanced ==")
for row in sorted(stuck, key=lambda row: -row["seconds"]):
    print(
        f"  {row['step']:11} >= {row['seconds']:7.0f}s  calls={row['calls']:4}"
        f"  longest silence={row['quiet']:7.0f}s  {row['status']}"
    )
