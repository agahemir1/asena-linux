"""Prepare complete offline Debian source, including locked Cargo and native sources."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tarfile
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
NATIVE_SHA = "1ebb01ab14e98afd1727f68f64981c37bd23305b1f131f5667c02b94cf593192"


def run(*args, cwd=ROOT, stdout=None):
    return subprocess.run(args, cwd=cwd, check=True, stdout=stdout)


def ignored(directory, names):
    excluded = {".git", "target", ".netlify", ".claude", ".ssh", ".gnupg",
                "node_modules", ".astro", "__pycache__", "id_rsa", "id_ed25519", "id_ecdsa", "credentials.json"}
    if Path(directory) == ROOT / "website":
        excluded.add("dist")
    return {name for name in names if name in excluded or name == ".env" or
            name.startswith(".env.") and name != ".env.example" or
            Path(name).suffix in {".key", ".p12", ".pfx", ".qcow2", ".iso", ".vdi", ".vmdk"}}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--version", required=True)
    parser.add_argument("--source-ref", required=True)
    parser.add_argument("--resume", action="store_true", help="Resume an incomplete preparation directory")
    args = parser.parse_args()
    if not all(c.isalnum() or c in ".~+" for c in args.version):
        parser.error("Use a Debian native version without a revision")
    work = ROOT / "target/packages/asena"
    destination = work / f"asena-{args.version}"
    if destination.exists() and (not args.resume or (destination / "source-info.json").exists()):
        parser.error(f"Source destination exists: {destination}")
    work.mkdir(parents=True, exist_ok=True)
    shutil.copytree(ROOT, destination, ignore=ignored, symlinks=True, dirs_exist_ok=args.resume)
    shutil.copytree(destination / "packaging/debian", destination / "debian", dirs_exist_ok=args.resume)
    changelog = destination / "debian/changelog"
    original = changelog.read_text()
    original_version = original.split("(", 1)[1].split(")", 1)[0]
    changelog.write_text(original.replace(f"({original_version})", f"({args.version})", 1))
    run("python3", "packaging/debian/inventory.py", "--check", cwd=destination)
    with (destination / ".cargo/config.toml").open("a") as config:
        vendor = subprocess.check_output(
            ["cargo", "vendor", "--locked", "--versioned-dirs", "vendor/cargo"], cwd=destination,
        ).decode()
        config.write("\n" + vendor)
    native = ROOT / "target/backports/gtk4-layer-shell-1.3.0.tar.gz"
    if not native.exists():
        raise SystemExit("Run scripts/backport-layer-shell.sh to fetch the checked native source first")
    if hashlib.sha256(native.read_bytes()).hexdigest() != NATIVE_SHA:
        raise SystemExit("Native source checksum mismatch")
    unpacked = destination / "third-party"
    unpacked.mkdir(exist_ok=True)
    if not (unpacked / "gtk4-layer-shell").exists():
        with tarfile.open(native) as archive:
            archive.extractall(unpacked, filter="data")
        (unpacked / "gtk4-layer-shell-1.3.0").rename(unpacked / "gtk4-layer-shell")
    for patch in sorted((destination / "packaging/native").glob("*.patch")):
        with patch.open("rb") as stream:
            applied = subprocess.run(["patch", "-p1", "--reverse", "--dry-run"], cwd=unpacked / "gtk4-layer-shell",
                                     stdin=stream, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            if applied.returncode:
                stream.seek(0)
                subprocess.run(["patch", "-p1", "--forward"], cwd=unpacked / "gtk4-layer-shell",
                               stdin=stream, check=True)
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--locked", "--offline", "--format-version=1"], cwd=destination,
    ))
    notices = destination / "third-party-licenses"
    notices.mkdir(exist_ok=True)
    inventory = []
    for package in metadata["packages"]:
        manifest = Path(package["manifest_path"])
        if package["source"] is None:
            continue
        entry = {key: package.get(key) for key in ("name", "version", "license", "repository")}
        inventory.append(entry)
        name = package["name"] + "-" + package["version"]
        for path in manifest.parent.iterdir():
            if path.is_file() and path.name.upper().startswith(("LICENSE", "COPYING", "NOTICE", "COPYRIGHT")):
                (notices / name).mkdir(exist_ok=True)
                shutil.copyfile(path, notices / name / path.name)
    (notices / "inventory.json").write_text(json.dumps(inventory, indent=2) + "\n")
    shutil.copyfile(unpacked / "gtk4-layer-shell/LICENSE", notices / "gtk4-layer-shell.LICENSE")
    for path in (destination / "vendor/polkit-agent-rs").glob("LICENSE*"):
        shutil.copyfile(path, notices / ("polkit-agent-rs." + path.name))
    with urllib.request.urlopen("https://raw.githubusercontent.com/spdx/license-list-data/main/text/CC-BY-SA-4.0.txt", timeout=30) as response:
        (notices / "CC-BY-SA-4.0.txt").write_bytes(response.read())
    (destination / "source-info.json").write_text(json.dumps({
        "source_ref": args.source_ref, "debian_version": args.version,
        "cargo_lock_sha256": hashlib.sha256((destination / "Cargo.lock").read_bytes()).hexdigest(),
        "gtk4_layer_shell_sha256": NATIVE_SHA, "network_policy": "source preparation only; offline package build",
        "native_patches_sha256": {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                                  for p in sorted((destination / "packaging/native").glob("*.patch"))},
    }, indent=2) + "\n")
    (destination / "debian/rules").chmod(0o755)
    print(destination, flush=True)


if __name__ == "__main__":
    main()
