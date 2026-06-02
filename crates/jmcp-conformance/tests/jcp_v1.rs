use jmcp_conformance::{
    ci_forgery_fixture, false_evidence_fixture, fixture_envelope, fixture_signer,
    memory_poisoning_fixture, prompt_injection_fixture, tool_poisoning_fixture,
    voice_replay_fixture,
};
use std::{
    fs,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

static HOME_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn home_lock() -> &'static Mutex<()> {
    HOME_LOCK.get_or_init(|| Mutex::new(()))
}

struct HomeVarGuard {
    previous: Option<std::ffi::OsString>,
}

impl Drop for HomeVarGuard {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(home) => std::env::set_var("HOME", home),
            None => std::env::remove_var("HOME"),
        }
    }
}

fn prepare_fixture_home() -> PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before epoch")
        .as_nanos();
    let home = std::env::temp_dir().join(format!("jmcp-conformance-home-{unique}"));
    let key_path = home.join(".local/share/jmcp/keys/local.key");
    fs::create_dir_all(key_path.parent().expect("fixture key parent")).unwrap();
    fs::write(&key_path, "5a".repeat(32)).unwrap();
    home
}

#[test]
fn fixture_is_valid_jcp_v1() {
    let envelope = fixture_envelope();
    let signer = fixture_signer();
    envelope.validate().unwrap();
    envelope.verify_local_signature(&signer).unwrap();
    assert_eq!(envelope.kind, "work.submit");
    assert_eq!(
        envelope.id.to_string(),
        "00000000-0000-0000-0000-000000000001"
    );
}

#[test]
fn fixture_submits_through_app() {
    let _guard = home_lock().lock().unwrap();
    let home = prepare_fixture_home();
    let _home_guard = HomeVarGuard {
        previous: std::env::var_os("HOME"),
    };
    std::env::set_var("HOME", &home);
    let state = jmcp_app::AppState::new(jmcp_store::SqliteStore::in_memory().unwrap());
    let work_order = state.submit_envelope(fixture_envelope()).unwrap();
    assert_eq!(work_order.subject, "tenant/jankurai/demo");
    assert_eq!(state.list_work_orders().unwrap().len(), 1);
}

#[test]
fn adversarial_fixtures_are_deterministic_jcp_v1() {
    let signer = fixture_signer();
    let cases = [
        (
            "user.message.received",
            "00000000-0000-0000-0000-000000000101",
            prompt_injection_fixture(),
        ),
        (
            "tool.card.published",
            "00000000-0000-0000-0000-000000000102",
            tool_poisoning_fixture(),
        ),
        (
            "memory.proposed",
            "00000000-0000-0000-0000-000000000103",
            memory_poisoning_fixture(),
        ),
        (
            "voice.turn.transcribed",
            "00000000-0000-0000-0000-000000000104",
            voice_replay_fixture(),
        ),
        (
            "evidence.appended",
            "00000000-0000-0000-0000-000000000105",
            false_evidence_fixture(),
        ),
        (
            "evidence.attested",
            "00000000-0000-0000-0000-000000000106",
            ci_forgery_fixture(),
        ),
    ];

    for (expected_kind, expected_id, envelope) in cases {
        envelope.validate().unwrap();
        envelope.verify_local_signature(&signer).unwrap();
        assert_eq!(envelope.subject, "tenant/jankurai/demo");
        assert_eq!(envelope.kind, expected_kind);
        assert_eq!(envelope.id.to_string(), expected_id);
    }
}
