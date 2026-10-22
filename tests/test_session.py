"""Linux-only session contract tests; never start the user's compositor or services."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
SYSTEM_CONFIG = "/usr/share/asena/wayfire/wayfire.ini"


@unittest.skipUnless(sys.platform == "linux", "Session tests require Linux")
class SessionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="asena session ")
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.bin = self.base / "bin"
        self.bin.mkdir()
        self.result = self.base / "result.json"
        self.stopped = self.base / "stopped"
        self.env = dict(os.environ, PATH=f"{self.bin}:/usr/bin:/bin",
                        HOME=str(self.base / "home"), RESULT=str(self.result),
                        STOPPED=str(self.stopped))
        for key in ("ASENA_DEV", "ASENA_DAEMON", "ASENA_SOURCE_DIR", "ASENA_DEV_CONFIG",
                    "ASENA_DEV_LOG", "XDG_CONFIG_HOME", "XDG_DATA_HOME", "XDG_DATA_DIRS"):
            self.env.pop(key, None)
        self.program("wayfire", """
import json, os, sys
keys = ['ASENA_DEV', 'ASENA_SHELL', 'ASENA_AUTOSTART', 'GSETTINGS_SCHEMA_DIR',
        'ASENA_LOCALE_DIR', 'XDG_CURRENT_DESKTOP', 'XDG_SESSION_TYPE', 'GDK_BACKEND',
        'ASENA_WAYFIRE_CONFIG', 'XDG_DATA_DIRS', 'ASENA_DEV_LOG']
with open(os.environ['RESULT'], 'w') as output:
    json.dump({'args': sys.argv[1:], 'env': {key: os.getenv(key) for key in keys}}, output)
if os.getenv('ASENA_DEV_LOG'):
    with open(os.environ['ASENA_DEV_LOG'], 'a') as log:
        print('compositor child output', file=log)
""")
        self.program("systemctl", """
import os, sys
open(os.environ['STOPPED'], 'w').write(' '.join(sys.argv[1:]))
""")

    def program(self, name, body):
        path = self.bin / name
        path.write_text(f"#!{sys.executable}\n{body}", encoding="utf-8")
        path.chmod(0o755)

    def run_session(self, *args):
        return subprocess.run(["sh", str(ROOT / "scripts/asena-session"), *args],
                              env=self.env, capture_output=True, text=True, check=False)

    def capture(self):
        return json.loads(self.result.read_text())

    def test_development_uses_disposable_config_with_spaces(self):
        self.env["ASENA_SOURCE_DIR"] = str(self.base)
        config = self.base / "dev config/wayfire.ini"
        self.env["ASENA_DEV_CONFIG"] = str(config)
        result = self.run_session("--dev")
        self.assertEqual(result.returncode, 0, result.stderr)
        capture = self.capture()
        self.assertEqual(capture["args"], ["-c", str(config)])
        self.assertEqual(capture["env"]["ASENA_SHELL"], str(self.base / "target/debug/asena-shell"))
        self.assertEqual(capture["env"]["ASENA_WAYFIRE_CONFIG"], str(config))
        self.assertEqual(capture["env"]["XDG_CURRENT_DESKTOP"], "Asena")
        self.assertEqual(capture["env"]["GDK_BACKEND"], "wayland")
        # Wayfire discards its children's output; the log path still reaches
        # the session's stderr because the script becomes the compositor.
        self.assertIn("compositor child output", result.stderr)
        # The nested session never stops the real user's services.
        self.assertFalse(self.stopped.exists())

    def test_development_keeps_the_flatpak_prefix_known_to_the_private_bus(self):
        # dev-session.sh adds the Flatpak exports before starting the bus.
        flatpak = f"{self.base}/home/.local/share/flatpak/exports/share:/var/lib/flatpak/exports/share"
        self.env.update(ASENA_SOURCE_DIR=str(self.base),
                        ASENA_DEV_CONFIG=str(self.base / "wayfire.ini"),
                        XDG_DATA_DIRS=f"{flatpak}:/usr/share")
        result = self.run_session("--dev")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.capture()["env"]["XDG_DATA_DIRS"],
                         f"{self.base}/target/share:{flatpak}:/usr/share")

    def test_development_without_prepared_config_gives_actionable_error(self):
        self.env["ASENA_SOURCE_DIR"] = str(self.base)
        result = self.run_session("--dev")
        self.assertEqual(result.returncode, 2)
        self.assertIn("scripts/dev-session.sh", result.stderr)
        self.assertFalse(self.result.exists())

    def test_invalid_arguments_do_not_start_compositor(self):
        for args in [("--unknown",), ("--dev", "extra")]:
            self.env["ASENA_SOURCE_DIR"] = str(self.base)
            self.env["ASENA_DEV_CONFIG"] = str(self.base / "wayfire.ini")
            result = self.run_session(*args)
            self.assertEqual(result.returncode, 2)
            self.assertFalse(self.result.exists())

    def test_normal_session_uses_user_config_or_system_default(self):
        # Without an installed default the copy fails and the system path is used.
        result = self.run_session()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.capture()["args"], ["-c", SYSTEM_CONFIG])
        config = self.base / "home/.config/asena/wayfire.ini"
        config.parent.mkdir(parents=True, exist_ok=True)
        config.write_text("[core]\n")
        self.assertEqual(self.run_session().returncode, 0)
        capture = self.capture()
        self.assertEqual(capture["args"], ["-c", str(config)])
        self.assertEqual(capture["env"]["ASENA_WAYFIRE_CONFIG"], str(config))
        self.assertIn("flatpak/exports/share", capture["env"]["XDG_DATA_DIRS"])
        self.assertIsNone(capture["env"]["ASENA_DEV_LOG"])

    def test_symlinked_user_config_is_not_followed(self):
        target = self.base / "elsewhere"
        target.mkdir()
        config_home = self.base / "home/.config"
        config_home.mkdir(parents=True)
        (config_home / "asena").symlink_to(target, target_is_directory=True)
        self.assertEqual(self.run_session().returncode, 0)
        self.assertEqual(self.capture()["args"], ["-c", SYSTEM_CONFIG])
        self.assertEqual(list(target.iterdir()), [])

    def test_normal_session_stops_user_services_when_compositor_exits(self):
        self.program("wayfire", "raise SystemExit(3)\n")
        result = self.run_session()
        self.assertEqual(result.returncode, 3)
        self.assertEqual(self.stopped.read_text(), "--user stop asena-session.target")

    def test_nested_autostart_does_not_touch_user_manager(self):
        activation = self.base / "activation.json"
        self.program("systemctl", "raise SystemExit('Must not touch user manager')\n")
        # The private nested bus must learn the nested display, but never through systemd.
        self.program("dbus-update-activation-environment", f"""
import json, sys
if '--systemd' in sys.argv[1:]:
    raise SystemExit('Must not import environment into the user manager')
open({str(activation)!r}, 'w').write(json.dumps(sys.argv[1:]))
""")
        self.program("shell-probe", "import os\nopen(os.environ['RESULT'], 'w').write('shell-started')\n")
        self.env.pop("DISPLAY", None)
        self.env.update(ASENA_DEV="1", ASENA_SHELL=str(self.bin / "shell-probe"),
                        WAYLAND_DISPLAY="wayland-7", XDG_CURRENT_DESKTOP="Asena")
        result = subprocess.run(["sh", str(ROOT / "scripts/session-autostart.sh")],
                                env=self.env, capture_output=True, text=True, check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.result.read_text(), "shell-started")
        names = json.loads(activation.read_text())
        self.assertIn("WAYLAND_DISPLAY", names)
        self.assertIn("XDG_CURRENT_DESKTOP", names)
        self.assertNotIn("DISPLAY", names)

    def test_nested_autostart_writes_to_the_session_log(self):
        self.program("dbus-update-activation-environment", "")
        self.program("shell-probe", """
import os, sys
print('shell diagnostics', file=sys.stderr)
open(os.environ['RESULT'], 'w').write('shell-started')
""")
        log = self.base / "session log.txt"
        log.write_text("compositor started\n")
        self.env.update(ASENA_DEV="1", ASENA_SHELL=str(self.bin / "shell-probe"),
                        ASENA_DEV_LOG=str(log))
        script = ["sh", str(ROOT / "scripts/session-autostart.sh")]
        result = subprocess.run(script, env=self.env, capture_output=True, text=True, check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(log.read_text(), "compositor started\nshell diagnostics\n")
        # A log that cannot be opened must not stop the session.
        self.result.unlink()
        self.env["ASENA_DEV_LOG"] = str(self.base / "missing/log.txt")
        result = subprocess.run(script, env=self.env, capture_output=True, text=True, check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.result.read_text(), "shell-started")

    def test_compositor_autostart_runs_the_session_script(self):
        text = (ROOT / "data/wayfire/wayfire.ini").read_text(encoding="utf-8")
        self.assertIn('asena = exec sh "$ASENA_AUTOSTART"', text)
        self.assertIn("autostart_wf_shell = false", text)


if __name__ == "__main__":
    unittest.main()
