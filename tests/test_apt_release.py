"""Ensure generated catalog indexes are covered by the signed Release payload."""
import hashlib
import gzip
import importlib.util
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location("apt_archive", Path(__file__).resolve().parents[1] / "scripts/create-apt-test-repo.py")
archive = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(archive)


class AptReleaseTests(unittest.TestCase):
    def test_compressed_indexes_also_publish_canonical_checksums(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "Release").write_text("Origin: asena\nSHA256:\n")
            target = root / "main/source/Sources"
            target.parent.mkdir(parents=True)
            data = b"Package: asena\nVersion: 0.1.0\n"
            target.with_suffix(".gz").write_bytes(gzip.compress(data))
            archive.expand_indexes(root)
            self.assertEqual(target.read_bytes(), data)
            content = archive.release_bytes(root).decode()
            self.assertIn(hashlib.sha256(data).hexdigest(), content)
            self.assertIn(f"{len(data):16d} main/source/Sources", content)

    def test_catalog_and_package_indexes_are_hashed_with_their_lengths(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "Release").write_text("Origin: asena\nValid-Until: future\nSHA256:\n old data\n")
            for name in ("main/dep11/Components-amd64.yml.gz", "main/binary-amd64/Packages.gz"):
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                data = name.encode()
                path.write_bytes(data)
            (root / "InRelease").write_text("old signature")
            content = archive.release_bytes(root).decode()
            self.assertTrue(content.startswith("Origin: asena\nValid-Until: future\n"))
            self.assertNotIn("old data", content)
            self.assertNotIn("InRelease", content)
            for name in ("main/dep11/Components-amd64.yml.gz", "main/binary-amd64/Packages.gz"):
                data = name.encode()
                self.assertIn(hashlib.sha256(data).hexdigest(), content)
                self.assertIn(f"{len(data):16d} {name}", content)


if __name__ == "__main__":
    unittest.main()
