"""Inspect actual Debian payloads, runtime paths, metadata and source checksums."""
import argparse
import csv
from email.parser import Parser
import hashlib
import io
from pathlib import Path
import subprocess
import tarfile
import tempfile
import xml.etree.ElementTree as ET


def output(*args):
    return subprocess.check_output(args)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("directory", type=Path)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    owners, apps, rows = {}, set(), []
    binaries = sorted(args.directory.glob("*.deb"))
    if not binaries:
        parser.error("No Debian packages were found")
    for archive in binaries:
        control = Parser().parsestr(output("dpkg-deb", "--field", str(archive)).decode())
        package = control["Package"]
        with archive.open("rb") as stream:
            digest = hashlib.file_digest(stream, "sha256").hexdigest()
        rows.append([package, control["Version"], control["Architecture"], digest, archive.name])
        with tarfile.open(fileobj=io.BytesIO(output("dpkg-deb", "--fsys-tarfile", str(archive)))) as payload:
            paths = {member.name.lstrip("./") for member in payload if member.isfile() or member.issym()}
            payload.members.clear()
        with tarfile.open(fileobj=io.BytesIO(output("dpkg-deb", "--fsys-tarfile", str(archive)))) as payload:
            for member in payload:
                if not (member.isfile() or member.issym()):
                    continue
                name = member.name.lstrip("./")
                if name in owners and owners[name] != package:
                    raise ValueError(f"Conflicting file ownership: {name}: {owners[name]} and {package}")
                owners[name] = package
                if member.uid != 0 or member.gid != 0 or (member.isfile() and member.mode & 0o6002):
                    raise ValueError(f"Unexpected ownership or permissions: {package}: {name}")
                if name in {"etc/greetd/config.toml", "etc/X11/default-display-manager"}:
                    raise ValueError("Installing the package would replace display manager configuration")
                if not member.isfile():
                    continue
                stream = payload.extractfile(member)
                if name.startswith("usr/share/metainfo/"):
                    component = ET.fromstring(stream.read())
                    app = component.findtext("id")
                    desktop = component.findtext("launchable")
                    if "usr/share/applications/" + desktop not in paths:
                        raise ValueError(f"Metainfo launchable is missing from {package}")
                    apps.add(app)
                    with tempfile.TemporaryDirectory() as temporary:
                        path = Path(temporary) / (app + ".metainfo.xml")
                        path.write_bytes(ET.tostring(component))
                        subprocess.run(["appstreamcli", "validate", "--no-net", str(path)], check=True)
                elif stream.read(4) == b"\x7fELF" and not name.startswith("usr/lib/debug/"):
                    stream.seek(0)
                    with tempfile.NamedTemporaryFile() as temporary:
                        temporary.write(stream.read())
                        temporary.flush()
                        rpath = output("patchelf", "--print-rpath", temporary.name).decode()
                        if "/build/" in rpath or "/home/" in rpath or "/target/" in rpath:
                            raise ValueError(f"Developer path in installed ELF: {package}: {rpath}")
                        needed = output("patchelf", "--print-needed", temporary.name).decode()
                        if "libgtk4-layer-shell.so" in needed and "/asena" not in rpath:
                            raise ValueError(f"Missing private library runtime path: {package}")
    for path in sorted(args.directory.glob("*.dsc")):
        subprocess.run(["dscverify", "--nosigcheck", str(path)], check=True)
    args.report.parent.mkdir(parents=True, exist_ok=True)
    with args.report.open("w", newline="") as stream:
        writer = csv.writer(stream, delimiter="\t")
        writer.writerow(["Package", "Version", "Architecture", "SHA256", "File"])
        writer.writerows(rows)
    print(f"Package checks passed: {len(binaries)} binaries, {len(apps)} application IDs, {len(owners)} uniquely owned paths.")


if __name__ == "__main__":
    main()
