use std::{
    io::Write,
    process::{Command, Stdio},
};

use chrono::{Duration, TimeZone, Utc};
use jmcp_domain::{
    ApprovalChallenge, ApprovalChallengeState, ApprovalChannel, Attention, AttentionLevel,
    Evidence, JituxFrame, JituxFrameBase, JituxFrameSource, PaneVm, Task, WorkOrder,
    WorkOrderStatus,
};
use jmcp_now::{queue_blockers_panes, queue_blockers_projection, NowReads};
use uuid::Uuid;

const JITUX_FRAME_SCHEMA: &str =
    include_str!("../../../schemas/jitux/1.0.0/jitux-frame.schema.json");

#[test]
fn queue_blocker_panes_round_trip_as_canonical_pane_vms() {
    let panes = queue_blockers_panes(&reads(), fixed_time());

    assert_eq!(panes.len(), 2);
    let json = serde_json::to_value(&panes).expect("canonical panes serialize");
    let decoded: Vec<PaneVm> = serde_json::from_value(json).expect("canonical panes deserialize");

    assert_eq!(decoded, panes);
    assert!(panes
        .iter()
        .any(|pane| pane.title == "Bridge write lease"
            && pane.preview.headline.contains("MCP bridge")));
}

#[test]
fn queue_blocker_pane_upsert_frames_validate_against_canonical_schema() {
    let panes = queue_blockers_panes(&reads(), fixed_time());

    for (index, pane) in panes.into_iter().enumerate() {
        let frame = JituxFrame::PaneUpsert {
            base: base(index as u64 + 1),
            pane,
        };
        let value = serde_json::to_value(&frame).expect("pane frame json");

        validate_against_jitux_schema(&value);
        assert_eq!(
            serde_json::from_value::<JituxFrame>(value).expect("pane frame round trip"),
            frame
        );
    }
}

#[test]
fn queue_blocker_action_ready_frames_validate_against_canonical_schema() {
    let projection = queue_blockers_projection(&reads(), fixed_time());
    let pane = projection
        .panes
        .iter()
        .find(|pane| pane.title == "Bridge write lease")
        .expect("focused pane");
    let action = projection
        .prepared_actions
        .get(&pane.id)
        .expect("prepared actions")
        .iter()
        .find(|action| action.requires_approval)
        .expect("approval action")
        .clone();

    let frame = JituxFrame::ActionReady {
        base: base(100),
        pane_id: pane.id.clone(),
        action,
    };
    let value = serde_json::to_value(&frame).expect("action frame json");

    validate_against_jitux_schema(&value);
    assert_eq!(
        serde_json::from_value::<JituxFrame>(value).expect("action frame round trip"),
        frame
    );
}

fn validate_against_jitux_schema(instance: &serde_json::Value) {
    let schema: serde_json::Value =
        serde_json::from_str(JITUX_FRAME_SCHEMA).expect("canonical JITUX schema parses");
    let payload = serde_json::json!({
        "schema": schema,
        "instance": instance,
    });
    let mut child = Command::new("python3")
        .arg("-c")
        .arg(
            r#"
import json
import jsonschema
import sys

payload = json.load(sys.stdin)
jsonschema.Draft202012Validator(payload["schema"]).validate(payload["instance"])
"#,
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("python3 jsonschema validator starts");

    let mut stdin = child.stdin.take().expect("validator stdin");
    stdin
        .write_all(serde_json::to_string(&payload).unwrap().as_bytes())
        .expect("validator input write");
    drop(stdin);

    let output = child.wait_with_output().expect("validator exits");
    assert!(
        output.status.success(),
        "JITUX schema validation failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn reads() -> NowReads {
    let focused_id = uuid("22222222-2222-4222-8222-222222222222");
    let other_id = uuid("99999999-9999-4999-8999-999999999991");
    let state = jmcp_app::AppState::new(jmcp_store::SqliteStore::in_memory().unwrap());
    NowReads {
        work_orders: vec![
            work_order(
                focused_id,
                "Bridge write lease",
                WorkOrderStatus::AwaitingApproval,
            )
            .with_evidence(),
            work_order(
                other_id,
                "Local inventory probe",
                WorkOrderStatus::Submitted,
            ),
        ],
        leases: vec![jmcp_domain::Lease {
            work_order_id: focused_id,
            holder: "jmcpd".to_owned(),
            expires_at: fixed_time() + Duration::minutes(10),
        }],
        attention_packets: jmcp_app::attention_inbox_sample(),
        approval_challenges: vec![ApprovalChallenge {
            id: uuid("88888888-8888-4888-8888-888888888881"),
            work_order_id: focused_id,
            approver: "ops".to_owned(),
            channel: ApprovalChannel::Local,
            target_user_id: None,
            target_chat_id: None,
            token_hash: "sha256:test".to_owned(),
            expires_at: fixed_time() + Duration::minutes(20),
            state: ApprovalChallengeState::Pending,
            decision: None,
            created_at: fixed_time() - Duration::minutes(5),
            updated_at: fixed_time() - Duration::minutes(4),
        }],
        incidents: jmcp_app::incident_records_sample(),
        autonomous_actions: state.list_autonomous_actions().unwrap(),
    }
}

trait WorkOrderFixture {
    fn with_evidence(self) -> Self;
}

impl WorkOrderFixture for WorkOrder {
    fn with_evidence(mut self) -> Self {
        self.evidence.push(Evidence {
            kind: "service-card".to_owned(),
            uri: "sha256:evidence".to_owned(),
            captured_at: fixed_time() - Duration::minutes(3),
        });
        self
    }
}

fn work_order(id: Uuid, subject: &str, status: WorkOrderStatus) -> WorkOrder {
    WorkOrder {
        id,
        subject: subject.to_owned(),
        task: Task {
            kind: "jmcp.test".to_owned(),
            payload: serde_json::json!({ "id": id.to_string() }),
        },
        status,
        evidence: Vec::new(),
        attention: vec![Attention {
            level: AttentionLevel::Warn,
            reason: "test attention".to_owned(),
        }],
        created_at: fixed_time() - Duration::minutes(30),
        updated_at: fixed_time() - Duration::minutes(5),
    }
}

fn base(seq: u64) -> JituxFrameBase {
    JituxFrameBase {
        v: 1,
        session_id: "jmcp_now_test".to_owned(),
        seq,
        frame_id: format!("now_frame_{seq:04}"),
        emitted_at: fixed_time(),
        source: JituxFrameSource::Projection,
        ttl_ms: Some(30_000),
    }
}

fn fixed_time() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2025, 1, 1, 12, 0, 0)
        .single()
        .expect("valid fixed time")
}

fn uuid(value: &str) -> Uuid {
    Uuid::parse_str(value).expect("valid uuid")
}
