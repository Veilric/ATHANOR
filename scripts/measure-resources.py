#!/usr/bin/env python3
"""Linux process CPU/RSS sampler. Run once for each panel state; no app data is read."""
import argparse
import json
import os
from pathlib import Path
import time

p = argparse.ArgumentParser()
p.add_argument("pid", type=int)
p.add_argument("--seconds", type=float, default=60)
p.add_argument("--state", required=True, choices=["hidden", "compact", "expanded"])
p.add_argument("--output", type=Path)
a = p.parse_args()
if a.seconds <= 0:
    p.error("seconds must be positive")
proc = Path(f"/proc/{a.pid}")

def sample():
    fields = (proc / "stat").read_text().rsplit(")", 1)[1].split()
    status = (proc / "status").read_text().splitlines()
    rss = int(next(line.split()[1] for line in status if line.startswith("VmRSS:")))
    return int(fields[11]) + int(fields[12]), rss

start_ticks, start_rss = sample()
start = time.monotonic()
peak = start_rss
while time.monotonic() - start < a.seconds:
    time.sleep(min(0.25, a.seconds - (time.monotonic() - start)))
    _, rss = sample()
    peak = max(peak, rss)
end_ticks, end_rss = sample()
elapsed = time.monotonic() - start
result = {"pid": a.pid, "state": a.state, "seconds": elapsed,
          "cpu_percent_one_core": (end_ticks - start_ticks) / os.sysconf("SC_CLK_TCK") / elapsed * 100,
          "rss_start_mib": start_rss / 1024, "rss_end_mib": end_rss / 1024,
          "rss_peak_mib": peak / 1024, "gpu_memory": "not included; measure separately"}
value = json.dumps(result, indent=2)
if a.output:
    a.output.parent.mkdir(parents=True, exist_ok=True)
    a.output.write_text(value + "\n")
print(value)
