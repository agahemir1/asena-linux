"""Check the observable package inventory against the actual desktop entries."""
import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("debian_inventory", ROOT / "packaging/debian/inventory.py")
inventory = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(inventory)


class DebianInventoryTests(unittest.TestCase):
    def test_each_desktop_entry_has_an_independently_installable_package(self):
        apps = inventory.applications(ROOT)
        packages = inventory.packages(ROOT)
        for binary in apps:
            self.assertIn(binary, packages)
            self.assertTrue((ROOT / "crates" / binary / "src/main.rs").is_file())
            self.assertIn("asena-common", packages[binary][1])

    def test_desktop_includes_every_current_application(self):
        deps = inventory.packages(ROOT)["asena-desktop"][1].split(", ")
        self.assertTrue(set(inventory.applications(ROOT)).issubset(deps))

    def test_unimplemented_welcome_is_not_shipped_as_a_working_component(self):
        self.assertNotIn("asena-welcome", inventory.packages(ROOT))

    def test_greeter_installation_does_not_require_a_new_display_manager(self):
        deps = inventory.packages(ROOT)["asena-greeter"][1]
        self.assertNotIn("greetd", deps)
        self.assertIn("Suggests: greetd, cage", inventory.control(ROOT))

    def test_generated_control_matches_the_inventory(self):
        self.assertEqual((ROOT / "packaging/debian/control").read_text(encoding="utf-8"), inventory.control(ROOT))


if __name__ == "__main__":
    unittest.main()
