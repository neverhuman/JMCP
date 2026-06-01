use jcp_core::{Envelope, Subject};
use serde_json::json;

pub fn fixture_envelope() -> Envelope {
    Envelope::new(
        Subject {
            tenant: "tenant".to_owned(),
            service: "jankurai".to_owned(),
            entity: "demo".to_owned(),
        },
        "work.submit",
        json!({"command": "echo hello"}),
    )
    .sign_stub("conformance")
}
