"""Self-tests for the repo guard tools (stdlib unittest, no network, no core/ access for bad cases).

Run:  python3 -I -m unittest discover -s tools/tests -v        (from the repo root)
 or:  python3 -I tools/tests/test_tools.py

Every tool is exercised through its real command line (`python -I tools/<name>.py`)
so the exit codes CI relies on are what is tested. Each tool must FAIL on a
synthetic bad input and PASS on a good one.
"""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path

TOOLS = Path(__file__).resolve().parent.parent
REPO = TOOLS.parent
HEALTHCARE_REF = "wjsjkemamttsuilhihip"  # the existing "ilios Healthcare" project; must be forbidden forever
FAKE_REF = "abcdefghijklmnopqrst"  # 20 chars, synthetic


def run_tool(name: str, *args: str, cwd: Path | None = None) -> subprocess.CompletedProcess:
    return subprocess.run(
        [sys.executable, "-I", str(TOOLS / f"{name}.py"), *args],
        capture_output=True,
        text=True,
        cwd=cwd,
        check=False,
    )


class TempDirCase(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory(prefix="proxim-tools-test-")
        self.addCleanup(self._tmp.cleanup)
        self.tmp = Path(self._tmp.name)

    def write(self, rel: str, content: str = "") -> Path:
        path = self.tmp / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(textwrap.dedent(content), encoding="utf-8")
        return path


# --------------------------------------------------------------------------
# check-no-network
# --------------------------------------------------------------------------

def pkg_id(name: str) -> str:
    return f"path+file:///w/{name}#{name}@0.1.0"


def synthetic_metadata(members: list[str], edges: dict[str, list[str]], declared: dict[str, list[dict]] | None = None) -> dict:
    declared = declared or {}
    names = set(members) | set(edges) | {d for ds in edges.values() for d in ds}
    return {
        "packages": [
            {"id": pkg_id(n), "name": n, "dependencies": declared.get(n, [])} for n in sorted(names)
        ],
        "workspace_members": [pkg_id(m) for m in members],
        "resolve": {
            "nodes": [
                {
                    "id": pkg_id(n),
                    "deps": [
                        {"pkg": pkg_id(d), "dep_kinds": [{"kind": None, "target": None}]}
                        for d in edges.get(n, [])
                    ],
                }
                for n in sorted(names)
            ]
        },
    }


class CheckNoNetwork(TempDirCase):
    def run_meta(self, metadata: dict) -> subprocess.CompletedProcess:
        path = self.tmp / "meta.json"
        path.write_text(json.dumps(metadata), encoding="utf-8")
        return run_tool("check-no-network", "--metadata-json", str(path))

    def test_check_no_network_passes_on_clean_graph(self):
        meta = synthetic_metadata(
            ["proxim-geo", "proxim-decide"],
            {"proxim-decide": ["proxim-geo", "libm", "serde"], "proxim-geo": ["libm"]},
        )
        res = self.run_meta(meta)
        self.assertEqual(res.returncode, 0, res.stdout + res.stderr)
        self.assertIn("OK", res.stdout)

    def test_check_no_network_rejects_direct_denylisted_crate(self):
        meta = synthetic_metadata(["proxim-decide"], {"proxim-decide": ["reqwest"]})
        res = self.run_meta(meta)
        self.assertEqual(res.returncode, 1, res.stdout + res.stderr)
        self.assertIn("reqwest", res.stdout)

    def test_check_no_network_rejects_transitive_denylisted_crate_and_prints_path(self):
        meta = synthetic_metadata(
            ["proxim-pack"],
            {"proxim-pack": ["innocent"], "innocent": ["middle"], "middle": ["mio"]},
        )
        res = self.run_meta(meta)
        self.assertEqual(res.returncode, 1, res.stdout + res.stderr)
        self.assertIn("proxim-pack -> innocent -> middle -> mio", res.stdout)

    def test_check_no_network_rejects_glob_pattern_matches(self):
        # 'rustls-*' and 'hyper*' are glob patterns in the denylist.
        for crate in ("rustls-pemfile", "hyper-util", "tokio-util", "reqwest-middleware"):
            with self.subTest(crate=crate):
                res = self.run_meta(synthetic_metadata(["proxim-sim"], {"proxim-sim": [crate]}))
                self.assertEqual(res.returncode, 1, res.stdout + res.stderr)

    def test_check_no_network_rejects_declared_optional_dependency_even_if_unresolved(self):
        meta = synthetic_metadata(
            ["proxim-events"],
            {"proxim-events": []},
            declared={"proxim-events": [{"name": "ureq", "optional": True, "kind": None}]},
        )
        res = self.run_meta(meta)
        self.assertEqual(res.returncode, 1, res.stdout + res.stderr)
        self.assertIn("ureq", res.stdout)
        self.assertIn("optional", res.stdout)

    def test_check_no_network_does_not_confuse_h3o_with_http3(self):
        # h3o (hex index) is allowed; `h3` (HTTP/3) is denied.
        ok = self.run_meta(synthetic_metadata(["proxim-geo"], {"proxim-geo": ["h3o"]}))
        self.assertEqual(ok.returncode, 0, ok.stdout + ok.stderr)
        bad = self.run_meta(synthetic_metadata(["proxim-geo"], {"proxim-geo": ["h3"]}))
        self.assertEqual(bad.returncode, 1, bad.stdout + bad.stderr)

    def test_check_no_network_empty_denylist_is_a_tool_error_not_a_pass(self):
        deny = self.write("deny.txt", "# nothing\n")
        meta = self.tmp / "meta.json"
        meta.write_text(json.dumps(synthetic_metadata(["a"], {})), encoding="utf-8")
        res = run_tool("check-no-network", "--metadata-json", str(meta), "--denylist", str(deny))
        self.assertEqual(res.returncode, 2, res.stdout + res.stderr)

    def test_check_no_network_missing_manifest_is_a_tool_error_not_a_pass(self):
        res = run_tool("check-no-network", "--manifest", str(self.tmp / "nope" / "Cargo.toml"))
        self.assertEqual(res.returncode, 2, res.stdout + res.stderr)

    # -- end to end with real `cargo metadata` on a temp workspace (path crates only, no network) --

    def make_workspace(self, with_bad_dep: bool) -> Path:
        ws = self.tmp / "ws"
        members = ["engine", "reqwest"] if with_bad_dep else ["engine", "libm"]
        self.write("ws/Cargo.toml", f'[workspace]\nresolver = "2"\nmembers = {json.dumps(members)}\n')
        dep_name = "reqwest" if with_bad_dep else "libm"
        self.write(
            "ws/engine/Cargo.toml",
            f'[package]\nname = "engine"\nversion = "0.1.0"\nedition = "2021"\n\n[dependencies]\n{dep_name} = {{ path = "../{dep_name}" }}\n',
        )
        self.write("ws/engine/src/lib.rs", "pub fn f() {}\n")
        self.write(
            f"ws/{dep_name}/Cargo.toml",
            f'[package]\nname = "{dep_name}"\nversion = "0.1.0"\nedition = "2021"\n',
        )
        self.write(f"ws/{dep_name}/src/lib.rs", "pub fn g() {}\n")
        return ws / "Cargo.toml"

    @unittest.skipUnless(shutil.which("cargo"), "cargo not installed")
    def test_check_no_network_real_cargo_rejects_workspace_depending_on_denylisted_crate(self):
        manifest = self.make_workspace(with_bad_dep=True)
        res = run_tool("check-no-network", "--manifest", str(manifest), "--offline")
        self.assertEqual(res.returncode, 1, res.stdout + res.stderr)
        self.assertIn("reqwest", res.stdout)

    @unittest.skipUnless(shutil.which("cargo"), "cargo not installed")
    def test_check_no_network_real_cargo_accepts_clean_workspace(self):
        manifest = self.make_workspace(with_bad_dep=False)
        res = run_tool("check-no-network", "--manifest", str(manifest), "--offline")
        self.assertEqual(res.returncode, 0, res.stdout + res.stderr)

    @unittest.skipUnless(shutil.which("cargo"), "cargo not installed")
    def test_check_no_network_source_scan_rejects_socket_use(self):
        manifest = self.make_workspace(with_bad_dep=False)
        (manifest.parent / "engine" / "src" / "lib.rs").write_text(
            "use std::net::TcpStream;\npub fn f() {}\n", encoding="utf-8"
        )
        res = run_tool("check-no-network", "--manifest", str(manifest), "--offline")
        self.assertEqual(res.returncode, 1, res.stdout + res.stderr)
        self.assertIn("lib.rs:1", res.stdout)
        skipped = run_tool("check-no-network", "--manifest", str(manifest), "--offline", "--no-source-scan")
        self.assertEqual(skipped.returncode, 0, skipped.stdout + skipped.stderr)


# --------------------------------------------------------------------------
# check-supabase-ref
# --------------------------------------------------------------------------

class CheckSupabaseRef(TempDirCase):
    def lists(self, allowed: str = "") -> list[str]:
        forb = self.write("lists/forbidden.txt", f"# forbidden\n{HEALTHCARE_REF}\n")
        allow = self.write("lists/allowed.txt", f"# allowed\n{allowed}\n")
        return ["--forbidden", str(forb), "--allowed", str(allow)]

    def run_ref(self, *extra: str, allowed: str = "") -> subprocess.CompletedProcess:
        root = self.tmp / "repo"
        root.mkdir(exist_ok=True)
        return run_tool("check-supabase-ref", "--root", str(root), *self.lists(allowed), *extra)

    def test_supabase_ref_real_forbidden_list_contains_the_healthcare_project(self):
        text = (TOOLS / "forbidden-supabase-refs.txt").read_text(encoding="utf-8")
        self.assertIn(HEALTHCARE_REF, text.split())

    def test_supabase_ref_real_allowed_list_is_empty_until_owner_confirms_a_project(self):
        lines = [
            ln.split("#", 1)[0].strip()
            for ln in (TOOLS / "allowed-supabase-refs.txt").read_text(encoding="utf-8").splitlines()
        ]
        self.assertEqual([ln for ln in lines if ln], [])

    def test_supabase_ref_passes_on_config_without_project_id(self):
        self.write("repo/supabase/config.toml", '# no project id\n[api]\nport = 54321\n')
        res = self.run_ref()
        self.assertEqual(res.returncode, 0, res.stdout + res.stderr)

    def test_supabase_ref_rejects_forbidden_ref_as_project_id(self):
        self.write("repo/supabase/config.toml", f'project_id = "{HEALTHCARE_REF}"\n')
        res = self.run_ref()
        self.assertEqual(res.returncode, 1, res.stdout + res.stderr)
        self.assertIn("FORBIDDEN", res.stdout)

    def test_supabase_ref_rejects_forbidden_ref_in_remotes_block(self):
        self.write("repo/supabase/config.toml", f'[remotes.staging]\nproject_id = "{HEALTHCARE_REF}"\n')
        res = self.run_ref()
        self.assertEqual(res.returncode, 1, res.stdout + res.stderr)

    def test_supabase_ref_rejects_forbidden_ref_in_env_file_url(self):
        self.write("repo/apps/console/.env.local", f"NEXT_PUBLIC_SUPABASE_URL=https://{HEALTHCARE_REF}.supabase.co\n")
        res = self.run_ref()
        self.assertEqual(res.returncode, 1, res.stdout + res.stderr)
        self.assertIn(".env.local", res.stdout)

    def test_supabase_ref_rejects_forbidden_ref_in_workflow(self):
        self.write(
            "repo/.github/workflows/deploy.yml",
            f"jobs:\n  db:\n    steps:\n      - run: supabase link --project-ref {HEALTHCARE_REF}\n",
        )
        res = self.run_ref()
        self.assertEqual(res.returncode, 1, res.stdout + res.stderr)
        self.assertIn("deploy.yml", res.stdout)

    def test_supabase_ref_rejects_forbidden_ref_case_insensitively_and_in_temp_link_file(self):
        self.write("repo/supabase/.temp/project-ref", HEALTHCARE_REF.upper() + "\n")
        res = self.run_ref()
        self.assertEqual(res.returncode, 1, res.stdout + res.stderr)

    def test_supabase_ref_rejects_any_unlisted_project_id(self):
        self.write("repo/supabase/config.toml", 'project_id = "proxim-local"\n')
        res = self.run_ref()
        self.assertEqual(res.returncode, 1, res.stdout + res.stderr)
        self.assertIn("allowed-supabase-refs", res.stdout)

    def test_supabase_ref_accepts_a_listed_project_id(self):
        self.write("repo/supabase/config.toml", 'project_id = "proxim-local"\n')
        res = self.run_ref(allowed="proxim-local")
        self.assertEqual(res.returncode, 0, res.stdout + res.stderr)

    def test_supabase_ref_rejects_unlisted_ref_in_env_url_but_accepts_when_listed(self):
        self.write("repo/.env.production", f"SUPABASE_URL=https://{FAKE_REF}.supabase.co\n")
        self.assertEqual(self.run_ref().returncode, 1)
        self.assertEqual(self.run_ref(allowed=FAKE_REF).returncode, 0)

    def test_supabase_ref_ignores_variable_values_and_other_vendors_project_ids(self):
        self.write(
            "repo/.github/workflows/ci.yml",
            "env:\n  SUPABASE_PROJECT_REF: ${{ secrets.SUPABASE_PROJECT_REF }}\n  VERCEL_PROJECT_ID: prj_abcdefghijklmnop\n",
        )
        res = self.run_ref()
        self.assertEqual(res.returncode, 0, res.stdout + res.stderr)

    def test_supabase_ref_runtime_value_forbidden_is_rejected_and_empty_is_skipped(self):
        bad = self.run_ref("--ref", HEALTHCARE_REF)
        self.assertEqual(bad.returncode, 1, bad.stdout + bad.stderr)
        empty = self.run_ref("--ref", "")
        self.assertEqual(empty.returncode, 0, empty.stdout + empty.stderr)
        unlisted = self.run_ref("--ref", FAKE_REF)
        self.assertEqual(unlisted.returncode, 1, unlisted.stdout + unlisted.stderr)

    def test_supabase_ref_does_not_scan_docs_prose(self):
        self.write("repo/docs/PLAN.md", f"never reuse {HEALTHCARE_REF}\n")
        res = self.run_ref()
        self.assertEqual(res.returncode, 0, res.stdout + res.stderr)

    def test_supabase_ref_empty_forbidden_list_is_a_tool_error(self):
        forb = self.write("lists/forbidden.txt", "# nothing\n")
        allow = self.write("lists/allowed.txt", "")
        (self.tmp / "repo").mkdir()
        res = run_tool("check-supabase-ref", "--root", str(self.tmp / "repo"), "--forbidden", str(forb), "--allowed", str(allow))
        self.assertEqual(res.returncode, 2, res.stdout + res.stderr)

    def test_supabase_ref_real_repo_is_clean(self):
        res = run_tool("check-supabase-ref")
        self.assertEqual(res.returncode, 0, res.stdout + res.stderr)


# --------------------------------------------------------------------------
# check-invariants
# --------------------------------------------------------------------------

HEADER = "| ID | Kind | Status | Milestone | Test ID | Guards | Wording |\n|---|---|---|---|---|---|---|\n"


class CheckInvariants(TempDirCase):
    def doc(self, *rows: str) -> Path:
        return self.write("docs/privacy-invariants.md", "# Invariants\n\n" + HEADER + "\n".join(rows) + "\n")

    def run_inv(self, *extra: str, require_ids: bool = False) -> subprocess.CompletedProcess:
        args = ["--root", str(self.tmp), *extra]
        if not require_ids:
            args.append("--no-require-ids")
        return run_tool("check-invariants", *args)

    def test_invariants_enforced_with_existing_test_passes(self):
        self.write("core/tests/consent.rs", "#[test]\nfn consent_missing_is_denied() {}\n")
        self.doc("| I4 | base | enforced | M1 | `core/tests/consent.rs::consent_missing_is_denied` | - | consent fails closed |")
        res = self.run_inv()
        self.assertEqual(res.returncode, 0, res.stdout + res.stderr)

    def test_invariants_enforced_with_missing_test_file_fails(self):
        self.doc("| I4 | base | enforced | M1 | `core/tests/consent.rs::consent_missing_is_denied` | - | x |")
        res = self.run_inv()
        self.assertEqual(res.returncode, 1, res.stdout + res.stderr)
        self.assertIn("does not exist", res.stdout)

    def test_invariants_enforced_with_missing_test_name_fails(self):
        self.write("core/tests/consent.rs", "fn something_else() {}\n")
        self.doc("| I4 | base | enforced | M1 | `core/tests/consent.rs::consent_missing_is_denied` | - | x |")
        res = self.run_inv()
        self.assertEqual(res.returncode, 1, res.stdout + res.stderr)
        self.assertIn("not found", res.stdout)

    def test_invariants_test_name_must_match_whole_word(self):
        self.write("core/tests/consent.rs", "fn consent_missing_is_denied_extra() {}\n")
        self.doc("| I4 | base | enforced | M1 | `core/tests/consent.rs::consent_missing_is_denied` | - | x |")
        self.assertEqual(self.run_inv().returncode, 1)

    def test_invariants_enforced_without_any_test_id_fails(self):
        for empty in ("", "-", "none", "TBD"):
            with self.subTest(test_id=empty):
                self.doc(f"| I4 | base | enforced | M1 | {empty} | - | x |")
                res = self.run_inv()
                self.assertEqual(res.returncode, 1, res.stdout + res.stderr)
                self.assertIn("no Test ID", res.stdout)

    def test_invariants_row_without_status_fails(self):
        self.doc("| I4 | base |  | M1 | `a::b` | - | x |")
        res = self.run_inv()
        self.assertEqual(res.returncode, 1, res.stdout + res.stderr)
        self.assertIn("status", res.stdout)

    def test_invariants_unknown_status_fails(self):
        self.doc("| I4 | base | done | M1 | `a::b` | - | x |")
        self.assertEqual(self.run_inv().returncode, 1)

    def test_invariants_duplicate_id_fails(self):
        self.doc(
            "| I4 | base | planned | M3 | `a/b.rs::t` | - | x |",
            "| I4 | base | planned | M3 | `a/b.rs::t` | - | x |",
        )
        res = self.run_inv()
        self.assertEqual(res.returncode, 1, res.stdout + res.stderr)
        self.assertIn("duplicate", res.stdout)

    def test_invariants_planned_only_warns_and_lists(self):
        self.doc("| I2 | base | planned | M3 | `tools/privacy-lint/test.py::test_lat_field` | - | x |")
        res = self.run_inv()
        self.assertEqual(res.returncode, 0, res.stdout + res.stderr)
        self.assertIn("WARN planned I2", res.stdout)

    def test_invariants_planned_row_whose_test_exists_warns_to_flip(self):
        self.write("core/tests/c.rs", "fn t() {}\n")
        self.doc("| I4 | base | planned | M3 | `core/tests/c.rs::t` | - | x |")
        res = self.run_inv()
        self.assertEqual(res.returncode, 0, res.stdout + res.stderr)
        self.assertIn("flip to 'enforced'", res.stdout)

    def test_invariants_design_rule_guard_matching_fails_and_not_matching_passes(self):
        self.write("core/crates/proxim-events/src/notif_post.rs", "")
        self.doc("| I13 | new | design-rule | Stage 2 | `x/y.rs::t` | core/crates/**/notif*.rs | x |")
        bad = self.run_inv()
        self.assertEqual(bad.returncode, 1, bad.stdout + bad.stderr)
        self.assertIn("guarded path exists", bad.stdout)
        self.doc("| I13 | new | design-rule | Stage 2 | `x/y.rs::t` | core/crates/**/zzz*.rs | x |")
        good = self.run_inv()
        self.assertEqual(good.returncode, 0, good.stdout + good.stderr)

    def test_invariants_tool_reference_needs_tool_and_self_test(self):
        self.doc("| I1 | base | enforced | M0 | `tool:check-no-network` | - | x |")
        self.assertEqual(self.run_inv().returncode, 1)  # neither exists in the temp root
        self.write("tools/check-no-network.py", "")
        self.assertEqual(self.run_inv().returncode, 1)  # no self-test
        self.write("tools/tests/test_tools.py", "# check-no-network tests\n")
        res = self.run_inv()
        self.assertEqual(res.returncode, 0, res.stdout + res.stderr)

    def test_invariants_multiple_test_ids_all_must_exist(self):
        self.write("a/one.rs", "fn t_one() {}\n")
        self.doc("| I1 | base | enforced | M0 | `a/one.rs::t_one`; `a/two.rs::t_two` | - | x |")
        self.assertEqual(self.run_inv().returncode, 1)
        self.write("a/two.rs", "fn t_two() {}\n")
        self.assertEqual(self.run_inv().returncode, 0)

    def test_invariants_missing_required_id_fails_by_default(self):
        self.write("a.rs", "fn t() {}\n")
        self.doc("| I1 | base | enforced | M0 | `a.rs::t` | - | x |")
        res = self.run_inv(require_ids=True)
        self.assertEqual(res.returncode, 1, res.stdout + res.stderr)
        self.assertIn("required invariant I2", res.stdout)

    def test_invariants_no_table_is_a_tool_error(self):
        self.write("docs/privacy-invariants.md", "# nothing here\n")
        res = self.run_inv()
        self.assertEqual(res.returncode, 2, res.stdout + res.stderr)

    def test_invariants_real_document_passes(self):
        res = run_tool("check-invariants")
        self.assertEqual(res.returncode, 0, res.stdout + res.stderr)
        self.assertIn("check-invariants: OK", res.stdout)


if __name__ == "__main__":
    unittest.main(verbosity=2)
