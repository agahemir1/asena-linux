"""Exercise publication checks against disposable Git repositories."""

import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1] / "scripts/check-publication.py"
SPEC = importlib.util.spec_from_file_location("publication", SCRIPT)
publication = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(publication)


class PublicationTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.git("init", "-q")
        self.git("config", "user.name", "Publication test")
        self.git("config", "user.email", "test@example.invalid")
        self.write("README.md", "Test source\n")
        self.commit()

    def git(self, *args):
        return subprocess.run(
            ["git", "-C", str(self.root), *args], check=True,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        )

    def write(self, name, text):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")

    def commit(self):
        self.git("add", "-A")
        self.git("commit", "-qm", "test: save fixture")

    def findings(self):
        return publication.check(self.root)[0]

    def test_clean_source_and_public_archive_key_are_allowed(self):
        self.write("data/archive.asc", "-----BEGIN PGP PUBLIC KEY BLOCK-----\n")
        self.write("scripts/check-publication.py", SCRIPT.read_text(encoding="utf-8"))
        self.assertEqual(self.findings(), [])

    def test_ignored_local_settings_are_excluded(self):
        self.write(".gitignore", ".netlify/\n.env\n")
        self.write(".netlify/state.json", "local account")
        self.write(".env", "local setting")
        self.assertEqual(self.findings(), [])

    def test_tracked_local_settings_cannot_be_hidden_by_gitignore(self):
        self.write(".netlify/state.json", "local account")
        self.commit()
        self.write(".gitignore", ".netlify/\n")
        self.assertTrue(any(".netlify/state.json" in row for row in self.findings()))

    def test_deleted_secret_is_found_in_history_without_printing_it(self):
        secret = "ghp_" + "A" * 36
        self.write("old.txt", secret)
        self.commit()
        (self.root / "old.txt").unlink()
        self.commit()
        rows = self.findings()
        self.assertTrue(any(row.startswith("history:") and "old.txt" in row for row in rows))
        self.assertNotIn(secret, "\n".join(rows))

    def test_untracked_private_key_is_found(self):
        self.write("unexpected.txt", "-----BEGIN " + "OPENSSH PRIVATE KEY-----\n")
        self.assertTrue(any("unexpected.txt" in row for row in self.findings()))

    def test_private_pgp_key_is_found(self):
        self.write("archive.asc", "-----BEGIN PGP " + "PRIVATE KEY BLOCK-----\n")
        self.assertTrue(any("archive.asc" in row for row in self.findings()))

    def test_private_tool_snapshots_are_not_part_of_published_branches(self):
        clean = self.git("rev-parse", "HEAD").stdout.decode().strip()
        self.write(".claude/launch.json", "local preview settings")
        self.commit()
        snapshot = self.git("rev-parse", "HEAD").stdout.decode().strip()
        self.git("update-ref", "refs/codex/test-snapshot", snapshot)
        self.git("reset", "--hard", clean)
        self.assertEqual(self.findings(), [])

    def test_disk_images_are_rejected(self):
        self.write("release.iso", "image")
        self.assertTrue(any("disk or ISO image" in row for row in self.findings()))

    def test_oversized_files_are_found_in_worktree_and_history(self):
        self.write("large.bin", "x" * 256)
        self.commit()
        with patch.object(publication, "MAX_FILE_SIZE", 128):
            rows = self.findings()
        self.assertTrue(any(row.startswith("working tree: large.bin") for row in rows))
        self.assertTrue(any(row.startswith("history: large.bin") for row in rows))


if __name__ == "__main__":
    unittest.main()
