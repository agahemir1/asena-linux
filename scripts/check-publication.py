"""Check publication candidates and reachable Git history without printing secrets."""

from pathlib import Path, PurePosixPath
import re
import subprocess
import sys


MAX_FILE_SIZE = 100 * 1024 * 1024
PRIVATE_DIRS = {".netlify", ".claude", ".ssh", ".gnupg"}
GENERATED_DIRS = {"target", "node_modules", ".astro"}
ARTIFACT_SUFFIXES = {".iso", ".qcow2", ".vdi", ".vmdk"}
PRIVATE_SUFFIXES = {".key", ".p12", ".pfx"}
SECRET_PATTERNS = (
    re.compile(rb"-----BEGIN (?:[A-Z0-9]+ )*PRIVATE KEY-----"),
    re.compile(rb"-----BEGIN PGP " rb"PRIVATE KEY BLOCK-----"),
    re.compile(rb"\bgh[pousr]_[A-Za-z0-9]{36,}\b"),
    re.compile(rb"\bgithub_pat_[A-Za-z0-9_]{60,}\b"),
    re.compile(rb"\bAKIA[A-Z0-9]{16}\b"),
)


def git(root, *args, data=None):
    return subprocess.run(
        ["git", "-C", str(root), *args], input=data,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True,
    ).stdout


def path_problem(name):
    path = PurePosixPath(name)
    if PRIVATE_DIRS.intersection(path.parts):
        return "local account or agent configuration"
    if GENERATED_DIRS.intersection(path.parts) or name.startswith("website/dist/"):
        return "generated output"
    if path.name == ".env" or (
        path.name.startswith(".env.") and path.name != ".env.example"
    ):
        return "environment file"
    if path.name in {"id_rsa", "id_ed25519", "id_ecdsa", "credentials.json"}:
        return "credential filename"
    if path.suffix.lower() in PRIVATE_SUFFIXES:
        return "key or identity file; review outside the source repository"
    if path.suffix.lower() in ARTIFACT_SUFFIXES:
        return "disk or ISO image"
    return None


def content_problem(data):
    if any(pattern.search(data) for pattern in SECRET_PATTERNS):
        return "possible private key or access token"
    return None


def check(root):
    problems = set()
    names = set(git(root, "ls-files", "--cached", "--others", "--exclude-standard", "-z")
                .decode("utf-8").split("\0")) - {""}
    worktree_count = 0
    for name in sorted(names):
        path = root / name
        if not path.is_file() and not path.is_symlink():
            continue
        worktree_count += 1
        reason = path_problem(name)
        if reason:
            problems.add(f"working tree: {name}: {reason}")
        if path.is_symlink():
            data = str(path.readlink()).encode("utf-8")
        elif path.stat().st_size >= MAX_FILE_SIZE:
            problems.add(f"working tree: {name}: file reaches the 100 MiB limit")
            continue
        else:
            data = path.read_bytes()
        reason = content_problem(data)
        if reason:
            problems.add(f"working tree: {name}: {reason}")

    objects = {}
    for line in git(root, "rev-list", "--objects", "HEAD", "--branches", "--tags").decode("utf-8").splitlines():
        oid, _, name = line.partition(" ")
        objects[oid] = name

    metadata = git(root, "cat-file", "--batch-check",
                   data="".join(f"{oid}\n" for oid in objects).encode("ascii"))
    blobs = []
    for line in metadata.decode("ascii").splitlines():
        oid, kind, size = line.split()
        if kind == "blob":
            blobs.append((oid, int(size)))
            name = objects[oid]
            if name and (reason := path_problem(name)):
                problems.add(f"history: {name}: {reason}")

    # One object per request keeps large histories out of memory.
    with subprocess.Popen(
        ["git", "-C", str(root), "cat-file", "--batch"],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE,
    ) as process:
        for oid, size in blobs:
            label = objects[oid] or oid
            if size >= MAX_FILE_SIZE:
                problems.add(f"history: {label}: file reaches the 100 MiB limit")
                continue
            process.stdin.write(f"{oid}\n".encode("ascii"))
            process.stdin.flush()
            header = process.stdout.readline().decode("ascii").split()
            if header != [oid, "blob", str(size)]:
                raise ValueError("Unexpected Git object header")
            data = process.stdout.read(size)
            if len(data) != size or process.stdout.read(1) != b"\n":
                raise ValueError("Incomplete Git object")
            if reason := content_problem(data):
                problems.add(f"history: {label} ({oid[:12]}): {reason}")
        process.stdin.close()
        if process.wait() != 0:
            raise ValueError("Git object inspection failed")
    return sorted(problems), worktree_count, len(blobs)


def main():
    root = Path(__file__).resolve().parents[1]
    try:
        problems, files, blobs = check(root)
    except (OSError, ValueError, subprocess.CalledProcessError):
        print("Publication check could not complete; no files were changed.", file=sys.stderr)
        return 1
    if problems:
        print("Publication check failed. Findings contain paths only:", file=sys.stderr)
        for problem in problems:
            print(f"  {problem}", file=sys.stderr)
        return 1
    print(f"Publication checks passed: {files} candidate files, {blobs} historical blobs.")
    print("Checked known credential patterns, local files and size limits; review before publishing.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
