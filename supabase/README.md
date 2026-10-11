# supabase/

Status: placeholder. M0/M1 build no backend (the core has no networking and no server calls).

- `config.toml` deliberately has no `project_id`. No Proxim Supabase project exists. Creating one needs owner
  confirmation of cost, organisation and region (see `docs/DECISIONS.md`).
- Never reuse the existing healthcare project. `python3 -I tools/check-supabase-ref.py` fails CI if its ref appears
  in `supabase/config.toml`, `.env*` or workflows, or if any project id is configured that is not listed in
  `tools/allowed-supabase-refs.txt`.
- Migrations live in `supabase/migrations/` as files and are never applied by hand to a remote project.
- Every table holding sender or user data enables RLS in the migration that creates it.
