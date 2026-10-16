#!/usr/bin/env python3
"""check-no-network: privacy invariant I1 (the core has no network dependency).

Fails (exit 1) if ANY crate in the Cargo workspace depends, directly or
transitively, on a crate matching tools/network-denylist.txt, or if a workspace
crate declares such a dependency (even an optional one), or if workspace Rust
source names a socket type.

Run:   python3 -I tools/check-no-network.py [options]

Exit codes: 0 = clean, 1 = violation found, 2 = tool error (cargo missing,
manifest missing, unreadable metadata). Tool errors are failures on purpose: a
guard that cannot run must not look green.

Options:
  --manifest PATH        Cargo manifest to check (default: core/Cargo.toml)
  --metadata-json PATH   read `cargo metadata` JSON from a file instead of running cargo
  --denylist PATH        denylist file (default: tools/network-denylist.txt)
  --locked / --offline   passed through to `cargo metadata`
  --no-source-scan       skip the grep for TcpStream / UdpSocket / std::net in *.rs

Stdlib only.
"""

from __future__ import annotations

import argparse
import fnmatch
import json
import re
import subprocess
import sys
from collections import deque
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

# Source-level backstop for code that opens sockets without a crate.
SOCKET_RE = re.compile(r"\b(TcpStream|TcpListener|UdpSocket|UnixStream|UnixListener)\b|\bstd::net::|\bcore::net::")


def load_denylist(path: Path) -> list[str]:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as exc:
        raise SystemExit(f"check-no-network: cannot read denylist {path}: {exc}") from exc
    patterns = []
    for raw in text.splitlines():
        line = raw.strip()
        if line and not line.startswith("#"):
            patterns.append(line)
    if not patterns:
        raise SystemExit(f"check-no-network: denylist {path} has no patterns (refusing to pass vacuously)")
    return patterns


def denied_by(name: str, patterns: list[str]) -> str | None:
    for pat in patterns:
        if fnmatch.fnmatchcase(name, pat):
            return pat
    return None


def run_cargo_metadata(manifest: Path, locked: bool, offline: bool) -> dict:
    cmd = [
        "cargo",
        "metadata",
        "--format-version",
        "1",
        "--all-features",
        "--manifest-path",
        str(manifest),
    ]
    if locked:
        cmd.append("--locked")
    if offline:
        cmd.append("--offline")
    try:
        proc = subprocess.run(
            cmd,
            cwd=manifest.parent,
            capture_output=True,
            text=True,
            check=False,
        )
    except FileNotFoundError as exc:
        raise SystemExit("check-no-network: `cargo` not found on PATH") from exc
    if proc.returncode != 0:
        sys.stderr.write(proc.stderr)
        raise SystemExit(f"check-no-network: `cargo metadata` failed (exit {proc.returncode})")
    try:
        return json.loads(proc.stdout)
    except json.JSONDecodeError as exc:
        raise SystemExit(f"check-no-network: cargo metadata did not return JSON: {exc}") from exc


def analyze(metadata: dict, patterns: list[str]) -> list[str]:
    """Return a list of human-readable violations."""
    packages = {p["id"]: p for p in metadata.get("packages", [])}
    members = list(metadata.get("workspace_members", []))
    resolve = metadata.get("resolve")
    if not members:
        raise SystemExit("check-no-network: metadata has no workspace_members (refusing to pass vacuously)")
    if resolve is None:
        raise SystemExit("check-no-network: metadata has no resolve graph (was --no-deps used?)")
    graph: dict[str, list[tuple[str, list[str]]]] = {}
    for node in resolve.get("nodes", []):
        edges = []
        for dep in node.get("deps", []):
            kinds = sorted({(k.get("kind") or "normal") for k in dep.get("dep_kinds", [])}) or ["normal"]
            edges.append((dep["pkg"], kinds))
        graph[node["id"]] = edges

    def name_of(pkg_id: str) -> str:
        pkg = packages.get(pkg_id)
        return pkg["name"] if pkg else pkg_id

    violations: list[str] = []
    seen_violation: set[tuple[str, str]] = set()

    # 1. Declared dependencies of workspace crates (catches optional / unresolved ones).
    for member in members:
        pkg = packages.get(member)
        if pkg is None:
            continue
        for dep in pkg.get("dependencies", []):
            hit = denied_by(dep["name"], patterns)
            if hit:
                key = (pkg["name"], dep["name"])
                if key in seen_violation:
                    continue
                seen_violation.add(key)
                kind = dep.get("kind") or "normal"
                opt = " (optional)" if dep.get("optional") else ""
                violations.append(
                    f"{pkg['name']} declares dependency '{dep['name']}'{opt} [{kind}] matching denylist pattern '{hit}'"
                )

    # 2. Transitive closure over the resolved graph, with a witness path.
    for member in members:
        queue: deque[tuple[str, list[str]]] = deque([(member, [name_of(member)])])
        visited = {member}
        while queue:
            current, path = queue.popleft()
            for child, kinds in graph.get(current, []):
                if child in visited:
                    continue
                visited.add(child)
                child_name = name_of(child)
                child_path = path + [child_name]
                hit = denied_by(child_name, patterns)
                if hit:
                    key = (name_of(member), child_name)
                    if key not in seen_violation:
                        seen_violation.add(key)
                        violations.append(
                            f"{name_of(member)} depends on '{child_name}' ({'/'.join(kinds)}) matching denylist pattern "
                            f"'{hit}' via: {' -> '.join(child_path)}"
                        )
                queue.append((child, child_path))

    # 3. A workspace member that is itself a denylisted name (e.g. a local crate called `reqwest`).
    for member in members:
        hit = denied_by(name_of(member), patterns)
        if hit:
            key = ("<workspace>", name_of(member))
            if key not in seen_violation:
                seen_violation.add(key)
                violations.append(f"workspace crate '{name_of(member)}' matches denylist pattern '{hit}'")
    return violations


def scan_sources(workspace_dir: Path) -> list[str]:
    violations: list[str] = []
    skip_dirs = {"target", ".git", "node_modules"}
    for path in sorted(workspace_dir.rglob("*.rs")):
        if any(part in skip_dirs for part in path.relative_to(workspace_dir).parts):
            continue
        try:
            text = path.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        for lineno, line in enumerate(text.splitlines(), start=1):
            stripped = line.lstrip()
            if stripped.startswith("//"):
                continue
            if SOCKET_RE.search(line):
                rel = path.relative_to(workspace_dir)
                violations.append(f"{rel}:{lineno}: socket / std::net use in engine source: {line.strip()[:100]}")
    return violations


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--manifest", default=str(REPO_ROOT / "core" / "Cargo.toml"))
    ap.add_argument("--metadata-json")
    ap.add_argument("--denylist", default=str(REPO_ROOT / "tools" / "network-denylist.txt"))
    ap.add_argument("--locked", action="store_true")
    ap.add_argument("--offline", action="store_true")
    ap.add_argument("--no-source-scan", action="store_true")
    args = ap.parse_args(argv)

    try:
        patterns = load_denylist(Path(args.denylist))
        manifest = Path(args.manifest)
        if args.metadata_json:
            try:
                metadata = json.loads(Path(args.metadata_json).read_text(encoding="utf-8"))
            except (OSError, json.JSONDecodeError) as exc:
                raise SystemExit(f"check-no-network: cannot read metadata JSON: {exc}") from exc
            workspace_dir = None
        else:
            if not manifest.is_file():
                raise SystemExit(f"check-no-network: manifest not found: {manifest}")
            metadata = run_cargo_metadata(manifest, args.locked, args.offline)
            workspace_dir = manifest.parent

        violations = analyze(metadata, patterns)
        if workspace_dir is not None and not args.no_source_scan:
            violations += scan_sources(workspace_dir)
    except SystemExit as exc:
        if isinstance(exc.code, str):
            sys.stderr.write(exc.code + "\n")
            return 2
        raise

    n_members = len(metadata.get("workspace_members", []))
    if violations:
        print(f"check-no-network: FAIL ({len(violations)} violation(s), {n_members} workspace crate(s) checked)")
        for v in violations:
            print(f"  - {v}")
        return 1
    n_pkgs = len(metadata.get("packages", []))
    print(f"check-no-network: OK ({n_members} workspace crate(s), {n_pkgs} package(s) in the resolved graph, "
          f"{len(patterns)} denylist pattern(s))")
    return 0


if __name__ == "__main__":
    sys.exit(main())
