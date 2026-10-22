"""Accent icon themes stay in sync with the palette and the generator."""

import configparser
import importlib.util
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]
ICONS = ROOT / "data/icons"


def generator():
    spec = importlib.util.spec_from_file_location("asena_icons", ROOT / "scripts/icons/apps.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def core_accents():
    text = (ROOT / "crates/asena-core/src/theme.rs").read_text(encoding="utf-8")
    return {name: f"#{rgb}" for name, rgb in re.findall(r'\("(\w+)", 0x([0-9a-f]{6})\)', text)}


class IconThemeTests(unittest.TestCase):
    def setUp(self):
        self.icons = generator()

    def test_generator_palette_matches_asena_core(self):
        self.assertEqual(self.icons.ACCENTS, core_accents())

    def test_teal_keeps_the_reviewed_drawings(self):
        for name, draw in self.icons.ACCENT_ICONS.items():
            drawn = (ICONS / "hicolor/scalable/apps" / f"{name}.svg").read_text(encoding="utf-8")
            self.assertEqual(draw(), drawn, name)
            self.assertEqual(draw("teal"), drawn, name)

    def test_every_other_accent_has_a_complete_theme(self):
        places = {f"{name}.svg" for name in self.icons.PLACES}
        apps = {f"{name}.svg" for name in self.icons.ACCENT_ICONS}
        self.assertEqual({path.name for path in (ICONS / "Asena/scalable/places").iterdir()}, places)
        for accent in self.icons.ACCENTS:
            theme = ICONS / f"Asena-{accent}"
            if accent == self.icons.BASE_ACCENT:
                self.assertFalse(theme.exists())
                continue
            index = configparser.ConfigParser()
            index.read(theme / "index.theme", encoding="utf-8")
            self.assertEqual(index["Icon Theme"]["Inherits"].split(",")[0], "Asena")
            self.assertEqual({path.name for path in (theme / "scalable/places").iterdir()}, places)
            self.assertEqual({path.name for path in (theme / "scalable/apps").iterdir()}, apps)
            # The committed files are what the generator draws.
            for name, emblem in self.icons.PLACES.items():
                drawn = (theme / "scalable/places" / f"{name}.svg").read_text(encoding="utf-8")
                self.assertEqual(self.icons.folder(accent, emblem), drawn, (accent, name))

    def test_tint_moves_the_main_tone_to_the_accent(self):
        for accent, color in self.icons.ACCENTS.items():
            if accent == self.icons.BASE_ACCENT:
                continue
            hue, _, _ = self.icons.hls(self.icons.tint(self.icons.DESIGN_TONE, accent))
            accent_hue, _, _ = self.icons.hls(color)
            distance = min(abs(hue - accent_hue), 1 - abs(hue - accent_hue))
            self.assertLess(distance, 0.01, accent)

    def test_every_application_has_its_own_drawn_icon(self):
        apps = ICONS / "hicolor/scalable/apps"
        for entry in sorted((ROOT / "data/applications").glob("org.asenalinux.*.desktop")):
            parser = configparser.ConfigParser(interpolation=None)
            parser.read(entry, encoding="utf-8")
            icon = parser["Desktop Entry"]["Icon"]
            self.assertEqual(icon, entry.stem, entry.name)
            self.assertIn(icon, self.icons.ICONS, entry.name)
            drawn = (apps / f"{icon}.svg").read_text(encoding="utf-8")
            self.assertEqual(self.icons.ICONS[icon](), drawn, entry.name)
            self.assertTrue((ICONS / "Asena/scalable/apps" / f"{icon}.svg").exists(), entry.name)

    def test_base_theme_lists_the_places_directory(self):
        index = configparser.ConfigParser()
        index.read(ICONS / "Asena/index.theme", encoding="utf-8")
        self.assertIn("scalable/places", index["Icon Theme"]["Directories"].split(","))
        self.assertIn("scalable/places", index)


if __name__ == "__main__":
    unittest.main()
