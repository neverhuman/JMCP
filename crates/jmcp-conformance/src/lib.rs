use jcp_core::{Envelope, LocalSigner, Subject};
use serde_json::json;

pub fn fixture_envelope() -> Envelope {
    let signer = LocalSigner::load_or_create_default().expect("load local signer");
    signer.sign(Envelope::new(
        Subject {
            tenant: "tenant".to_owned(),
            service: "jankurai".to_owned(),
            entity: "demo".to_owned(),
        },
        "work.submit",
        json!({"command": "echo hello"}),
    ))
}
