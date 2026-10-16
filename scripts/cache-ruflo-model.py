#!/usr/bin/env python3
"""Prefetch only checksum-pinned public model files, separately from task execution."""
import hashlib
import json
import os
from pathlib import Path
import sys
import tempfile
import urllib.request

if len(sys.argv) != 2:
    raise SystemExit("Usage: cache-ruflo-model.py MODEL_ROOT")
manifest = json.loads(Path(__file__).with_name("ruflo-model-manifest.json").read_text())
root = Path(sys.argv[1]).resolve() / manifest["model"]
for name, expected in manifest["files"].items():
    destination = root / name
    if destination.is_file() and hashlib.sha256(destination.read_bytes()).hexdigest() == expected["sha256"]:
        continue
    destination.parent.mkdir(parents=True, exist_ok=True)
    url = f'https://huggingface.co/{manifest["model"]}/resolve/{manifest["revision"]}/{name}'
    # No cookies, authorization header, task text, or project data are sent.
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(dir=destination.parent, delete=False) as out:
            temporary = Path(out.name)
            with urllib.request.urlopen(url, timeout=120) as response:
                while chunk := response.read(1024 * 1024):
                    out.write(chunk)
        data = temporary.read_bytes()
        if len(data) != expected["bytes"] or hashlib.sha256(data).hexdigest() != expected["sha256"]:
            raise SystemExit(f"Model download integrity check failed: {name}")
        os.replace(temporary, destination)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)
print(f'Pinned public model ready: {manifest["model"]}@{manifest["revision"]}')
