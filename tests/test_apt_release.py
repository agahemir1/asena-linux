"""Ensure generated catalog indexes are covered by the signed Release payload."""
import hashlib
import importlib.util
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location("apt_archive", Path(__file__).resolve().parents[1] / "scripts/create-apt-test-repo.py")
archive = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(archive)


class AptReleaseTests(unittest.TestCase):
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
