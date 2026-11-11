#!/usr/bin/env python3
"""Synthetic local socketpair probe; never contacts a network endpoint."""
import json
from pathlib import Path
import socket
import subprocess
import sys


def probe(root):
    runner = Path(root).resolve() / "scripts/offline-exec.py"
    rows = []
    marker = b"synthetic-offline-stdio-canary"
    for fd, stream in enumerate(("stdin", "stdout", "stderr")):
        for child in (False, True):
            left, right = socket.socketpair()
            try:
                code = f"import os; os.write({fd}, {marker!r})"
                if child:
                    code = f"import subprocess,sys; subprocess.run([sys.executable,'-c',{code!r}],check=True)"
                kwargs = {stream: left}
                for other in ("stdin", "stdout", "stderr"):
                    if other != stream:
                        kwargs[other] = subprocess.DEVNULL
                result = subprocess.run([sys.executable, str(runner), sys.executable,
                                         "-c", code], timeout=10, **kwargs)
                left.close()
                right.settimeout(1)
                data = right.recv(4096)
                rows.append({"id": f"{stream}-{'child' if child else 'direct'}",
                             "exit": result.returncode,
                             "canary_forwarded": marker in data,
                             "pass": result.returncode != 0 and marker not in data})
            finally:
                left.close()
                right.close()
    return {"cases": rows, "passed": sum(row["pass"] for row in rows), "total": len(rows)}


if __name__ == "__main__":
    report = probe(sys.argv[1])
    print(json.dumps(report))
    sys.exit(0 if report["passed"] == report["total"] else 1)
