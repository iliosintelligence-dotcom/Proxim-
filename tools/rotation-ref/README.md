# Rotation reference kernel (OI-ROT-1)

Normative Python reference for wording rotation (policy H, `select_variant`), the pending-decision window and the envelope v2 verifier (Addendum 02, 4.3, 5.1 and 5.3). Every other implementation must reproduce its vectors bit for bit. The first is the Rust port in `core/crates/proxim-decide/src/rotation.rs` and `core/crates/proxim-pack`.

## Files

| File | What it is |
|---|---|
| `refkernel.py` | **Normative.** select, commit, dismiss, ledger and pending window. Integer-only; no clock or randomness. |
| `envelope.py` | **Normative.** Envelope v2 entries: canonical creative bytes, digests, signed message, verify. Needs `cryptography`. |
| `kernel2.py`, `crosscheck.py` | A second, independent Python kernel and the cross-check: 0 mismatches over 81,531 steps. |
| `gen_golden.py` | Wrote `golden_v2.json` once on 2026-10-10. **Never run again.** The vectors are frozen and the ECDSA signatures in them used random nonces. |
| `verify_golden.py` | Python gate: the reference reproduces the committed `golden_v2.json`. The only change from the working copy is the path to the committed file. |
| `gen_newcount.py` | Writes the supplement `golden_v2_newcount.json`, the one-wording and revoked-to-one `new_count` cases that 5.1 requires. It does a dry run unless `PROXIM_BLESS=1` is set. |
| `vid_fixture.json`, `vidforge.c` | Two bodies sharing a 4-byte vid (a second preimage found in 16.7 s on 4 cores), and the search program that found them. |

## Provenance

`golden_v2.json` and the Python files were written by the rotation specialist on 2026-10-10 as a scratchpad working copy. They were committed unchanged on 2026-10-17, with SHA-256 `3a57baae90196f27299c85e419848ed187213cbbf4482f509459de99b216e6b5`, which `proxim-decide/tests/rotation_golden.rs` pins.

The working copy had the envelope entry layout this repository now uses (pack-format 4.1): withheld digests, then shipped bodies, with the signature carried in the pack's signature table.

## Run

```
python3 -I tools/rotation-ref/verify_golden.py      # Python gate
python3 -I tools/rotation-ref/crosscheck.py         # two Python kernels agree
python3 -I tools/rotation-ref/gen_newcount.py       # supplement is current
cd core && cargo test -p proxim-decide --test rotation_golden && cargo test -p proxim-pack --test rotation_envelope
```
