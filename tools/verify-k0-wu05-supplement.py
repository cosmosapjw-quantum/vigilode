#!/usr/bin/env python3
from __future__ import annotations
import base64, pathlib, sys, zlib
path = pathlib.Path(__file__).with_name("verify-k0-wu05-supplement.payload.b64")
source = zlib.decompress(base64.b64decode(path.read_text(encoding="ascii"))).decode("utf-8")
if "--dump-source" in sys.argv:
    sys.stdout.write(source)
    raise SystemExit(0)
code = compile(source, str(path), "exec")
namespace = {"__name__": "__main__", "__file__": str(path)}
exec(code, namespace)
