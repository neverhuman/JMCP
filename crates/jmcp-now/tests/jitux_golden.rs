use chrono::{DateTime, TimeZone, Utc};
use jmcp_now::{
    Accent, ActionMethod, Card, CardKind, CardStatus, DrilldownKind, DrilldownRef, PaneKind,
    PaneStatus, PreparedAction, RankFactorKind, RankFactors, RankReason, RiskBand, SafetyClass,
    Scene, SceneLayout, SceneMode,
};
use schemars::schema_for;
use serde::Serialize;
use serde_json::json;

const SCENE_GOLDEN: &str = include_str!("golden/queue_blockers_scene.json");
const SCHEMA_GOLDEN: &str = include_str!("golden/scene.schema.json");

#[test]
fn scene_json_matches_golden() {
    let actual = pretty(&golden_scene());
    assert_eq!(actual, SCENE_GOLDEN);

    let value: serde_json::Value = serde_json::from_str(&actual).expect("scene json");
    let first_card = &value["cards"][0];
    assert!(first_card.get("whyNow").is_some());
    assert!(first_card.get("rankReason").is_some());
    assert!(first_card.get("evidenceRefs").is_some());
    assert!(first_card["actions"][0].get("safetyClass").is_some());
}

#[test]
fn scene_schema_matches_golden() {
    let actual = pretty(&schema_for!(Scene));
    assert_eq!(actual, SCHEMA_GOLDEN);
}

#[test]
fn enum_wire_values_are_snake_case() {
    assert_eq!(
        json!({
            "paneKind": wire(&[
                PaneKind::Queue,
                PaneKind::Jeryu,
                PaneKind::Jailgun,
                PaneKind::Jekko,
                PaneKind::Evidence,
                PaneKind::Replay,
                PaneKind::Approval,
                PaneKind::AdapterHealth,
                PaneKind::Memory,
                PaneKind::Autonomy,
            ]),
            "paneStatus": wire(&[
                PaneStatus::Predicted,
                PaneStatus::Incubating,
                PaneStatus::Warm,
                PaneStatus::Active,
                PaneStatus::Degraded,
            ]),
            "sceneMode": wire(&[
                SceneMode::Focus,
                SceneMode::Fan,
                SceneMode::Compare,
                SceneMode::Tunnel,
            ]),
            "accent": wire(&[
                Accent::Purple,
                Accent::Red,
                Accent::Amber,
                Accent::Green,
                Accent::Blue,
                Accent::Slate,
            ]),
            "sceneLayout": wire(&[
                SceneLayout::Stack,
                SceneLayout::Timeline,
                SceneLayout::Matrix,
                SceneLayout::Drilldown,
            ]),
            "cardKind": wire(&[
                CardKind::QueueBlocker,
                CardKind::Lease,
                CardKind::Approval,
                CardKind::Evidence,
                CardKind::Incident,
                CardKind::AdapterHealth,
                CardKind::Replay,
                CardKind::Autonomy,
                CardKind::Memory,
            ]),
            "cardStatus": wire(&[
                CardStatus::Probing,
                CardStatus::Ranked,
                CardStatus::Verified,
            ]),
            "riskBand": wire(&[
                RiskBand::Low,
                RiskBand::Medium,
                RiskBand::High,
            ]),
            "safetyClass": wire(&[
                SafetyClass::ReadOnly,
                SafetyClass::BoundedAuto,
                SafetyClass::ApprovalRequired,
                SafetyClass::ManualOnly,
            ]),
            "drilldownKind": wire(&[
                DrilldownKind::Evidence,
                DrilldownKind::Lease,
                DrilldownKind::Approval,
                DrilldownKind::Replay,
                DrilldownKind::System,
                DrilldownKind::Raw,
            ]),
            "actionMethod": wire(&[
                ActionMethod::Get,
                ActionMethod::Post,
            ]),
            "rankFactorKind": wire(&[
                RankFactorKind::Risk,
                RankFactorKind::Actionability,
                RankFactorKind::Freshness,
                RankFactorKind::BlastRadius,
                RankFactorKind::LeasePressure,
                RankFactorKind::UserRelevance,
            ]),
        }),
        json!({
            "paneKind": ["queue", "jeryu", "jailgun", "jekko", "evidence", "replay", "approval", "adapter_health", "memory", "autonomy"],
            "paneStatus": ["predicted", "incubating", "warm", "active", "degraded"],
            "sceneMode": ["focus", "fan", "compare", "tunnel"],
            "accent": ["purple", "red", "amber", "green", "blue", "slate"],
            "sceneLayout": ["stack", "timeline", "matrix", "drilldown"],
            "cardKind": ["queue_blocker", "lease", "approval", "evidence", "incident", "adapter_health", "replay", "autonomy", "memory"],
            "cardStatus": ["probing", "ranked", "verified"],
            "riskBand": ["low", "medium", "high"],
            "safetyClass": ["read_only", "bounded_auto", "approval_required", "manual_only"],
            "drilldownKind": ["evidence", "lease", "approval", "replay", "system", "raw"],
            "actionMethod": ["get", "post"],
            "rankFactorKind": ["risk", "actionability", "freshness", "blast_radius", "lease_pressure", "user_relevance"],
        })
    );
}

#[test]
#[ignore = "writes committed golden fixtures when JMCP_NOW_WRITE_GOLDEN=1"]
fn write_golden_files() {
    if std::env::var("JMCP_NOW_WRITE_GOLDEN").as_deref() != Ok("1") {
        return;
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    std::fs::create_dir_all(&root).expect("golden dir");
    std::fs::write(
        root.join("queue_blockers_scene.json"),
        pretty(&golden_scene()),
    )
    .expect("scene golden write");
    std::fs::write(root.join("scene.schema.json"), pretty(&schema_for!(Scene)))
        .expect("schema golden write");
}

fn golden_scene() -> Scene {
    Scene {
        key: "queue_blockers".to_owned(),
        kind: PaneKind::Queue,
        mode: SceneMode::Focus,
        accent: Accent::Purple,
        title: "What's blocking the queue?".to_owned(),
        layout: SceneLayout::Stack,
        status: PaneStatus::Active,
        generation: 42,
        captured_at: fixed_time(),
        cards: vec![
            card(
                "queue-card",
                CardKind::QueueBlocker,
                CardStatus::Verified,
                RiskBand::High,
                RankFactorKind::Risk,
            )
            .with_actions(all_actions())
            .with_drilldowns(all_drilldowns())
            .with_evidence_refs(vec![drilldown("evidence:1", DrilldownKind::Evidence)]),
            card(
                "lease-card",
                CardKind::Lease,
                CardStatus::Ranked,
                RiskBand::Medium,
                RankFactorKind::LeasePressure,
            ),
            card(
                "approval-card",
                CardKind::Approval,
                CardStatus::Probing,
                RiskBand::Low,
                RankFactorKind::Actionability,
            ),
            card(
                "evidence-card",
                CardKind::Evidence,
                CardStatus::Verified,
                RiskBand::Medium,
                RankFactorKind::Freshness,
            ),
            card(
                "incident-card",
                CardKind::Incident,
                CardStatus::Ranked,
                RiskBand::High,
                RankFactorKind::BlastRadius,
            ),
            card(
                "adapter-card",
                CardKind::AdapterHealth,
                CardStatus::Ranked,
                RiskBand::Medium,
                RankFactorKind::UserRelevance,
            ),
            card(
                "replay-card",
                CardKind::Replay,
                CardStatus::Verified,
                RiskBand::Low,
                RankFactorKind::Freshness,
            ),
            card(
                "autonomy-card",
                CardKind::Autonomy,
                CardStatus::Ranked,
                RiskBand::Medium,
                RankFactorKind::Actionability,
            ),
            card(
                "memory-card",
                CardKind::Memory,
                CardStatus::Ranked,
                RiskBand::Low,
                RankFactorKind::UserRelevance,
            ),
        ],
        narration_hint:
            "Name the highest ranked blocker, then show approval, lease, and evidence refs."
                .to_owned(),
    }
}

trait CardFixture {
    fn with_actions(self, actions: Vec<PreparedAction>) -> Self;
    fn with_drilldowns(self, drilldowns: Vec<DrilldownRef>) -> Self;
    fn with_evidence_refs(self, evidence_refs: Vec<DrilldownRef>) -> Self;
}

impl CardFixture for Card {
    fn with_actions(mut self, actions: Vec<PreparedAction>) -> Self {
        self.actions = actions;
        self
    }

    fn with_drilldowns(mut self, drilldowns: Vec<DrilldownRef>) -> Self {
        self.drilldowns = drilldowns;
        self
    }

    fn with_evidence_refs(mut self, evidence_refs: Vec<DrilldownRef>) -> Self {
        self.evidence_refs = evidence_refs;
        self
    }
}

fn card(
    id: &str,
    kind: CardKind,
    status: CardStatus,
    risk: RiskBand,
    dominant_factor: RankFactorKind,
) -> Card {
    Card {
        id: id.to_owned(),
        kind,
        title: format!("{id} title"),
        status,
        rank: 0.75,
        risk,
        why_now: format!("{id} is visible now because the queue is blocked."),
        rank_reason: RankReason {
            score: 0.75,
            factors: RankFactors {
                risk: 0.8,
                actionability: 0.7,
                freshness: 0.6,
                blast_radius: 0.5,
                lease_pressure: 0.4,
                user_relevance: 0.3,
            },
            summary: format!("{id} ranks here for a visible operational reason."),
            dominant_factor,
        },
        evidence_refs: Vec::new(),
        drilldowns: Vec::new(),
        actions: Vec::new(),
    }
}

fn all_actions() -> Vec<PreparedAction> {
    vec![
        action("read", SafetyClass::ReadOnly, ActionMethod::Get),
        action("auto", SafetyClass::BoundedAuto, ActionMethod::Post),
        action(
            "approval",
            SafetyClass::ApprovalRequired,
            ActionMethod::Post,
        ),
        action("manual", SafetyClass::ManualOnly, ActionMethod::Get),
    ]
}

fn action(id: &str, safety_class: SafetyClass, method: ActionMethod) -> PreparedAction {
    PreparedAction {
        id: format!("action:{id}"),
        label: format!("{id} action"),
        safety_class,
        ready: !matches!(
            safety_class,
            SafetyClass::ApprovalRequired | SafetyClass::ManualOnly
        ),
        reason: format!("{id} action is governed by its safety class."),
        target: format!("targets/{id}"),
        method,
    }
}

fn all_drilldowns() -> Vec<DrilldownRef> {
    vec![
        drilldown("evidence", DrilldownKind::Evidence),
        drilldown("lease", DrilldownKind::Lease),
        drilldown("approval", DrilldownKind::Approval),
        drilldown("replay", DrilldownKind::Replay),
        drilldown("system", DrilldownKind::System),
        drilldown("raw", DrilldownKind::Raw),
    ]
}

fn drilldown(id: &str, kind: DrilldownKind) -> DrilldownRef {
    DrilldownRef {
        id: format!("drilldown:{id}"),
        label: format!("{id} drilldown"),
        kind,
        target: format!("targets/{id}"),
    }
}

fn fixed_time() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2025, 1, 1, 12, 0, 0)
        .single()
        .expect("valid fixed time")
}

fn pretty<T>(value: &T) -> String
where
    T: Serialize,
{
    let mut json = serde_json::to_string_pretty(value).expect("pretty json");
    json.push('\n');
    json
}

fn wire<T>(variants: &[T]) -> Vec<serde_json::Value>
where
    T: Serialize,
{
    variants
        .iter()
        .map(|variant| serde_json::to_value(variant).expect("enum json"))
        .collect()
}
