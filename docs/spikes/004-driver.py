import json, os, subprocess, sys, threading, time

SP = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.join(SP, "spike4-repo")
OUT = os.path.join(SP, "spike4-transcript.ndjson")

TASK = ("Create ten files in this directory named step01.txt through step10.txt. "
        "Each must contain the single word DONE followed by its own number, e.g. 'DONE 1'. "
        "Create them ONE AT A TIME, and after writing each one read it back to confirm it. "
        "Do not batch them. Work through all ten in order.")

INJECT = ("STOP. Change of instructions: do not create any more stepNN.txt files. "
          "Instead, immediately write a file named INTERRUPTED.txt whose contents are "
          "the number of stepNN.txt files you had created before this message, and then finish.")

cmd = ["claude", "-p",
       "--input-format", "stream-json",
       "--output-format", "stream-json",
       "--verbose", "--replay-user-messages",
       "--model", "sonnet",
       "--permission-mode", "acceptEdits",
       "--allowedTools", "Read,Write,Bash(ls:*)"]

p = subprocess.Popen(cmd, cwd=REPO, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                     stderr=open(os.path.join(SP, "spike4-stderr.txt"), "wb"), text=True, bufsize=1)

t0 = time.time()
events = []          # (elapsed, raw line)
inject_at = [None]
first_after = [None]
seen_writes = [0]
done = threading.Event()

def send(text):
    msg = {"type": "user", "message": {"role": "user", "content": text}}
    p.stdin.write(json.dumps(msg) + "\n")
    p.stdin.flush()

def reader():
    for line in p.stdout:
        el = time.time() - t0
        events.append((el, line.rstrip("\n")))
        try:
            d = json.loads(line)
        except Exception:
            continue
        # count Write tool calls to know work has really started
        m = d.get("message") or {}
        c = m.get("content")
        if isinstance(c, list):
            for b in c:
                if isinstance(b, dict) and b.get("type") == "tool_use" and b.get("name") == "Write":
                    seen_writes[0] += 1
        if d.get("type") == "result":
            done.set()
    done.set()

threading.Thread(target=reader, daemon=True).start()

send(TASK)

# wait until it has genuinely started working: 3 Write calls, or 45s
deadline = time.time() + 45
while seen_writes[0] < 3 and time.time() < deadline and not done.is_set():
    time.sleep(0.2)

if done.is_set():
    print("RUN FINISHED BEFORE INJECTION — writes seen:", seen_writes[0])
else:
    inject_at[0] = time.time() - t0
    writes_at_inject = seen_writes[0]
    send(INJECT)
    print(f"INJECTED at t={inject_at[0]:.2f}s after {writes_at_inject} Write calls")

done.wait(timeout=240)
try:
    p.stdin.close()
except Exception:
    pass
try:
    p.wait(timeout=30)
except Exception:
    p.kill()

with open(OUT, "w") as f:
    for el, line in events:
        f.write(line + "\n")
with open(OUT + ".times", "w") as f:
    for el, line in events:
        try:
            d = json.loads(line)
            tag = f"{d.get('type')}/{d.get('subtype','')}"
        except Exception:
            tag = "?"
        f.write(f"{el:8.3f}\t{tag}\n")

print("exit:", p.returncode, "| events:", len(events), "| inject_at:", inject_at[0])
print("files:", sorted(os.listdir(REPO)))
