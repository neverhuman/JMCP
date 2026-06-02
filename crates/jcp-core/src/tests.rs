use super::*;
use serde_json::json;
use std::str::FromStr;

const SEED_A: [u8; SEED_LEN] = [7u8; SEED_LEN];
const SEED_B: [u8; SEED_LEN] = [9u8; SEED_LEN];

#[test]
fn validates_subject_and_hash() {
    let subject = Subject::from_str("tenant/service/entity").unwrap();
    let mut envelope = Envelope::new(subject, "work.submit", json!({"a": 1}));
    envelope.validate().unwrap();
    envelope.payload = json!({"a": 2});
    assert_eq!(envelope.validate(), Err(CoreError::PayloadHashMismatch));
}

#[test]
fn local_signature_round_trips() {
    let key_path = std::env::temp_dir().join(format!("jmcp-test-{}.key", Uuid::new_v4()));
    let signer = LocalSigner::load_or_create(&key_path).unwrap();
    let envelope = signer.sign(Envelope::new("t/s/e".parse().unwrap(), "x", json!({})));
    envelope.verify_local_signature(&signer).unwrap();
    let _ = fs::remove_file(key_path);
}

#[test]
fn local_signature_rejects_tampering() {
    let key_path = std::env::temp_dir().join(format!("jmcp-test-{}.key", Uuid::new_v4()));
    let signer = LocalSigner::load_or_create(&key_path).unwrap();
    let mut envelope = signer.sign(Envelope::new("t/s/e".parse().unwrap(), "x", json!({})));
    envelope.kind = "changed".to_owned();
    assert_eq!(
        envelope.verify_local_signature(&signer),
        Err(CoreError::SignatureMismatch)
    );
    let _ = fs::remove_file(key_path);
}

#[test]
fn ed25519_sign_verify_round_trip() {
    let signer = LocalSigner::from_seed("test:a", &SEED_A);
    let envelope = signer.sign(Envelope::new(
        "t/s/e".parse().unwrap(),
        "x",
        json!({"a": 1}),
    ));
    assert_eq!(envelope.signature.as_ref().unwrap().alg, "ed25519");
    envelope.verify_signature().unwrap();
}

#[test]
fn ed25519_is_deterministic_from_fixed_seed() {
    let signer = LocalSigner::from_seed("test:a", &SEED_A);
    let env = Envelope::new("t/s/e".parse().unwrap(), "x", json!({"a": 1}));
    let one = signer.sign(env.clone());
    let two = signer.sign(env);
    assert_eq!(one.signature, two.signature);
}

#[test]
fn ed25519_rejects_tampered_payload() {
    let signer = LocalSigner::from_seed("test:a", &SEED_A);
    let mut envelope = signer.sign(Envelope::new("t/s/e".parse().unwrap(), "x", json!({})));
    envelope.kind = "changed".to_owned();
    assert_eq!(
        envelope.verify_signature(),
        Err(CoreError::SignatureMismatch)
    );
}

#[test]
fn ed25519_rejects_wrong_key() {
    let signer = LocalSigner::from_seed("test:a", &SEED_A);
    let other = LocalSigner::from_seed("test:b", &SEED_B);
    let mut envelope = signer.sign(Envelope::new("t/s/e".parse().unwrap(), "x", json!({})));
    if let Some(sig) = envelope.signature.as_mut() {
        sig.public_key = hex::encode(other.verifying_key().as_bytes());
    }
    assert_eq!(
        envelope.verify_signature(),
        Err(CoreError::SignatureMismatch)
    );
}

#[test]
fn ed25519_missing_signature_errors() {
    let envelope = Envelope::new("t/s/e".parse().unwrap(), "x", json!({}));
    assert_eq!(
        envelope.verify_signature(),
        Err(CoreError::MissingSignature)
    );
}
