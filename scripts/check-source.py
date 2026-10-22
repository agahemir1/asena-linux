"""Portable source/data checks; does not compile or execute Linux components."""

from pathlib import Path
import sys
import tomllib
import xml.etree.ElementTree as ET


def main():
    root = Path(__file__).resolve().parents[1]
    manifest = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
    expected = {
        "asena-core", "asena-services", "asena-wayland", "asena-widgets",
        "asena-shell", "asenactl", "asena-session", "asena-daemon", "asena-lock",
        "asena-greeter", "asena-polkit-agent", "asena-settings", "asena-welcome",
        "asena-screenshot", "asena-app", "asena-files", "asena-terminal",
        "asena-store", "asena-text", "asena-documents", "asena-photos",
        "asena-player", "asena-archive", "asena-calculator", "asena-calendar",
        "asena-notes", "asena-clock", "asena-camera", "asena-weather",
        "asena-packages",
    }
    actual = set()
    for path in sorted((root / "crates").glob("*/Cargo.toml")):
        crate = tomllib.loads(path.read_text(encoding="utf-8"))
        actual.add(crate["package"]["name"])
        assert crate["lints"]["workspace"] is True, path
        for field in ("version", "edition", "rust-version", "license"):
            assert crate["package"][field] == {"workspace": True}, (path, field)
        assert any((path.parent / "src" / f).is_file() for f in ("lib.rs", "main.rs")), path
    assert actual == expected, actual ^ expected
    assert manifest["workspace"]["package"]["rust-version"] == "1.85.1"
    deps = manifest["workspace"]["dependencies"]
    assert deps["gtk"]["features"] == ["v4_18"]
    assert deps["adw"]["features"] == ["v1_7"]
    rust_sources = sorted(p.relative_to(root).as_posix() for p in (root / "crates").rglob("*.rs"))
    listed = (root / "po/POTFILES").read_text(encoding="utf-8").splitlines()
    assert listed == rust_sources, "Update po/POTFILES with every Rust source"
    for path in (root / "data").rglob("*.xml"):
        ET.parse(path)
    resources = ET.parse(root / "data/asena.gresource.xml")
    for item in resources.findall(".//file"):
        source = root / "data" / item.text
        assert source.is_file() or (source.suffix == ".ui" and source.with_suffix(".blp").is_file()), item.text
    schema = ET.parse(root / "data/schemas/org.asenalinux.shell.gschema.xml")
    assert schema.find("schema").attrib["id"] == "org.asenalinux.shell"
    assert schema.find(".//key[@name='pinned-apps']").attrib["type"] == "as"
    for language in (root / "po/LINGUAS").read_text().splitlines():
        if language and not language.startswith("#"):
            assert (root / "po" / f"{language}.po").is_file()
    # Linux scripts must remain LF-only, including when edited on Windows.
    scripts = [*(root / "scripts").glob("*.sh"), root / "scripts/asena-session"]
    for path in scripts:
        assert path.read_bytes().startswith(b"#!/"), path
        assert b"\r" not in path.read_bytes(), path
    print(f"Source checks passed: {len(actual)} crates, {len(rust_sources)} Rust files, XML/resources/POTFILES/LF.")


if __name__ == "__main__":
    try:
        main()
    except (AssertionError, KeyError, ValueError, ET.ParseError) as error:
        print(f"Source check failed: {error}", file=sys.stderr)
        sys.exit(1)
