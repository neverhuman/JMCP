use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature as Ed25519Signature, Signer, SigningKey, VerifyingKey};
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs, io,
    path::{Path, PathBuf},
    str::FromStr,
};
use thiserror::Error;
use uuid::Uuid;

const SEED_LEN: usize = 32;
pub const SIGNATURE_ALG: &str = "ed25519";

pub const JCP_VERSION: &str = "1.0.0";
pub const JPCM_PROFILE: &str = "jpcm/1.0.0";

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CoreError {
    #[error("unsupported protocol version {0}")]
    UnsupportedVersion(String),
    #[error("invalid subject")]
    InvalidSubject,
    #[error("payload hash mismatch")]
    PayloadHashMismatch,
    #[error("missing signature")]
    MissingSignature,
    #[error("signature mismatch")]
    SignatureMismatch,
    #[error("invalid signature encoding")]
    InvalidSignature,
    #[error("unsupported signature algorithm {0}")]
    UnsupportedSignatureAlg(String),
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Subject {
    pub tenant: String,
    pub service: String,
    pub entity: String,
}

impl FromStr for Subject {
    type Err = CoreError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = input.split('/').collect();
        if parts.len() != 3 || parts.iter().any(|part| part.is_empty()) {
            return Err(CoreError::InvalidSubject);
        }
        Ok(Self {
            tenant: parts[0].to_owned(),
            service: parts[1].to_owned(),
            entity: parts[2].to_owned(),
        })
    }
}

impl Subject {
    pub fn as_wire(&self) -> String {
        format!("{}/{}/{}", self.tenant, self.service, self.entity)
    }
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct Envelope {
    pub jcp_version: String,
    pub transport_profile: String,
    pub id: Uuid,
    pub subject: String,
    pub issued_at: DateTime<Utc>,
    pub kind: String,
    pub payload_hash: String,
    pub payload: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<Signature>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct Signature {
    pub alg: String,
    pub key_id: String,
    pub value: String,
    #[serde(default)]
    pub public_key: String,
}

impl Envelope {
    pub fn new(subject: Subject, kind: impl Into<String>, payload: Value) -> Self {
        let payload_hash = payload_hash(&payload);
        Self {
            jcp_version: JCP_VERSION.to_owned(),
            transport_profile: JPCM_PROFILE.to_owned(),
            id: Uuid::new_v4(),
            subject: subject.as_wire(),
            issued_at: Utc::now(),
            kind: kind.into(),
            payload_hash,
            payload,
            signature: None,
        }
    }

    pub fn validate(&self) -> Result<(), CoreError> {
        if self.jcp_version != JCP_VERSION {
            return Err(CoreError::UnsupportedVersion(self.jcp_version.clone()));
        }
        Subject::from_str(&self.subject)?;
        if self.payload_hash != payload_hash(&self.payload) {
            return Err(CoreError::PayloadHashMismatch);
        }
        Ok(())
    }

    pub fn verify_local_signature(&self, signer: &LocalSigner) -> Result<(), CoreError> {
        let signature = self.signature.as_ref().ok_or(CoreError::MissingSignature)?;
        let expected = signer.signature_for(self);
        if signature == &expected {
            Ok(())
        } else {
            Err(CoreError::SignatureMismatch)
        }
    }

    /// Verify the envelope against the public key embedded in its signature.
    ///
    /// This is publicly verifiable and requires no secret material, which is the
    /// real win for multi-producer logs: any reader can confirm authenticity.
    pub fn verify_signature(&self) -> Result<(), CoreError> {
        let signature = self.signature.as_ref().ok_or(CoreError::MissingSignature)?;
        if signature.alg != SIGNATURE_ALG {
            return Err(CoreError::UnsupportedSignatureAlg(signature.alg.clone()));
        }
        let verifying_key = decode_verifying_key(&signature.public_key)?;
        let sig = decode_signature(&signature.value)?;
        verifying_key
            .verify_strict(signing_input(self).as_bytes(), &sig)
            .map_err(|_| CoreError::SignatureMismatch)
    }
}

#[derive(Clone)]
pub struct LocalSigner {
    key_id: String,
    signing_key: SigningKey,
}

impl std::fmt::Debug for LocalSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never expose secret seed material in debug output.
        f.debug_struct("LocalSigner")
            .field("key_id", &self.key_id)
            .field("public_key", &hex::encode(self.verifying_key().as_bytes()))
            .finish_non_exhaustive()
    }
}

impl LocalSigner {
    pub fn load_or_create_default() -> io::Result<Self> {
        let path = default_key_path()?;
        Self::load_or_create(path)
    }

    pub fn load_or_create(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut seed = [0u8; SEED_LEN];
            getrandom::getrandom(&mut seed)
                .map_err(|err| io::Error::other(format!("rng failure: {err}")))?;
            let hex_seed = hex::encode(seed);
            // Establish the key file atomically and first-writer-wins, so every
            // reader sees a single stable seed. Parallel tests / multiple
            // processes must never see a partially written file, an
            // `AlreadyExists` collision, or a seed that flips mid-run (which
            // would break symmetric `verify_local_signature`). Write a
            // uniquely-named temp, then hard-link it into place; if the
            // destination already exists a peer won, so keep theirs. Always
            // clean up the temp.
            let tmp = match path.file_name() {
                Some(name) => path.with_file_name(format!(
                    ".{}.tmp.{}",
                    name.to_string_lossy(),
                    &hex_seed[..16]
                )),
                None => path.with_extension(format!("tmp.{}", &hex_seed[..16])),
            };
            write_secret(&tmp, hex_seed.as_bytes())?;
            let linked = fs::hard_link(&tmp, path);
            let _ = fs::remove_file(&tmp);
            if let Err(err) = linked {
                if err.kind() != io::ErrorKind::AlreadyExists && !path.exists() {
                    return Err(err);
                }
            }
        }

        let raw = fs::read(path)?;
        let hex_str: Vec<u8> = raw
            .into_iter()
            .filter(|byte| !byte.is_ascii_whitespace())
            .collect();
        let seed_bytes =
            hex::decode(&hex_str).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        let seed: [u8; SEED_LEN] = seed_bytes.as_slice().try_into().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "key file does not contain a 32-byte seed",
            )
        })?;
        Ok(Self::from_seed(key_id_from_seed(&seed), &seed))
    }

    /// Construct a signer from a fixed 32-byte seed. Primarily for deterministic
    /// tests that must not depend on disk or RNG.
    pub fn from_seed(key_id: impl Into<String>, seed: &[u8; SEED_LEN]) -> Self {
        Self {
            key_id: key_id.into(),
            signing_key: SigningKey::from_bytes(seed),
        }
    }

    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    pub fn sign(&self, mut envelope: Envelope) -> Envelope {
        envelope.signature = Some(self.signature_for(&envelope));
        envelope
    }

    fn signature_for(&self, envelope: &Envelope) -> Signature {
        let sig: Ed25519Signature = self.signing_key.sign(signing_input(envelope).as_bytes());
        Signature {
            alg: SIGNATURE_ALG.to_owned(),
            key_id: self.key_id.clone(),
            value: hex::encode(sig.to_bytes()),
            public_key: hex::encode(self.verifying_key().as_bytes()),
        }
    }
}

fn key_id_from_seed(seed: &[u8; SEED_LEN]) -> String {
    let public = SigningKey::from_bytes(seed).verifying_key();
    let digest = hex::encode(Sha256::digest(public.as_bytes()));
    format!("local:{}", &digest[..16])
}

fn decode_verifying_key(hex_str: &str) -> Result<VerifyingKey, CoreError> {
    let bytes = hex::decode(hex_str).map_err(|_| CoreError::InvalidSignature)?;
    let bytes: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| CoreError::InvalidSignature)?;
    VerifyingKey::from_bytes(&bytes).map_err(|_| CoreError::InvalidSignature)
}

fn decode_signature(hex_str: &str) -> Result<Ed25519Signature, CoreError> {
    let bytes = hex::decode(hex_str).map_err(|_| CoreError::InvalidSignature)?;
    let bytes: [u8; 64] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| CoreError::InvalidSignature)?;
    Ok(Ed25519Signature::from_bytes(&bytes))
}

fn default_key_path() -> io::Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is not set"))?;
    Ok(home
        .join(".local")
        .join("share")
        .join("jmcp")
        .join("keys")
        .join("local.key"))
}

fn signing_input(envelope: &Envelope) -> String {
    format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n",
        envelope.jcp_version,
        envelope.transport_profile,
        envelope.id,
        envelope.subject,
        envelope.issued_at.to_rfc3339(),
        envelope.kind,
        envelope.payload_hash
    )
}

#[cfg(unix)]
fn write_secret(path: &Path, contents: &[u8]) -> io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(contents)
}

#[cfg(not(unix))]
fn write_secret(path: &Path, contents: &[u8]) -> io::Result<()> {
    fs::write(path, contents)
}

pub fn payload_hash(payload: &Value) -> String {
    let bytes = serde_json::to_vec(payload).expect("serde_json value is serializable");
    format!("sha256:{}", hex::encode(Sha256::digest(bytes)))
}

pub fn envelope_schema() -> Value {
    serde_json::to_value(schema_for!(Envelope)).expect("schema serializes")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validates_subject_and_hash() {
        let subject = Subject::from_str("tenant/service/entity").unwrap();
        let mut envelope = Envelope::new(subject, "work.submit", json!({"a": 1}));
        envelope.validate().unwrap();
        envelope.payload = json!({"a": 2});
        assert_eq!(envelope.validate(), Err(CoreError::PayloadHashMismatch));
    }

    const SEED_A: [u8; SEED_LEN] = [7u8; SEED_LEN];
    const SEED_B: [u8; SEED_LEN] = [9u8; SEED_LEN];

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
        // Replace the embedded public key with a different one; signature no longer verifies.
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
}
