"""Single inventory for Debian package ownership and dependency generation."""
import configparser
from pathlib import Path

BASE = "asena-common (= ${source:Version}), asena-artwork (= ${source:Version})"
EXTRA = {
    "asena-archive": "libarchive-tools",
    "asena-backup": "rsync, systemd, procps",
    "asena-camera": "gstreamer1.0-gtk4, gstreamer1.0-pipewire, gstreamer1.0-plugins-good",
    "asena-disks": "udisks2, dosfstools, exfatprogs, ntfs-3g",
    "asena-documents": "poppler-utils",
    "asena-files": "gvfs, gvfs-backends",
    "asena-player": "libgtk-4-media-gstreamer, gstreamer1.0-plugins-good",
    "asena-settings": "asena-daemon (= ${binary:Version}), cups-pk-helper, cups-client, accountsservice",
    "asena-store": "asena-packages (= ${binary:Version}), appstream, flatpak, gnome-keyring",
}


def applications(root):
    apps = {}
    for path in sorted((root / "data/applications").glob("*.desktop")):
        parser = configparser.ConfigParser(interpolation=None)
        parser.optionxform = str
        parser.read(path, encoding="utf-8")
        entry = dict(parser["Desktop Entry"])
        binary = entry["Exec"].split()[0]
        if binary in apps:
            raise ValueError(f"Duplicate application binary: {binary}")
        apps[binary] = {**entry, "id": path.stem, "desktop": path.name}
    return apps


def packages(root):
    result = {
        "asena-common": ("all", "gsettings-desktop-schemas", "Shared Asena settings and translations"),
        "asena-artwork": ("all", "adwaita-icon-theme, librsvg2-common", "Asena icons and wallpapers"),
        "asena-layer-shell": ("any", "", "Private approved GTK layer shell 1.3.0 library"),
        "asena-shell": ("any", BASE + ", asenactl, asena-lock, grim, slurp", "Asena desktop shell"),
        "asena-daemon": ("any", BASE + ", asena-lock, dbus, systemd, wlsunset, swayidle, procps", "Asena desktop services"),
        "asena-lock": ("any", BASE, "Asena session locker"),
        "asena-greeter": ("any", BASE, "Asena greetd login screen"),
        "asena-polkit-agent": ("any", BASE + ", polkitd", "Asena authorization agent"),
        "asena-session": ("any", "asena-shell, asena-daemon, asena-polkit-agent, wayfire (>= 0.9), libwlroots-0.18 (>= 0.18.2-3+asena1), xwayland, dbus-user-session, systemd", "Asena Wayland session"),
        "asenactl": ("any", "asena-common (= ${source:Version})", "Command line control for Asena"),
        "asena-packages": ("any", "asena-common (= ${source:Version}), apt, dpkg, dbus, polkitd, systemd", "Authorized Asena package service"),
        "asena-recovery": ("any", BASE + ", python3, rsync, squashfs-tools, util-linux, grub2-common, dbus, polkitd", "Asena recovery service and offline worker"),
    }
    for binary, app in applications(root).items():
        deps = BASE + (", " + EXTRA[binary] if binary in EXTRA else "")
        result[binary] = ("any", deps, app["Comment"])
    desktop_deps = ", ".join(name for name in result if name not in {"asena-common", "asena-artwork", "asena-layer-shell"})
    desktop_deps += ", network-manager, bluez, upower, pipewire, pipewire-pulse, wireplumber, xdg-desktop-portal, xdg-desktop-portal-wlr, xdg-desktop-portal-gtk, fonts-inter, fonts-jetbrains-mono"
    result["asena-desktop"] = ("all", desktop_deps, "Asena desktop and default applications")
    return result


def control(root):
    build = "debhelper-compat (= 13), build-essential, cargo, rustc (>= 1.85.1), pkg-config, meson, ninja-build, patchelf, python3, blueprint-compiler, gettext, libglib2.0-dev-bin, libgtk-4-dev (>= 4.18), libadwaita-1-dev (>= 1.7), libpulse-dev, libpam0g-dev, libpolkit-agent-1-dev, libpolkit-gobject-1-dev, libapt-pkg-dev, libflatpak-dev, libappstream-dev, libsoup-3.0-dev, libvte-2.91-gtk4-dev, libgstreamer1.0-dev, libgstreamer-plugins-base1.0-dev, libwayland-dev, wayland-protocols, libxml2-utils, desktop-file-utils, appstream, librsvg2-bin, locales, libc-l10n"
    build += ", git, patch"
    text = f"Source: asena\nSection: x11\nPriority: optional\nMaintainer: agahemir1 <projectxurme@gmail.com>\nBuild-Depends: {build}\nStandards-Version: 4.7.2\nRules-Requires-Root: no\nHomepage: https://asenalinux.netlify.app\nVcs-Git: https://github.com/agahemir1/asena-linux.git\nVcs-Browser: https://github.com/agahemir1/asena-linux\n"
    for name, (arch, deps, description) in packages(root).items():
        depends = (["${shlibs:Depends}"] if arch == "any" else []) + ["${misc:Depends}"]
        if deps:
            depends.append(deps)
        text += f"\nPackage: {name}\nArchitecture: {arch}\nDepends: {', '.join(depends)}\n"
        if name == "asena-greeter":
            text += "Suggests: greetd, cage\n"
        text += f"Description: {description}\n Part of the Asena desktop for Debian 13.\n"
    return text


if __name__ == "__main__":
    import sys
    root = Path(__file__).resolve().parents[2]
    expected = control(root)
    path = Path(__file__).with_name("control")
    if "--check" in sys.argv:
        if path.read_text(encoding="utf-8") != expected:
            raise SystemExit("Regenerate packaging/debian/control with inventory.py")
    else:
        path.write_text(expected, encoding="utf-8", newline="\n")
