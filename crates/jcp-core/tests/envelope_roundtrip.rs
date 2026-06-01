//! Deterministic invariant (property-style) + integration tests for the
//! `jcp-core` signing envelope.
//!
//! No `proptest` dependency: a tiny inline xorshift64* PRNG generates many
//! pseudo-random subjects/payloads from a fixed seed, so the run is fully
//! deterministic and offline. For every generated envelope we assert the core
//! signing invariants:
//!
//! - sign -> `verify_signature()` always succeeds;
//! - any payload mutation makes `verify_signature()` fail;
//! - `payload_hash` is stable (recomputing it equals the stored value, and a
//!   mutated payload changes it);
//! - the envelope JSON round-trips (serialize -> deserialize -> equal).

use jcp_core::{payload_hash, CoreError, Envelope, LocalSigner};
use serde_json::{json, Value};
use std::str::FromStr;

/// Deterministic xorshift64* PRNG. Inline so the tests need no extra crate and
/// produce the identical stream on every machine/run.
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        // Avoid the all-zero state, which is a fixed point for xorshift.
        Self {
            state: seed ^ 0x9E37_79B9_7F4A_7C15,
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform-ish value in `0..bound` (bound > 0).
    fn below(&mut self, bound: u64) -> u64 {
        self.next_u64() % bound
    }
}

/// A fixed 32-byte signer seed: deterministic, never touches disk or the OS RNG.
const SIGNER_SEED: [u8; 32] = [0x42u8; 32];

/// Build a subject string with three non-empty, slash-free path segments, as
/// `Subject::from_str` requires.
fn random_subject(rng: &mut Rng) -> String {
    let segment = |rng: &mut Rng| -> String {
        let len = 1 + rng.below(8) as usize;
        (0..len)
            .map(|_| {
                // lowercase ascii letters: never empty, never contains '/'.
                let c = b'a' + (rng.below(26) as u8);
                c as char
            })
            .collect()
    };
    format!("{}/{}/{}", segment(rng), segment(rng), segment(rng))
}

/// Build a small but varied JSON payload.
fn random_payload(rng: &mut Rng) -> Value {
    match rng.below(5) {
        0 => json!({}),
        1 => json!({ "n": rng.next_u64() }),
        2 => json!({ "s": format!("v{}", rng.below(1000)), "b": rng.below(2) == 0 }),
        3 => json!({
            "nested": { "a": rng.below(100), "list": [rng.below(10), rng.below(10)] }
        }),
        _ => json!([rng.below(50), rng.below(50), { "k": rng.below(7) }]),
    }
}

/// Mutate a payload so it is guaranteed to differ from the original (and thus
/// produce a different `payload_hash`).
fn mutate_payload(value: &Value, rng: &mut Rng) -> Value {
    let mut object = match value {
        Value::Object(map) => map.clone(),
        _ => serde_json::Map::new(),
    };
    // Insert a uniquely-keyed marker so the result always differs from `value`,
    // regardless of the original shape (object, array, or scalar).
    object.insert(
        format!("__mutation_{}", rng.next_u64()),
        json!(rng.next_u64()),
    );
    Value::Object(object)
}

#[test]
fn envelope_signing_invariants_hold_over_random_inputs() {
    let signer = LocalSigner::from_seed("test:roundtrip", &SIGNER_SEED);
    let mut rng = Rng::new(0xC0FF_EE12_3456_789A);

    for i in 0..256u32 {
        let subject_str = random_subject(&mut rng);
        // The generated subject must parse: sanity-check the generator itself.
        let subject = jcp_core::Subject::from_str(&subject_str)
            .unwrap_or_else(|_| panic!("generated subject must parse: {subject_str}"));

        let payload = random_payload(&mut rng);
        let kind = format!("work.kind.{}", rng.below(16));

        let signed = signer.sign(Envelope::new(subject.clone(), &kind, payload.clone()));

        // Invariant 1: sign -> verify always succeeds.
        signed
            .verify_signature()
            .unwrap_or_else(|err| panic!("iteration {i}: signed envelope must verify: {err:?}"));

        // Invariant 2: payload_hash is stable -- recomputing over the stored
        // payload reproduces exactly the stored hash.
        assert_eq!(
            signed.payload_hash,
            payload_hash(&signed.payload),
            "iteration {i}: payload_hash must be stable",
        );

        // Invariant 3: any payload mutation makes verify fail. We rebuild the
        // envelope around the mutated payload but keep the original signature,
        // so the embedded payload_hash no longer matches the signed content.
        let mutated = mutate_payload(&payload, &mut rng);
        assert_ne!(
            payload_hash(&mutated),
            signed.payload_hash,
            "iteration {i}: mutated payload must change the hash",
        );
        let mut tampered = signed.clone();
        tampered.payload = mutated;
        tampered.payload_hash = payload_hash(&tampered.payload);
        assert_eq!(
            tampered.verify_signature(),
            Err(CoreError::SignatureMismatch),
            "iteration {i}: tampered payload must fail signature verification",
        );

        // Invariant 4: envelope JSON round-trips.
        let serialized = serde_json::to_string(&signed)
            .unwrap_or_else(|err| panic!("iteration {i}: envelope must serialize: {err}"));
        let deserialized: Envelope = serde_json::from_str(&serialized)
            .unwrap_or_else(|err| panic!("iteration {i}: envelope must deserialize: {err}"));
        assert_eq!(
            signed, deserialized,
            "iteration {i}: envelope must round-trip through JSON unchanged",
        );
        // The round-tripped envelope must still verify.
        deserialized.verify_signature().unwrap_or_else(|err| {
            panic!("iteration {i}: round-tripped envelope must verify: {err:?}")
        });
    }
}

#[test]
fn signing_is_deterministic_for_fixed_seed_and_envelope() {
    // Same seed + same envelope -> identical signature, byte for byte.
    let signer = LocalSigner::from_seed("test:determinism", &SIGNER_SEED);
    let envelope = Envelope::new(
        jcp_core::Subject::from_str("tenant/service/entity").unwrap(),
        "work.submit",
        json!({"a": 1, "b": [2, 3]}),
    );
    let one = signer.sign(envelope.clone());
    let two = signer.sign(envelope);
    assert_eq!(one.signature, two.signature);
}
