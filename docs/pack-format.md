# Signed packs: wire format (M2)

Normative for `core/crates/proxim-pack` (the Rust verifier every phone runs) and `packages/pack` (the TypeScript compiler and second verifier). Sources: PLAN 3, 5.2 and M2; Addendum 01 I17, N10, 11.3 and A16; Addendum 02 4.3 and 4.4. Where those leave a byte layout open, this page decides it.

## 1. What the phone receives

| Object | Route | Signed by | Format |
|---|---|---|---|
| Key set | `GET /m/v1/keyset` | a pinned **root** key | compact JWS, `typ` `proxim-keyset+jws` |
| Revocation list | `GET /m/v1/revocations` | a **manifest** key | compact JWS, `typ` `proxim-revocations+jws` |
| Tile manifest | `GET /t/v1/{r5}/manifest` | a **manifest** key | compact JWS, `typ` `proxim-manifest+jws` |
| Pack | `GET /t/v1/{r5}/{sha256}.pack` | each entry by an **approve** key; the pack bytes by the manifest's hash | binary, section 4 |

The phone pins the root public keys in the SDK build. Everything else is trusted only through that chain: root → key set → approve and manifest keys → manifests and entries.

## 2. Common rules

- **Integers** in binary structures are unsigned big-endian. Times are Unix milliseconds (`*_ms`, i64).
- **JSON** payloads are canonical JSON (engine spec 3.2), parsed by the strict parser and required to be byte-for-byte canonical. Unknown fields are rejected.
- **Base64url** is RFC 4648 section 5 without padding. Non-zero trailing bits, padding or any other character is rejected.
- **kid:** 1 to 32 characters from `a-z 0-9 -`. Public keys are SEC1 uncompressed P-256 points (65 bytes, `0x04 || X || Y`), base64url in JSON.
- **Signatures:** ES256 (ECDSA P-256 with SHA-256), 64 bytes `r || s`. High-S is accepted, as KMS signers do not normalise. Integrity rests on hashes, not on signature bytes.
- **Fail closed:** any failure rejects the object. Section 6 lists which failures drop one entry and which reject a whole pack.

### 2.1 Compact JWS

`b64u(header) "." b64u(payload) "." b64u(sig)`. The header must be exactly the canonical JSON `{"alg":"ES256","kid":<kid>,"typ":<typ>}` with no other member. `typ` must be the type the caller asked for, so a manifest can never be accepted as a key set. The signature covers the ASCII bytes `b64u(header) "." b64u(payload)`. Total size is at most 256 KiB.

## 3. JSON documents

### 3.1 Key set (`proxim-keyset/1`)

```
{"format":"proxim-keyset/1","version":u64,"issued_at_ms":i64,"expires_at_ms":i64,
 "keys":[{"kid","purpose":"approve"|"manifest","pub":b64u,"not_before_ms":i64,"not_after_ms":i64}],
 "revoked_kids":[kid]}
```

- Signed by a root kid the phone pins.
- **Accept only if all of these hold:**
  - `issued_at_ms <= now < expires_at_ms`;
  - `version` is at least the stored version;
  - an equal version has the same payload hash as the stored copy;
  - kids are unique;
  - no key reuses a root kid;
  - a revoked kid is not also listed as a key;
  - there are at most 64 keys.
- A key is usable at `now` if `not_before_ms <= now < not_after_ms`, its kid is not in `revoked_kids`, and its purpose matches the use.
- Errors:
  - expired → `PACK_EXPIRED`;
  - a lower version, or an equal version with different bytes → `SEQ_ROLLBACK`;
  - anything else → `SIGNATURE_INVALID`.

### 3.2 Revocation list (`proxim-revocations/1`)

```
{"format":"proxim-revocations/1","seq":u64,"issued_at_ms":i64,"expires_at_ms":i64,
 "entries":[{"scope":"variant","id","rev":u32,"vid":"8 hex"} | {"scope":"signpost_rev","id","rev":u32}
          | {"scope":"signpost","id"} | {"scope":"sender_org","sender"} | {"scope":"kid","kid"}]}
```

- The scopes are Addendum 01 I20 plus the variant scope of Addendum 02 9.2.
- Signed by a usable manifest key.
- Rollback rule: `seq` is at least the stored seq, and an equal seq needs equal bytes.
- At most 65,536 entries.
- Freshness windows (W_service and others) are applied by the SDK, not here.

### 3.3 Tile manifest (`proxim-manifest/1`)

```
{"format":"proxim-manifest/1","cell":"<H3 index, 15 lowercase hex>","seq":u64,"pack_kind":"hosted_tile",
 "issued_at_ms":i64,"expires_at_ms":i64,"tiers":[{"tier":0..2,"bytes":u32,"sha256":"64 hex"}]}
```

- Signed by a usable manifest key.
- **Accept only if all of these hold:**
  - `cell` is the cell the phone asked for;
  - `pack_kind` is `hosted_tile` (`owner_bundle` is reserved until M15);
  - `issued_at_ms <= now < expires_at_ms`;
  - tiers are unique and in ascending order, with 1 to 3 of them;
  - `bytes` is at most 4 MiB.
- Rollback rule, per cell: `seq` is at least the stored seq, and an equal seq needs equal bytes.

## 4. Pack container

```
offset  size  field
0       4     magic "PXPK"
4       1     format_version = 1
5       1     pack_kind: 1 = hosted_tile (2 = owner_bundle, reserved: rejected)
6       1     entry_auth: 1 = ES256 per-entry signature table (any other value: rejected)
7       1     tier (0..2)
8       8     cell (H3 index, u64)
16      8     seq (u64)
24      1     n_kids (1..16), then per kid: u8 len (1..32) + kid bytes
..      4     n_entries (0..4096)
..      4     stream_len, then stream_len bytes: raw DEFLATE (RFC 1951) of the entry stream
..            signature table: n_entries x (u8 kid_index + 64 B r||s)
              end of file: no trailing byte
```

- The **signature table sits outside the compressed stream**, as Addendum 01 N10 requires. Each row costs 65 bytes.
- **Decompressed entry stream:** n_entries records, each `u32 len (1..65,536)` followed by the entry bytes. The stream holds at most 4 MiB decompressed, and no byte may follow the last record.

### 4.1 Entry (envelope v2, Addendum 02 4.3)

```
u16 core_len (1..16,384), core   canonical JSON of the Signpost (engine spec 3), including "pack_kind"
u8  n (1..8)                     declared creatives (v1 compiler: n <= 6)
u8  ship_mask                    bit i = creative i shipped; bit 0 set; bits >= n clear
for i in 0..n:  shipped -> u16 body_len (1..1,024) + body
                else    -> 32-byte digest d_i
```

- **Creative digest:** `d_i = SHA-256("PROXIM-CREATIVE-V2" 0x00 || body_i)`.
- **Variant id:** `vid_i` is the first 4 bytes of `d_i`.
- **Signed message:** `"PROXIM-ENV-V2" 0x00 || u16(core_len) || core || u8(n) || d_0 .. d_(n-1)`. The signature is the matching signature-table row, made with that row's kid.
- `ship_mask` is not signed, and it may only withhold creatives. The manifest's pack hash covers it.
- The v1 compiler ships every creative: a build fails rather than ship fewer than were declared. The verifier already accepts masks.

The core must contain `"pack_kind":"hosted_tile"`, so signed content cannot be moved into another pipe. It may contain `"live_at_ms":i64`; before that time the entry is refused with `NOT_YET_LIVE`. The rest of the Signpost is checked again by `decide` (expiry, triggers, consent and so on).

## 5. Opening a pack

Inputs:

- the pack bytes;
- an accepted manifest and the tier asked for;
- the accepted key set;
- the accepted revocation list (or none);
- `now`.

**Whole-pack checks.** If any of these fails, the pack is rejected with `SIGNATURE_INVALID`, because the pack is corrupt or mis-built:

1. The SHA-256 and length of the bytes equal the manifest's entry for that tier.
2. The magic, version, `pack_kind`, `entry_auth`, tier, cell and seq equal the manifest's values.
3. The kid table and counts are well formed; the stream inflates within its limits; every record parses; there is no trailing byte.
4. No two entries share a signpost id.

**Per-entry checks**, in this order. A failure drops only that entry, with the code shown:

| Step | Check | Code on failure |
|---|---|---|
| a | The entry parses; `ship_mask` is valid; there are no duplicate `vid`s. | `SIGNATURE_INVALID` |
| b | The kid is a usable **approve** key. | `KID_REVOKED` if the kid is listed but revoked, outside its window or of another purpose; otherwise `SIGNATURE_INVALID` |
| c | The ES256 signature verifies over the signed message. | `SIGNATURE_INVALID` |
| d | The core is canonical JSON and a valid Signpost; `pack_kind` matches; `live_at_ms`, if present, is an integer. | `SIGNATURE_INVALID` |
| e | The revocation list does not cover it, by scope: kid, sender_org, signpost, signpost and revision. | `REVOKED` |
| f | `now >= live_at_ms`. | `NOT_YET_LIVE` |

- Variant-scope revocations do not drop the entry. They withdraw the matching creative, and only that one. If the base creative (0) is revoked, the whole entry is dropped with `REVOKED`.
- The result lists:
  - the usable entries, each with its Signpost, the shipped creatives with their vids, and the kid;
  - the dropped entries, each with its index, id (when known) and code.

## 6. Splicing properties (Addendum 01 A16)

| Property | Held by |
|---|---|
| **S1 approved:** every rendered entry carries an approve-key signature | Steps b and c |
| **S2 unaltered:** no signed field or creative body can change | Envelope v2 signed message; the full 32-byte digests |
| **S3 not reused:** content cannot move to another cell, tier, kind or sequence | Header equals manifest; `pack_kind` inside the signed core; the manifest hash binds the bytes |
| **S4 nothing removed:** no entry, suppression zone or signature row can be dropped | The manifest's SHA-256 over the whole pack |
| **S5 no rollback:** an older pack, key set or list cannot be replayed | Seq and version guards per cell, and an equal seq requires equal bytes |

## 7. Determinism

The compiler sorts entries by `rank_milli` descending (default 1000), then by id ascending. It writes the kid table in first-use order, and compresses with zlib level 9 raw DEFLATE (strategy default, window 15, memLevel 8). The same snapshot therefore gives byte-identical packs and the same hash.

Entry signatures come from the snapshot, signed once at approval, so they never change at compile time. Manifest and list JWS signatures may vary between builds; only their payloads need to be deterministic.

## 8. Limits and sizes

- **Tier-0 budget:** at most 350,000 bytes including the header, kid table and signature table (Addendum 01 N10). The compiler fails the build rather than cut entries silently.
- **Measured (2026-10-16, `packages/pack/test/size.test.ts`):** 500 synthetic families, 60% circles and 40% 12-vertex route corridors, 65-byte signature rows.

  | Wordings per family | Word-list text | Random text (worst case) |
  |---|---|---|
  | 1 | 74,902 B | 88,999 B |
  | 4 | 93,980 B | 141,821 B |
  | 6 | 105,044 B | 175,738 B |

  Even the worst case uses about half the budget, so the 350,000-byte budget stands. Addendum 02's planning figure (F = 515 + 110N bytes per family) is deliberately pessimistic. Re-measure with real copy and real AI wordings at M7.
- **Opening time (host baseline):** a 500-entry pack opens in about 104 ms on the Linux build host (release build), about 0.2 ms per entry, almost all of it ES256 verification. iPhone and Android numbers come with the device runs (M8, M10).
- **Verifier bounds:**
  - 4 MiB of pack bytes;
  - 4 MiB of inflated stream;
  - 4,096 entries;
  - 16 kids.

## 9. Test keys

The keys under `core/testdata/keys/` are derived from fixed public seeds. They exist only for tests and golden vectors and must never be trusted by a release build. The SDK pins production roots separately (M8 and M10).
