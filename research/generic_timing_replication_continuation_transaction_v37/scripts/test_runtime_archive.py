from __future__ import annotations

import gzip
import hashlib
import io
import json
import tarfile
import tempfile
import unittest
from pathlib import Path

import runtime_archive


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


class RuntimeArchiveTests(unittest.TestCase):
    def make_fixture(self, root: Path) -> tuple[Path, Path]:
        runtime = root / "runtime"
        (runtime / "p1").mkdir(parents=True)
        (runtime / "p2").mkdir(parents=True)
        (runtime / "p1" / "a.json").write_text('{"a":1}\n', encoding="utf-8")
        (runtime / "p2" / "b.json").write_text('{"b":2}\n', encoding="utf-8")
        verification = root / "verification.json"
        verification.write_text(
            json.dumps(
                {
                    "input_sha256": {
                        "runtime": {
                            "p1/a.json": sha256(runtime / "p1" / "a.json"),
                            "p2/b.json": sha256(runtime / "p2" / "b.json"),
                        }
                    }
                }
            ),
            encoding="utf-8",
        )
        return runtime, verification

    def test_pack_is_byte_deterministic_and_unpack_round_trips(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            runtime, verification = self.make_fixture(root)
            first = root / "first.tar.gz"
            second = root / "second.tar.gz"
            first_sha = root / "first.sha256"
            second_sha = root / "second.sha256"

            runtime_archive.pack_runtime(runtime, verification, first, first_sha)
            runtime_archive.pack_runtime(runtime, verification, second, second_sha)

            self.assertEqual(first.read_bytes(), second.read_bytes())
            self.assertEqual(first_sha.read_text(), second_sha.read_text())

            restored = root / "restored"
            runtime_archive.unpack_runtime(first, verification, restored)
            self.assertEqual(
                (restored / "p1" / "a.json").read_bytes(),
                (runtime / "p1" / "a.json").read_bytes(),
            )
            self.assertEqual(
                (restored / "p2" / "b.json").read_bytes(),
                (runtime / "p2" / "b.json").read_bytes(),
            )

    def test_pack_rejects_runtime_hash_mismatch(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            runtime, verification = self.make_fixture(root)
            (runtime / "p1" / "a.json").write_text('{"a":9}\n', encoding="utf-8")
            with self.assertRaisesRegex(runtime_archive.ArchiveError, "hash mismatch"):
                runtime_archive.pack_runtime(
                    runtime,
                    verification,
                    root / "bad.tar.gz",
                    root / "bad.sha256",
                )

    def test_unpack_rejects_unexpected_archive_member(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            _, verification = self.make_fixture(root)
            archive = root / "unexpected.tar.gz"
            raw = io.BytesIO()
            with tarfile.open(fileobj=raw, mode="w", format=tarfile.USTAR_FORMAT) as tar:
                info = tarfile.TarInfo("runtime/p1/a.json")
                payload = b'{"a":1}\n'
                info.size = len(payload)
                tar.addfile(info, io.BytesIO(payload))
                extra = tarfile.TarInfo("runtime/extra.json")
                payload = b"{}\n"
                extra.size = len(payload)
                tar.addfile(extra, io.BytesIO(payload))
            with archive.open("wb") as handle:
                with gzip.GzipFile(filename="", mode="wb", fileobj=handle, mtime=0) as zipped:
                    zipped.write(raw.getvalue())

            with self.assertRaisesRegex(runtime_archive.ArchiveError, "member set mismatch"):
                runtime_archive.unpack_runtime(archive, verification, root / "restored")


if __name__ == "__main__":
    unittest.main()
