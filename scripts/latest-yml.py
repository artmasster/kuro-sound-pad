"""Writes the update feed the app reads (latest.yml, in electron-builder's format) for a release's installer.

    python -I scripts/latest-yml.py <version> <out.yml> <installer>

The feed names the installer and gives its size and base64 SHA-512, which the app checks after downloading.
Run it on the final, signed file.
"""
import base64
import datetime
import hashlib
import sys
from pathlib import Path

if len(sys.argv) != 4:
    sys.exit(__doc__)
version, out, installer = sys.argv[1], Path(sys.argv[2]), Path(sys.argv[3])

digest = hashlib.sha512()
with installer.open("rb") as f:
    for chunk in iter(lambda: f.read(1 << 20), b""):
        digest.update(chunk)
sha512 = base64.b64encode(digest.digest()).decode()
now = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.") + "000Z"
lines = [
    f"version: {version}",
    "files:",
    f"  - url: {installer.name}",
    f"    sha512: {sha512}",
    f"    size: {installer.stat().st_size}",
    f"path: {installer.name}",
    f"sha512: {sha512}",
    f"releaseDate: '{now}'",
    "",
]
out.write_text("\n".join(lines), encoding="utf-8", newline="\n")
print(out.read_text(encoding="utf-8"))
