# Signed pack format (M2)

Status: M2 in progress, 2026-10-17. Normative for `core/crates/proxim-pack`. Sources: PLAN 5.2 (distribution),
PLAN-ADDENDUM-02 4.3 and 12 (envelope v2), invariants I10 and I17. Where this file fixes a detail the plan left
open it says so under "Lead decisions"; the owner may revisit any of them.

## 1. Trust chain

```
pinned root keys (shipped in the SDK)
  -> keyset            signed by a root; lists online keys and revoked kids
       -> manifest     signed by an online key; one per H3 res-5 cell: seq, expiry, tier hashes
            -> pack    one file per tier; its SHA-256 must equal the manifest tier hash
                 -> entry   one signpost family; signed by an online key (envelope v2)
```

Every check is a pure function of its inputs: the verifier takes `now_ms` as an argument and has no clock,
network or storage. Rollback state (highest seq seen) is passed in and handed back, so the SDK shell persists it.

## 2. Primitives

- Signature: ECDSA P-256 with SHA-256 (ES256), 64 bytes raw `r || s`, big-endian. **Low-S only**: a signature
  whose `s` is above n/2 is rejected, so each message has one valid encoding. The signer normalises (KMS returns
  DER and either S).
- Public key: SEC1 uncompressed point, 65 bytes, written as 130 lowercase hex characters in JSON.
- Key id (`kid`): 1 to 64 bytes of `[a-z0-9._-]`.
- Digest: SHA-256. Hex in JSON is lowercase, 64 characters.
- Integers on the wire are big-endian. JSON payloads are canonical JSON (engine-spec 3.2: sorted keys, no
  whitespace, integers only) and must be byte-for-byte canonical, or they are rejected.

## 3. Signed document (keyset, manifest)

```
doc        = "PXSD" | u8 version=1 | u8 kind | u8 kid_len | kid | u32 payload_len | payload | sig[64]
signed_msg = "PROXIM-DOC-V1" 0x00 | u8 kind | u8 kid_len | kid | u32 payload_len | payload
kind       = 1 keyset, 2 manifest
```

The kind byte is inside the signed message, so a manifest can never be replayed as a keyset or the other way
round. Trailing bytes, an unknown version or kind, or a payload over 1 MiB are rejected.

### 3.1 Keyset (kind 1), signed by a pinned root

```json
{"expires_ms":..,"issued_ms":..,"keys":[{"kid":"..","not_after_ms":..,"not_before_ms":..,"pub":"04.."}],"revoked_kids":[".."],"seq":..,"v":1}
```

- `kid` of the document must be a pinned root; online keys may not sign keysets.
- `keys` 1 to 32, kids unique; `revoked_kids` 0 to 256. A kid may not appear in both.
- Valid at `now_ms` only if `issued_ms <= now_ms <= expires_ms`.
- Rollback: a keyset with a lower `seq` than the last one accepted is rejected; the same `seq` is accepted only
  with identical bytes.
- An online key signs only while `not_before_ms <= now_ms <= not_after_ms` and its kid is not revoked.

### 3.2 Manifest (kind 2), signed by an online key

```json
{"built_ms":..,"cell":..,"expires_ms":..,"seq":..,"tiers":[{"bytes":..,"sha256":"..","tier":0}],"v":1}
```

- `cell` is the H3 res-5 index as an integer and must equal the cell the device asked for.
- `tiers` 1 to 3 records; the record at position i is tier i (so tier 0 is always present); `bytes` at most 8 MiB.
- Valid only if `built_ms <= now_ms <= expires_ms` and `expires_ms - built_ms <= 72 h` (PLAN 5.2 hard TTL).
- Rollback per cell, as for the keyset: lower `seq` rejected, equal `seq` accepted only with the same manifest
  digest.

## 4. Pack (one tier file)

```
pack  = "PXPK" | u8 version=1 | u64 cell | u64 seq | u8 tier | u16 count | count x (u32 len | entry)
```

The pack's SHA-256 and byte length must equal the manifest's tier record, and `cell`, `seq` and `tier` must
match the manifest. A framing error rejects the whole pack. A bad entry is dropped on its own, with a reason,
and the rest of the pack stays usable (A02 4.3: "any failure drops the whole entry"). At most 4,096 entries.

## 5. Entry (envelope v2, A02 4.3)

```
entry      = u8 version=2 | u8 kid_len | kid | u16 core_len | core | u8 n | u8 ship_mask
             | for i in 0..n: (bit i of ship_mask set ? u16 body_len | body : d_i[32])
             | sig[64]
signed_msg = "PROXIM-ENV-V2" 0x00 | u16 core_len | core | u8 n | d_0 .. d_(n-1)
d_i        = SHA-256("PROXIM-CREATIVE-V2" 0x00 | body_i)
vid_i      = d_i[0..4]
```

- `core` is canonical JSON `{"expires_ms":..,"signpost":{..}}`; `signpost` is the engine's `Signpost`
  (engine-spec 3). Because the whole core is signed, every field the kernel reads (geometry, event,
  sensitivity, category, validity window, rank) is covered (I17). A reserved field present in the core makes it
  non-canonical and the entry is dropped.
- `body_i` is canonical JSON `{"body":..,"glanceable":..,"headline":..}`: headline 1 to 40 characters, body 0 to
  90 characters, glanceable 1 to 7 words. Characters are Unicode scalar values.
- `n` is 1 to 8. `ship_mask` must have bit 0 set (creative 0 always ships) and no bit at or above `n`. The mask is
  unsigned and can only withdraw creatives (the signature still covers every digest).
- The device recomputes the digest of every shipped body, rebuilds `signed_msg` with the digests in index order,
  requires the n `vid`s to be pairwise distinct, and verifies the signature with the key named by `kid` from the
  verified keyset. A 4-byte `vid` is never evidence of authenticity (I10).
- The entry is usable only if `now_ms <= expires_ms` and `expires_ms <= signpost.valid_until_ms`.
- Duplicate signpost `(id, rev)` within one pack: both entries are dropped.

## 6. Error codes

| Code | Meaning |
|---|---|
| `P_MALFORMED` | Framing, length, version, canonical-JSON or field-range error |
| `P_LIMIT` | A size or count bound exceeded |
| `P_UNKNOWN_KID` | Key id not in the pinned roots (keyset) or the keyset (manifest, entry) |
| `P_KID_REVOKED` | Key id on the verified keyset's revoked list |
| `P_KEY_NOT_VALID` | Online key outside its `not_before_ms` / `not_after_ms` window |
| `P_BAD_SIGNATURE` | Signature does not verify, or is high-S |
| `P_EXPIRED` | Document or entry not valid at `now_ms` |
| `P_ROLLBACK` | `seq` lower than the last accepted |
| `P_SEQ_CONFLICT` | Same `seq` as the last accepted, different content |
| `P_MISMATCH` | Cell, seq, tier, length or digest differs from what the parent document says |
| `P_DUPLICATE` | Two creatives with the same `vid`, or two entries for one signpost revision |

## 7. Lead decisions (owner may revisit)

1. **Binary signed documents instead of JWS.** PLAN section 2 named JWS ES256; Addendum 02 already moved the
   entry to a binary envelope. One binary framing for everything means one parser to fuzz and no base64. The
   signature algorithm (ES256, P-256) is unchanged, so KMS and the platform keystores still apply.
2. **Kid revocation lives in the root-signed keyset.** A compromised online key cannot then sign a list that
   leaves itself out. Signpost-, sender- and variant-scope revocation is I20 and lands in M3.
3. **Low-S signatures only**, so no signature is malleable.
4. **A bad entry drops alone; a bad pack, manifest or keyset rejects the whole document.**

## 8. Not in this slice

Signpost-scope revocation and class windows (I20, M3); the TypeScript compiler and its byte-identical build
test; KMS signing; delta packs; tier size measurement with real copy; FFI and WASM bindings for the verifier.
