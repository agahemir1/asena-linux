"""Branding assets stay what scripts/icons/brand.py draws (K07, K23)."""

import importlib.util
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]


def generator():
    spec = importlib.util.spec_from_file_location("asena_brand", ROOT / "scripts/icons/brand.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def os_release():
    values = {}
    for line in (ROOT / "data/branding/os-release").read_text(encoding="utf-8").splitlines():
        if line and not line.startswith("#"):
            key, _, value = line.partition("=")
            values[key] = value.strip('"')
    return values


class BrandingTests(unittest.TestCase):
    def setUp(self):
        self.brand = generator()

    def test_committed_assets_match_the_generator(self):
        for relative, draw in self.brand.OUTPUTS.items():
            committed = (ROOT / relative).read_text(encoding="utf-8")
            self.assertEqual(draw(), committed, relative)

    def test_weave_rejects_bands_that_hide_a_corner(self):
        with self.assertRaises(ValueError):
            self.brand.weave("#fff", "#000", 25.5, 8.0, "test")

    def test_ascii_logo_uses_two_colors_and_fits_a_terminal(self):
        text = self.brand.ascii_logo()
        self.assertEqual(set(re.findall(r"\$\d", text)), {"$1", "$2"})
        plain = re.sub(r"\$\d", "", text).splitlines()
        self.assertLessEqual(max(len(line) for line in plain), 42)
        self.assertLessEqual(len(plain), 21)

    def test_os_release_identifies_asena_on_debian(self):
        values = os_release()
        self.assertEqual(values["ID"], "asena")
        self.assertEqual(values["ID_LIKE"], "debian")
        self.assertEqual(values["VERSION_CODENAME"], "trixie")
        self.assertEqual(values["LOGO"], "asena-logo")
        self.assertTrue(values["PRETTY_NAME"].startswith("Asena Linux"))
        self.assertTrue((ROOT / "data/icons/hicolor/scalable/apps/asena-logo.svg").exists())


if __name__ == "__main__":
    unittest.main()
