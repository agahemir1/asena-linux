"""Install package-owned payloads without touching the build host's system."""
import os
from pathlib import Path
import shutil
import subprocess
import xml.etree.ElementTree as ET

from inventory import applications, packages

ROOT = Path.cwd()
TRIPLET = subprocess.check_output(["dpkg-architecture", "-qDEB_HOST_MULTIARCH"], text=True).strip()
OWNERS = {}


def put(package, source, destination, mode=0o644):
    destination = destination.lstrip("/")
    if destination in OWNERS:
        raise ValueError(f"Duplicate ownership of {destination}: {OWNERS[destination]} and {package}")
    OWNERS[destination] = package
    target = ROOT / "debian" / package / destination
    target.parent.mkdir(parents=True, exist_ok=True)
    source = ROOT / source
    if source.is_symlink():
        target.symlink_to(os.readlink(source))
    else:
        shutil.copyfile(source, target)
        target.chmod(mode)


def tree(package, source, destination):
    for path in sorted((ROOT / source).rglob("*")):
        if path.is_file() or path.is_symlink():
            put(package, path, str(Path(destination) / path.relative_to(ROOT / source)))


def metadata(package, app):
    component = ET.Element("component", type="desktop-application")
    fields = {"id": app["id"], "metadata_license": "CC0-1.0", "project_license": "GPL-3.0-or-later",
              "name": app["Name"], "summary": app["Comment"]}
    for tag, value in fields.items():
        ET.SubElement(component, tag).text = value
    developer = ET.SubElement(component, "developer", id="org.asenalinux")
    ET.SubElement(developer, "name").text = "Asena Linux contributors"
    for tag, key in (("name", "Name[tr]"), ("summary", "Comment[tr]")):
        if key in app:
            ET.SubElement(component, tag, {"{http://www.w3.org/XML/1998/namespace}lang": "tr"}).text = app[key]
    ET.SubElement(component, "launchable", type="desktop-id").text = app["desktop"]
    ET.SubElement(component, "url", type="homepage").text = "https://asenalinux.netlify.app"
    description = ET.SubElement(component, "description")
    ET.SubElement(description, "p").text = app["Comment"] + "."
    if "Comment[tr]" in app:
        ET.SubElement(description, "p", {"{http://www.w3.org/XML/1998/namespace}lang": "tr"}).text = app["Comment[tr]"] + "."
    categories = ET.SubElement(component, "categories")
    for category in app["Categories"].split(";"):
        if category and category != "GTK":
            ET.SubElement(categories, "category").text = category
    ET.SubElement(component, "content_rating", type="oars-1.1")
    target = ROOT / "debian" / package / "usr/share/metainfo" / (app["id"] + ".metainfo.xml")
    target.parent.mkdir(parents=True, exist_ok=True)
    ET.indent(component)
    ET.ElementTree(component).write(target, encoding="utf-8", xml_declaration=True)


def main():
    apps = applications(ROOT)
    for name, (arch, _, _) in packages(ROOT).items():
        (ROOT / "debian" / name).mkdir(parents=True, exist_ok=True)
        if arch == "any" and name != "asena-layer-shell":
            directory = "usr/libexec" if name == "asena-packages" else "usr/bin"
            put(name, f"target/release/{name}", f"{directory}/{name}", 0o755)
            binary = ROOT / "debian" / name / directory / name
            needed = subprocess.check_output(["patchelf", "--print-needed", str(binary)], text=True)
            if "libgtk4-layer-shell.so" in needed:
                subprocess.run(["patchelf", "--set-rpath", f"/usr/lib/{TRIPLET}/asena", str(binary)], check=True)
    native = ROOT / "debian/native/lib"
    for path in native.glob("libgtk4-layer-shell.so.*"):
        put("asena-layer-shell", path, f"usr/lib/{TRIPLET}/asena/{path.name}", 0o644)
    put("asena-layer-shell", "third-party/gtk4-layer-shell/LICENSE", "usr/share/doc/asena-layer-shell/LICENSE.gtk4-layer-shell")
    for path in (ROOT / "data/schemas").glob("*.xml"):
        put("asena-common", path, f"usr/share/glib-2.0/schemas/{path.name}")
    tree("asena-common", "target/locale", "usr/share/locale")
    put("asena-common", "source-info.json", "usr/share/asena/source-info.json")
    tree("asena-common", "third-party-licenses", "usr/share/doc/asena-common/third-party-licenses")
    for path in (ROOT / "data/icons").iterdir():
        if path.is_dir():
            tree("asena-artwork", path.relative_to(ROOT), f"usr/share/icons/{path.name}")
    tree("asena-artwork", "data/wallpapers", "usr/share/backgrounds/asena")
    put("asena-artwork", "third-party-licenses/CC-BY-SA-4.0.txt", "usr/share/doc/asena-artwork/CC-BY-SA-4.0")
    for package, app in apps.items():
        put(package, "data/applications/" + app["desktop"], "usr/share/applications/" + app["desktop"])
        metadata(package, app)
    put("asena-session", "data/sessions/asena.desktop", "usr/share/wayland-sessions/asena.desktop")
    put("asena-session", "data/wayfire/wayfire.ini", "usr/share/asena/wayfire/wayfire.ini")
    put("asena-session", "data/gsettings/90_asena.gschema.override", "usr/share/glib-2.0/schemas/90_asena.gschema.override")
    put("asena-session", "scripts/session-autostart.sh", "usr/libexec/asena/session-autostart.sh", 0o755)
    for path in (ROOT / "data/systemd").glob("*"):
        if not path.is_file():
            continue
        package = "asena-session" if path.name == "asena-session.target" else path.name.rsplit(".", 1)[0]
        put(package, path, f"usr/lib/systemd/user/{path.name}")
    for path in (ROOT / "data/systemd/system").glob("*"):
        package = "asena-packages" if path.name == "asena-offline-update.service" else path.stem
        put(package, path, f"usr/lib/systemd/system/{path.name}")
    timer = ROOT / "debian/asena-backup/usr/lib/systemd/user/timers.target.wants/asena-backup.timer"
    timer.parent.mkdir(parents=True, exist_ok=True)
    timer.symlink_to("../asena-backup.timer")
    for package, service in (("asena-settings", "org.asenalinux.Settings"), ("asena-daemon", "org.freedesktop.impl.portal.desktop.asena")):
        put(package, f"data/dbus-1/services/{service}.service", f"usr/share/dbus-1/services/{service}.service")
    for package, service in (("asena-packages", "Packages"), ("asena-recovery", "Recovery")):
        name = f"org.asenalinux.{service}"
        put(package, f"data/dbus-1/system-services/{name}.service", f"usr/share/dbus-1/system-services/{name}.service")
        put(package, f"data/dbus-1/system.d/{name}.conf", f"usr/share/dbus-1/system.d/{name}.conf")
        policy = f"org.asenalinux.{service.lower()}.policy"
        put(package, f"data/polkit/{policy}", f"usr/share/polkit-1/actions/{policy}")
    put("asena-daemon", "data/portals/asena.portal", "usr/share/xdg-desktop-portal/portals/asena.portal")
    put("asena-daemon", "data/portals/asena-portals.conf", "usr/share/xdg-desktop-portal/asena-portals.conf")
    put("asena-lock", "data/pam.d/asena-lock", "etc/pam.d/asena-lock")
    put("asena-greeter", "data/greetd/config.toml.example", "usr/share/doc/asena-greeter/examples/config.toml")
    put("asena-recovery", "data/recovery/recovery.py", "usr/lib/asena/recovery.py")
    put("asena-recovery", "scripts/install-recovery.sh", "usr/lib/asena/install-recovery", 0o755)
    put("asena-recovery", "data/grub/grub.d/43_asena_recovery", "etc/grub.d/43_asena_recovery", 0o755)
    put("asena-store", "data/apt/apt.conf.d/50asena-appstream-icons", "etc/apt/apt.conf.d/50asena-appstream-icons")
    print(f"Installed {len(OWNERS)} uniquely owned files in {len(packages(ROOT))} package trees.")


if __name__ == "__main__":
    main()
