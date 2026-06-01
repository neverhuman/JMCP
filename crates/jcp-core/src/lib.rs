use chrono::{DateTime, Utc};
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::str::FromStr;
use thiserror::Error;
use uuid::Uuid;

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

    pub fn sign_stub(mut self, key_id: impl Into<String>) -> Self {
        let signing_input = format!("{}.{}.{}", self.id, self.subject, self.payload_hash);
        let value = hex::encode(Sha256::digest(signing_input.as_bytes()));
        self.signature = Some(Signature {
            alg: "stub-sha256".to_owned(),
            key_id: key_id.into(),
            value,
        });
        self
    }

    pub fn verify_signature_stub(&self) -> Result<(), CoreError> {
        let signature = self.signature.as_ref().ok_or(CoreError::MissingSignature)?;
        let expected = self
            .clone()
            .sign_stub(signature.key_id.clone())
            .signature
            .unwrap();
        if signature == &expected {
            Ok(())
        } else {
            Err(CoreError::MissingSignature)
        }
    }
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

    #[test]
    fn signature_stub_round_trips() {
        let envelope = Envelope::new("t/s/e".parse().unwrap(), "x", json!({})).sign_stub("local");
        envelope.verify_signature_stub().unwrap();
    }
}
