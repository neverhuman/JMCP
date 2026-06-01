use jmcp_conformance::fixture_envelope;

#[test]
fn fixture_is_valid_jcp_v1() {
    let envelope = fixture_envelope();
    let signer = jcp_core::LocalSigner::load_or_create_default().unwrap();
    envelope.validate().unwrap();
    envelope.verify_local_signature(&signer).unwrap();
}

#[test]
fn fixture_submits_through_app() {
    let state = jmcp_app::AppState::new(jmcp_store::SqliteStore::in_memory().unwrap());
    let work_order = state.submit_envelope(fixture_envelope()).unwrap();
    assert_eq!(work_order.subject, "tenant/jankurai/demo");
    assert_eq!(state.list_work_orders().unwrap().len(), 1);
}
