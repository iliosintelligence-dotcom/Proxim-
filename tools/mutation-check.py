#!/usr/bin/env python3
"""mutation-check: deliberately broken engines must turn the scenario suite red.

For each mutation below, copy core/ (without target/) to a temporary directory, apply one exact
text substitution to an engine source file IN THE COPY, build the proxim-sim CLI there and run
every scenario (`proxim-sim run testdata/scenarios`, CI-sized seeds). The mutation is "killed"
when the run exits 1 and each scenario named in `expect_red` is reported FAIL. An unmutated copy
is run first and must pass. The working tree is never written: only the copy is changed, and it
is deleted afterwards.

Run:   python3 -I tools/mutation-check.py [--only NAME ...] [--target-dir DIR] [--list]
       (--target-dir reuses a Cargo build directory between runs; default: a directory under the
        system temp dir that is kept so reruns are incremental)
Exit:  0 = every mutation killed, 1 = a mutation survived or the baseline failed, 2 = tool error
       (a substitution did not match exactly once, cargo missing, ...).
Stdlib only.
"""

from __future__ import annotations

import argparse
import hashlib
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
CORE = REPO_ROOT / "core"


@dataclass(frozen=True)
class Mutation:
    name: str
    file: str  # relative to core/
    old: str
    new: str
    expect_red: tuple[str, ...]
    why: str


MUTATIONS = (
    Mutation(
        "enter-no-persistence",
        "crates/proxim-detect/src/enter.rs",
        "if n2 >= ENTER_MIN_FIXES && t.saturating_sub(first_t) >= span_needed {",
        "if true {",
        ("flap",),
        "remove the N_in = 3 fixes / 3 s (10 s slow) persistence: ENTER on the 2nd inside fix",
    ),
    Mutation(
        "approach-no-off-corridor",
        "crates/proxim-detect/src/approach.rs",
        ".any(|i| i.xt_mm.saturating_abs() > r.buffer_mm)",
        ".any(|i| i.xt_mm.saturating_abs() > i64::MAX)",
        ("parallel-frontage-road",),
        "disable the OFF_CORRIDOR gate",
    ),
    Mutation(
        "approach-bearing-flipped",
        "crates/proxim-detect/src/approach.rs",
        "if !dot.is_some_and(|d| d >= r.cos_tol_q30) {",
        "if !dot.is_some_and(|d| d <= -r.cos_tol_q30) {",
        ("expressway-inbound", "walking"),
        "flip the bearing check (accept travel against the route, reject along it)",
    ),
    Mutation(
        "approach-no-bearing",
        "crates/proxim-detect/src/approach.rs",
        "if !dot.is_some_and(|d| d >= r.cos_tol_q30) {",
        "if dot.is_some_and(|_| false) {",
        ("angled-crossing",),
        "remove the bearing gate entirely",
    ),
    Mutation(
        "decide-no-standoff",
        "crates/proxim-decide/src/lib.rs",
        "if standoff::fix_in_standoff(&zones, fix, acc_eff)? {",
        "if standoff::fix_in_standoff(&zones, fix, acc_eff)? && false {",
        ("health-standoff",),
        "disable F10 SENSITIVE_STANDOFF suppression",
    ),
    Mutation(
        "decide-no-day-cap",
        "crates/proxim-decide/src/lib.rs",
        "if book.day_count(sp.id.as_str()) >= ctx.eff.cap_day {",
        "if book.day_count(sp.id.as_str()) >= ctx.eff.cap_day + 1 {",
        ("s11-cap-exhaustion",),
        "allow a third fire per local day",
    ),
)


def tree_digest(root: Path) -> str:
    """SHA-256 over every tracked-looking file under core/ (not target/), to prove no change."""
    h = hashlib.sha256()
    for p in sorted(root.rglob("*")):
        rel = p.relative_to(root)
        if rel.parts and rel.parts[0] == "target":
            continue
        if p.is_file():
            h.update(str(rel).encode())
            h.update(b"\0")
            h.update(p.read_bytes())
    return h.hexdigest()


def copy_core(dst: Path) -> None:
    # shutil.copy (not copy2): every copied file gets a fresh mtime. Cargo fingerprints path
    # crates by mtime, and with preserved mtimes it reused the previous mutation's objects from the
    # shared target dir (observed: a decide mutation reported the detect mutation's failures).
    shutil.copytree(
        CORE, dst, ignore=shutil.ignore_patterns("target", "node_modules"), copy_function=shutil.copy
    )
    toolchain = REPO_ROOT / "rust-toolchain.toml"
    if toolchain.is_file():
        shutil.copy2(toolchain, dst.parent / "rust-toolchain.toml")


def run_suite(core_copy: Path, target_dir: Path) -> tuple[int, str]:
    cmd = [
        "cargo", "run", "--quiet", "--release", "--manifest-path", str(core_copy / "Cargo.toml"),
        "-p", "proxim-sim", "--bin", "proxim-sim", "--", "run", str(core_copy / "testdata" / "scenarios"),
    ]
    env = dict(**__import__("os").environ, CARGO_TARGET_DIR=str(target_dir))
    env.pop("PROXIM_SIM_SEEDS", None)
    try:
        proc = subprocess.run(cmd, cwd=core_copy, capture_output=True, text=True, env=env, check=False)
    except FileNotFoundError as exc:
        raise SystemExit("mutation-check: `cargo` not found on PATH") from exc
    return proc.returncode, proc.stdout + proc.stderr


def failed_ids(output: str) -> set[str]:
    out = set()
    for line in output.splitlines():
        parts = line.split()
        if len(parts) >= 2 and parts[0] == "FAIL":
            out.add(parts[1])
    return out


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--only", nargs="*", default=None)
    ap.add_argument("--target-dir", default=str(Path(tempfile.gettempdir()) / "proxim-mutation-target"))
    ap.add_argument("--list", action="store_true")
    args = ap.parse_args(argv)
    muts = [m for m in MUTATIONS if args.only is None or m.name in args.only]
    if args.list:
        for m in MUTATIONS:
            print(f"{m.name:28} {m.file:40} expect red: {', '.join(m.expect_red)}  ({m.why})")
        return 0
    if args.only is not None and len(muts) != len(args.only):
        print(f"mutation-check: unknown mutation in {args.only}", file=sys.stderr)
        return 2
    before = tree_digest(CORE)
    target_dir = Path(args.target_dir)
    results = []
    status = 0
    try:
        with tempfile.TemporaryDirectory(prefix="proxim-mutation-") as tmp:
            # Baseline: the unmodified engine must pass.
            base = Path(tmp) / "baseline" / "core"
            copy_core(base)
            code, out = run_suite(base, target_dir)
            if code != 0:
                print(out)
                print("mutation-check: BASELINE FAILED (the unmutated suite must pass)")
                return 1
            print("baseline                     PASS (unmutated suite is green)")
            for m in muts:
                work = Path(tmp) / m.name / "core"
                copy_core(work)
                f = work / m.file
                text = f.read_text(encoding="utf-8")
                n = text.count(m.old)
                if n != 1:
                    print(f"mutation-check: {m.name}: pattern found {n} times in {m.file} (need exactly 1)")
                    return 2
                f.write_text(text.replace(m.old, m.new), encoding="utf-8")
                code, out = run_suite(work, target_dir)
                fails = failed_ids(out)
                missing = [s for s in m.expect_red if s not in fails]
                if code == 1 and not missing:
                    verdict = "KILLED"
                elif code not in (0, 1):
                    print(out)
                    verdict = f"ERROR (exit {code})"
                    status = max(status, 2)
                else:
                    verdict = "SURVIVED"
                    status = max(status, 1)
                results.append((m, verdict, sorted(fails), missing))
                print(f"{m.name:28} {verdict:9} red: {', '.join(sorted(fails)) or '-'}"
                      + (f"  (expected red but green: {', '.join(missing)})" if missing else ""))
                shutil.rmtree(work.parent)
    finally:
        after = tree_digest(CORE)
        if after != before:
            print("mutation-check: THE WORKING TREE CHANGED (this must never happen)")
            status = 2
    killed = sum(1 for _, v, _, _ in results if v == "KILLED")
    print(f"mutation-check: {killed}/{len(results)} mutation(s) killed; working tree unchanged")
    return status


if __name__ == "__main__":
    sys.exit(main())
