#!/usr/bin/env python3
"""Offline publication gate. Report locations/categories, never matched values."""
import argparse
from pathlib import Path
import re
import subprocess
import sys

SECRET = re.compile(rb"(?:sk-(?:proj-|svcacct-)?[A-Za-z0-9_-]{24,}|gh[pousr]_[A-Za-z0-9_]{30,}|github_pat_[A-Za-z0-9_]{30,}|AKIA[0-9A-Z]{16}|-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----)")
HOME = re.compile(rb"(?:[A-Za-z]:[\\/]+Users[\\/]+[^\\/\x00\r\n\t \"<>]+|(?<![A-Za-z0-9_./-])/(?:Users|home)/[A-Za-z0-9_.-]+)")
EMAIL = re.compile(rb"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}")


def git(*args):
    return subprocess.check_output(["git", *args])


def public_email(email):
    return email.endswith(b"@users.noreply.github.com") or email == b"noreply@github.com"


def inspect_source():
    issues = []
    files = git("ls-files", "--cached", "--others", "--exclude-standard", "-z").decode().split("\0")
    for name in files:
        path = Path(name)
        if not path.is_file():
            continue
        data = path.read_bytes()
        for label, pattern in [("credential/private-key format", SECRET), ("personal home path", HOME)]:
            for match in pattern.finditer(data):
                issues.append(f"{name}:{data[:match.start()].count(bytes([10])) + 1}: {label}")
        if path.suffix.lower() in {".otf", ".png", ".rgba"} or name.startswith("licenses/"):
            continue
        for match in EMAIL.finditer(data):
            email = match.group()
            domain = email.split(b"@")[-1]
            if not public_email(email) and domain not in {b"example.com", b"example.org", b"example.net"}:
                issues.append(f"{name}:{data[:match.start()].count(bytes([10])) + 1}: non-noreply email")
    return issues


def inspect_history():
    issues = []
    for commit in git("rev-list", "--all").decode().splitlines():
        data = git("show", "-s", "--format=%an%x00%ae%x00%cn%x00%ce%x00%B", commit)
        author, email, committer, committer_email, body = data.split(b"\0", 4)
        if not public_email(email) or not public_email(committer_email):
            issues.append(f"commit {commit[:12]}: author/committer email is not noreply")
        if not all(re.fullmatch(rb"[A-Za-z0-9_.\[\]-]+", name) for name in (author, committer)):
            issues.append(f"commit {commit[:12]}: use a public handle instead of a personal name")
        if any(not public_email(m.group()) for m in EMAIL.finditer(body)):
            issues.append(f"commit {commit[:12]}: non-noreply email in commit message")
    return issues


def inspect_binary(path):
    data = path.read_bytes()
    issues = []
    # UTF-8/ASCII strings and UTF-16LE resources can both disclose build paths.
    for encoding in ("utf-8", "utf-16le"):
        decoded = data if encoding == "utf-8" else data.decode(encoding, errors="ignore").encode("utf-8")
        if HOME.search(decoded):
            issues.append(f"{path.name}: personal home path in {encoding} binary strings")
    if SECRET.search(data):
        issues.append(f"{path.name}: credential/private-key format in binary")
    return issues


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--history", action="store_true")
    parser.add_argument("--binary", type=Path)
    args = parser.parse_args()
    issues = inspect_source()
    if args.history:
        issues.extend(inspect_history())
    if args.binary:
        issues.extend(inspect_binary(args.binary))
    if issues:
        print("Publication check failed:", file=sys.stderr)
        for issue in sorted(set(issues)):
            print(f"  {issue}", file=sys.stderr)
        return 1
    print("Publication privacy check passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
