# core/testdata

Synthetic data only (invariant I2): no real traces, no real places of a person.

- `golden/<kind>/`  golden vectors (engine-spec 9), kinds decide detect path geo h3 trig. Filled in the next phase.
- `scenarios/`      replay scenarios for proxim-sim.
- `routes/`         GPX/GeoJSON fixture routes (synthetic).
- `keys/`           TEST-ONLY signing keys for pack tests. Never a production key (see deny/gitleaks rules).
