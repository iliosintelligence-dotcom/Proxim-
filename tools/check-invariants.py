#!/usr/bin/env python3
"""check-invariants: every invariant must map to a named test, and 'enforced' must be true.

Parses the markdown tables in docs/privacy-invariants.md. A table is read when
its header row has the columns ID, Status and Test ID (case-insensitive);
Milestone and Guards are read when present.

Rules
  * Every row needs an ID, a Status in {enforced, planned, design-rule}, a
    Milestone and a non-empty Test ID.                         -> else FAIL
  * IDs are unique.                                             -> else FAIL
  * enforced:     every Test ID entry must resolve to something that exists
                  in the repo today.                            -> else FAIL
  * planned:      listed as a warning (backlog). If its test already exists the
                  tool warns that the row should be flipped to enforced.
  * design-rule:  no guarded code path may exist yet. Every path glob in the
                  Guards column must match NOTHING in the repo.   -> else FAIL
                  (flipping a design-rule needs a passing test and a status
                  change in the same change.)
  * The required invariant IDs (I1..I12 from PLAN 6.1, I2a and I13..I26 from
    the addenda) must all have a row. A missing row is an invariant with no
    test.                                                       -> else FAIL

Test ID grammar (one or more, separated by ';' or '<br>'):
  path/to/file::test_name   file exists and test_name appears in it as a whole word
  tool:NAME                 tools/NAME.py exists and tools/tests/test_tools.py mentions NAME

Run:   python3 -I tools/check-invariants.py [--root DIR] [--doc FILE] [--no-require-ids] [--list-planned]
Exit:  0 = ok (warnings allowed), 1 = violation, 2 = tool error.
Stdlib only.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
STATUSES = ("enforced", "planned", "design-rule")
REQUIRED_IDS = (
    [f"I{n}" for n in range(1, 13)]  # PLAN 6.1
    + ["I2a"]  # Addendum 01 9.5
    + [f"I{n}" for n in range(13, 27)]  # Addendum 01 (I13-I20), Addendum 02 (I21-I26)
)
SEPARATOR_CELL_RE = re.compile(r"^:?-{2,}:?$")


def split_row(line: str) -> list[str]:
    line = line.strip()
    if line.startswith("|"):
        line = line[1:]
    if line.endswith("|"):
        line = line[:-1]
    return [c.strip() for c in line.split("|")]


def clean(cell: str) -> str:
    """Strip markdown decoration: backticks anywhere, and bold only when it wraps the whole cell
    (a '**' inside a glob such as core/crates/**/x.rs must survive)."""
    cell = cell.replace("`", "").strip()
    if len(cell) > 4 and cell.startswith("**") and cell.endswith("**"):
        cell = cell[2:-2]
    return cell.strip()


def parse_tables(text: str) -> list[dict]:
    rows: list[dict] = []
    lines = text.splitlines()
    i = 0
    while i < len(lines):
        line = lines[i]
        if line.lstrip().startswith("|") and i + 1 < len(lines):
            header = [clean(c).lower() for c in split_row(line)]
            sep = split_row(lines[i + 1]) if lines[i + 1].lstrip().startswith("|") else []
            if sep and all(SEPARATOR_CELL_RE.match(c.strip()) for c in sep) and {"id", "status", "test id"} <= set(header):
                idx = {name: header.index(name) for name in header}
                j = i + 2
                while j < len(lines) and lines[j].lstrip().startswith("|"):
                    cells = split_row(lines[j])
                    rec = {"line": j + 1}
                    for name, k in idx.items():
                        rec[name] = clean(cells[k]) if k < len(cells) else ""
                    rows.append(rec)
                    j += 1
                i = j
                continue
        i += 1
    return rows


def split_ids(cell: str) -> list[str]:
    parts = re.split(r"\s*(?:;|<br\s*/?>)\s*", cell)
    return [clean(p) for p in parts if clean(p)]


def whole_word(name: str, text: str) -> bool:
    return re.search(rf"(?<![A-Za-z0-9_]){re.escape(name)}(?![A-Za-z0-9_])", text) is not None


def resolve_test(entry: str, root: Path) -> tuple[bool, str]:
    """Return (exists, explanation)."""
    if entry.startswith("tool:"):
        name = entry[len("tool:"):].strip()
        if not re.fullmatch(r"[A-Za-z0-9_-]+", name):
            return False, f"bad tool name '{name}'"
        tool = root / "tools" / f"{name}.py"
        if not tool.is_file():
            return False, f"tools/{name}.py does not exist"
        selftest = root / "tools" / "tests" / "test_tools.py"
        if not selftest.is_file() or name not in selftest.read_text(encoding="utf-8", errors="replace"):
            return False, f"tools/tests/test_tools.py has no self-test mentioning '{name}'"
        return True, f"tools/{name}.py with self-test"
    if "::" in entry:
        rel, name = entry.split("::", 1)
        rel, name = rel.strip(), name.strip().split("::")[-1]  # module paths: only the leaf test name is searched
        if not rel or not name:
            return False, "empty path or test name"
        if Path(rel).is_absolute() or ".." in Path(rel).parts:
            return False, "path must be repo-relative"
        path = root / rel
        if not path.is_file():
            return False, f"file '{rel}' does not exist"
        text = path.read_text(encoding="utf-8", errors="replace")
        if not whole_word(name, text):
            return False, f"'{name}' not found in {rel}"
        return True, f"{rel} contains {name}"
    return False, "not of the form path::test_name or tool:NAME"


def guard_hits(globs: str, root: Path) -> list[str]:
    hits: list[str] = []
    for pattern in split_ids(globs):
        if pattern in {"-", "none", "n/a"}:
            continue
        if Path(pattern).is_absolute() or ".." in Path(pattern).parts:
            hits.append(f"(invalid guard glob '{pattern}')")
            continue
        for match in sorted(root.glob(pattern)):
            hits.append(match.relative_to(root).as_posix())
    return hits


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--root", default=str(REPO_ROOT))
    ap.add_argument("--doc", default=None, help="invariants document (default: <root>/docs/privacy-invariants.md)")
    ap.add_argument("--no-require-ids", action="store_true", help="do not insist that I1..I26 all have rows (self-tests)")
    ap.add_argument("--list-planned", action="store_true", help="print the planned backlog even when there are no problems")
    args = ap.parse_args(argv)

    root = Path(args.root).resolve()
    doc = Path(args.doc) if args.doc else root / "docs" / "privacy-invariants.md"
    try:
        text = doc.read_text(encoding="utf-8")
    except OSError as exc:
        sys.stderr.write(f"check-invariants: cannot read {doc}: {exc}\n")
        return 2

    rows = parse_tables(text)
    if not rows:
        sys.stderr.write(f"check-invariants: no invariant table found in {doc} (refusing to pass vacuously)\n")
        return 2

    errors: list[str] = []
    warnings: list[str] = []
    planned: list[dict] = []
    seen: dict[str, int] = {}
    counts = {s: 0 for s in STATUSES}

    for row in rows:
        rid = row.get("id", "")
        where = f"line {row['line']} [{rid or '?'}]"
        if not rid:
            errors.append(f"{where}: row has no ID")
            continue
        if rid in seen:
            errors.append(f"{where}: duplicate ID (first at line {seen[rid]})")
        seen[rid] = row["line"]

        status = row.get("status", "").lower()
        tests = split_ids(row.get("test id", ""))
        milestone = row.get("milestone", "") if "milestone" in row else "n/a"
        if status not in STATUSES:
            errors.append(f"{where}: status '{row.get('status', '')}' is not one of {', '.join(STATUSES)}")
            continue
        counts[status] += 1
        if not tests or tests[0].lower() in {"-", "none", "n/a", "tbd"}:
            errors.append(f"{where}: no Test ID (every invariant must map to a named test or tool)")
            continue
        if not milestone:
            errors.append(f"{where}: no Milestone")
            continue

        if status == "enforced":
            for entry in tests:
                ok, why = resolve_test(entry, root)
                if not ok:
                    errors.append(f"{where}: status is 'enforced' but test '{entry}' does not exist: {why}")
        elif status == "planned":
            planned.append(row)
            existing = [e for e in tests if resolve_test(e, root)[0]]
            if existing and len(existing) == len(tests):
                warnings.append(f"{where}: planned, but all its tests now exist ({'; '.join(existing)}): flip to 'enforced'?")
        else:  # design-rule
            hits = guard_hits(row.get("guards", ""), root) if "guards" in row else []
            for hit in hits:
                errors.append(
                    f"{where}: status is 'design-rule' but guarded path exists: {hit} "
                    f"(add the named test and change the status in the same change)"
                )

    if not args.no_require_ids:
        for rid in REQUIRED_IDS:
            if rid not in seen:
                errors.append(f"required invariant {rid} has no row in {doc.name} (an invariant with no test fails the build)")

    for w in warnings:
        print(f"WARN {w}")
    if planned and (args.list_planned or not errors):
        print(f"planned backlog ({len(planned)}): these rows are NOT enforced today")
        for row in planned:
            print(f"  WARN planned {row['id']:<6} {row.get('milestone', ''):<22} {row.get('test id', '')}")
    summary = ", ".join(f"{counts[s]} {s}" for s in STATUSES)
    if errors:
        print(f"check-invariants: FAIL ({len(errors)} problem(s); {summary})")
        for e in errors:
            print(f"  - {e}")
        return 1
    print(f"check-invariants: OK ({len(rows)} row(s): {summary})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
