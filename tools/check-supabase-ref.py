#!/usr/bin/env python3
"""check-supabase-ref: keep Proxim away from the wrong Supabase project.

Rules (docs/PLAN.md section 2 and 12, docs/DECISIONS.md):
  1. A ref listed in tools/forbidden-supabase-refs.txt (the existing "ilios
     Healthcare" project) must not appear in any config-like file.
  2. Any Supabase project reference that IS configured (project_id in
     supabase/config.toml or a [remotes.*] block, supabase/.temp/project-ref,
     SUPABASE_PROJECT_REF style keys, `--project-ref X`, https://<ref>.supabase.co
     URLs in .env* files or workflows) must be listed in
     tools/allowed-supabase-refs.txt. That list is EMPTY until the owner
     confirms creation of a Proxim project.
  3. Values that are variables (${{ secrets.X }}, $VAR, ${VAR}) cannot be
     checked statically; pass the runtime value with --ref in CI.

Run:   python3 -I tools/check-supabase-ref.py [--root DIR] [--ref VALUE ...]

Exit codes: 0 = clean, 1 = violation, 2 = tool error.
Stdlib only.
"""

from __future__ import annotations

import argparse
import os
import re
import sys
import tomllib
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

SKIP_DIRS = {".git", "node_modules", "target", ".claude", ".venv", "__pycache__", ".next", ".turbo", "dist"}
# Prose and test fixtures are not configuration. tools/ lists are handled explicitly.
SKIP_PREFIXES_FOR_BROAD_SCAN = ("docs/", "tools/tests/")
BROAD_SUFFIXES = {
    ".toml", ".yml", ".yaml", ".json", ".sh", ".bash", ".env", ".ts", ".tsx", ".js", ".mjs", ".cjs",
    ".py", ".rs", ".sql", ".txt", ".cfg", ".ini", ".conf", ".mk", "",
}
MAX_BYTES = 1_000_000

REF_SHAPE = r"[a-z0-9]{20}"
URL_REF_RE = re.compile(rf"(?<![a-z0-9])({REF_SHAPE})\.(?:[a-z]+\.)?(?:supabase\.co|supabase\.in|supabase\.net)", re.I)
DB_HOST_REF_RE = re.compile(rf"db\.({REF_SHAPE})\.supabase\.co", re.I)
POOLER_USER_RE = re.compile(rf"postgres\.({REF_SHAPE})(?![a-z0-9])", re.I)
# Only keys that are unambiguously about Supabase: other platforms also have "project id" keys
# (Vercel prj_..., GCP) that must not be mistaken for a Supabase ref.
KEY_VALUE_RE = re.compile(
    r"(?:supabase[_-]?(?:[a-z]+[_-])*project[_-]?(?:ref|id)|project[_-]?ref)[\"']?\s*[:=]\s*[\"']?(?P<val>[^\s\"',#}]+)",
    re.I,
)
CLI_FLAG_RE = re.compile(r"--project-ref[=\s]+[\"']?(?P<val>[^\s\"']+)", re.I)


def read_list(path: Path) -> list[str]:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as exc:
        raise SystemExit(f"check-supabase-ref: cannot read {path}: {exc}") from exc
    out = []
    for raw in text.splitlines():
        line = raw.split("#", 1)[0].strip()
        if line:
            out.append(line.lower())
    return out


def is_variable(value: str) -> bool:
    return value.startswith("$") or "${{" in value or value.startswith("%") or value.startswith("<")


def iter_files(root: Path):
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in SKIP_DIRS]
        for name in filenames:
            yield Path(dirpath) / name


def read_text(path: Path) -> str | None:
    try:
        if path.stat().st_size > MAX_BYTES:
            return None
        data = path.read_bytes()
    except OSError:
        return None
    if b"\x00" in data[:4096]:
        return None
    return data.decode("utf-8", errors="ignore")


def is_config_file(rel: Path) -> bool:
    parts = rel.parts
    name = rel.name
    if name.startswith(".env") or name.endswith(".env"):
        return True
    if name == "config.toml" and "supabase" in parts[:-1]:
        return True
    if "supabase" in parts[:-1] and ".temp" in parts[:-1]:
        return True
    if len(parts) >= 3 and parts[0] == ".github" and name.endswith((".yml", ".yaml")):
        return True
    return False


def toml_project_ids(text: str) -> list[str]:
    """Every `project_id` value anywhere in a TOML document (top level and [remotes.*])."""
    try:
        data = tomllib.loads(text)
    except tomllib.TOMLDecodeError:
        return []
    found: list[str] = []

    def walk(node):
        if isinstance(node, dict):
            for key, value in node.items():
                if key == "project_id" and isinstance(value, str):
                    found.append(value)
                else:
                    walk(value)
        elif isinstance(node, list):
            for item in node:
                walk(item)

    walk(data)
    return found


def configured_refs(rel: Path, text: str) -> list[tuple[str, str]]:
    """Return (ref, how) pairs for every statically visible project reference."""
    refs: list[tuple[str, str]] = []
    if rel.name == "config.toml":
        for pid in toml_project_ids(text):
            refs.append((pid, "project_id in config.toml"))
    if ".temp" in rel.parts and rel.name == "project-ref":
        value = text.strip()
        if value:
            refs.append((value, "supabase/.temp/project-ref"))
    for regex, how in ((URL_REF_RE, "supabase URL"), (DB_HOST_REF_RE, "db host"), (POOLER_USER_RE, "pooler user")):
        for m in regex.finditer(text):
            refs.append((m.group(1), how))
    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith("#") and rel.name != "config.toml":
            # commented-out lines in env files are still a smell, but we only fail on live config.
            continue
        if rel.name == "config.toml":
            continue  # handled by the TOML parser above
        for m in KEY_VALUE_RE.finditer(line):
            refs.append((m.group("val"), "project ref/id key"))
        for m in CLI_FLAG_RE.finditer(line):
            refs.append((m.group("val"), "--project-ref flag"))
    return refs


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--root", default=str(REPO_ROOT))
    ap.add_argument("--forbidden", default=str(REPO_ROOT / "tools" / "forbidden-supabase-refs.txt"))
    ap.add_argument("--allowed", default=str(REPO_ROOT / "tools" / "allowed-supabase-refs.txt"))
    ap.add_argument("--ref", action="append", default=[], help="a runtime value to check (for example a CI secret); repeatable")
    args = ap.parse_args(argv)

    try:
        forbidden = read_list(Path(args.forbidden))
        allowed = read_list(Path(args.allowed))
    except SystemExit as exc:
        sys.stderr.write(str(exc.code) + "\n")
        return 2
    if not forbidden:
        sys.stderr.write("check-supabase-ref: forbidden list is empty (refusing to pass vacuously)\n")
        return 2

    root = Path(args.root).resolve()
    if not root.is_dir():
        sys.stderr.write(f"check-supabase-ref: root not found: {root}\n")
        return 2

    forbidden_set = set(forbidden)
    allowed_set = set(allowed)
    violations: list[str] = []
    scanned = 0
    config_files = 0
    list_files = {Path(args.forbidden).resolve(), Path(args.allowed).resolve()}

    # Runtime values passed by CI.
    for value in args.ref:
        v = value.strip().lower()
        if not v:
            print("check-supabase-ref: note: empty --ref value skipped (secret not set in this run)")
            continue
        if v in forbidden_set:
            violations.append(f"--ref value is a FORBIDDEN project ref ({v})")
        elif v not in allowed_set:
            violations.append(f"--ref value '{v}' is not listed in tools/allowed-supabase-refs.txt")

    for path in iter_files(root):
        if path.resolve() in list_files:
            continue
        rel = path.relative_to(root)
        rel_posix = rel.as_posix()
        config_like = is_config_file(rel)
        broad = (
            not rel_posix.startswith(SKIP_PREFIXES_FOR_BROAD_SCAN)
            and (path.suffix.lower() in BROAD_SUFFIXES or path.name.startswith(".env") or path.name in {"Makefile"})
        )
        if not (config_like or broad):
            continue
        text = read_text(path)
        if text is None:
            continue
        scanned += 1
        lowered = text.lower()
        for ref in forbidden:
            if ref in lowered:
                violations.append(f"{rel_posix}: contains FORBIDDEN Supabase project ref '{ref}'")
        if config_like:
            config_files += 1
            for ref, how in configured_refs(rel, text):
                value = ref.strip().strip("\"'").lower()
                if not value or is_variable(value):
                    continue
                if value in forbidden_set:
                    continue  # already reported above
                if value not in allowed_set:
                    violations.append(
                        f"{rel_posix}: configures Supabase project reference '{value}' ({how}) "
                        f"which is not in tools/allowed-supabase-refs.txt"
                    )

    # De-duplicate while keeping order.
    seen = set()
    unique = [v for v in violations if not (v in seen or seen.add(v))]
    if unique:
        print(f"check-supabase-ref: FAIL ({len(unique)} violation(s))")
        for v in unique:
            print(f"  - {v}")
        return 1
    print(f"check-supabase-ref: OK ({scanned} file(s) scanned, {config_files} config-like, "
          f"{len(forbidden)} forbidden ref(s), {len(allowed)} allowed ref(s))")
    return 0


if __name__ == "__main__":
    sys.exit(main())
