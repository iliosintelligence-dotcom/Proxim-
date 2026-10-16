# scenarios

Replay scenarios for the proxim-sim scenario simulator: one JSON file per scenario, with format,
noise model, oracle and results in `docs/simulator.md`. All geometry is synthetic, in metres
relative to a synthetic origin (invariant I2). `fixtures/` holds verbatim signpost files that
scenarios pin by SHA-256. Run them with
`cargo test -p proxim-sim --test scenarios` or `proxim-sim run testdata/scenarios`.
