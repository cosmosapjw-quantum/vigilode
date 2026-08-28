#!/usr/bin/env python3
from __future__ import annotations
import base64, hashlib, pathlib, sys, zlib
PAYLOAD_SHA256 = "351e58f0ce7f4dcb18596c019e2b1b86fa034c404c1801bac6d9b9226f62cbc0"
path = pathlib.Path(__file__).with_name("verify-k0-wu05-supplement.payload.b64")
raw = path.read_bytes()
if hashlib.sha256(raw).hexdigest() != PAYLOAD_SHA256:
    raise SystemExit("STOP_INVALID: supplement payload identity mismatch")
source = zlib.decompress(base64.b64decode(raw)).decode("utf-8")
if "--dump-source" in sys.argv:
    sys.stdout.write(source); raise SystemExit(0)
exec(compile(source, str(path), "exec"), {"__name__":"__main__","__file__":str(path)})
