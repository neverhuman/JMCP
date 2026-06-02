use chrono::{Duration, Utc};
use jmcp_domain::{
    AttentionLevel, AttentionPacket, Evidence, IncidentRecord, IncidentSeverity, IncidentState,
    InventoryCard, InventoryCardKind, MemoryPromotionState, MemoryRecord, PromotionDecision,
    PromotionVerdict, VoiceCandidate, VoiceRiskLevel, VoiceSession, VoiceSessionState,
};
use uuid::Uuid;

pub fn voice_sessions_sample() -> Vec<VoiceSession> {
    vec![
        VoiceSession {
            id: uuid("11111111-1111-4111-8111-111111111111"),
            work_order_id: None,
            channel: "telegram".to_owned(),
            transcript: "approve the deployment with token alpha".to_owned(),
            confidence: 0.97,
            candidate: VoiceCandidate {
                decision: jmcp_domain::ApprovalDecision::Approved,
                risk: VoiceRiskLevel::High,
                confirmation_token: Some("alpha".to_owned()),
            },
            confirmation_evidence: vec![evidence("voice.transcript", "sha256:voice-alpha")],
            state: VoiceSessionState::Confirmed,
            created_at: sample_time(-8),
            updated_at: sample_time(-7),
        },
        VoiceSession {
            id: uuid("11111111-1111-4111-8111-111111111112"),
            work_order_id: Some(uuid("22222222-2222-4222-8222-222222222222")),
            channel: "text".to_owned(),
            transcript: "deny the bridge write lease".to_owned(),
            confidence: 0.93,
            candidate: VoiceCandidate {
                decision: jmcp_domain::ApprovalDecision::Rejected,
                risk: VoiceRiskLevel::Low,
                confirmation_token: None,
            },
            confirmation_evidence: Vec::new(),
            state: VoiceSessionState::Candidate,
            created_at: sample_time(-13),
            updated_at: sample_time(-11),
        },
    ]
}

pub fn attention_inbox_sample() -> Vec<AttentionPacket> {
    vec![
        AttentionPacket {
            id: uuid("33333333-3333-4333-8333-333333333333"),
            work_order_id: Some(uuid("22222222-2222-4222-8222-222222222222")),
            title: "Bridge write lease still blocked".to_owned(),
            why_now: "The MCP bridge still lacks service-card evidence and should remain quarantined.".to_owned(),
            alternatives: vec![
                "Keep the adapter read-only".to_owned(),
                "Promote a narrower lease after evidence lands".to_owned(),
            ],
            risk_delta: "Promoting now increases blast radius from local read-only to write access.".to_owned(),
            drill_down: "Open the adapter panel for the service card, evidence bundle, and quarantine note.".to_owned(),
            level: AttentionLevel::Page,
            created_at: sample_time(-5),
            updated_at: sample_time(-4),
        },
        AttentionPacket {
            id: uuid("33333333-3333-4333-8333-333333333334"),
            work_order_id: None,
            title: "Voice approval needs confirmation evidence".to_owned(),
            why_now: "A high-risk spoken approval is pending a confirmation token and transcript review.".to_owned(),
            alternatives: vec![
                "Request a fresh transcript".to_owned(),
                "Switch to text approval".to_owned(),
            ],
            risk_delta: "Accepting an under-verified voice command could approve an unintended destructive action.".to_owned(),
            drill_down: "Inspect the voice session to confirm confidence, transcript, and candidate decision.".to_owned(),
            level: AttentionLevel::Warn,
            created_at: sample_time(-3),
            updated_at: sample_time(-2),
        },
    ]
}

pub fn memory_records_sample() -> Vec<MemoryRecord> {
    vec![
        MemoryRecord {
            id: uuid("44444444-4444-4444-8444-444444444441"),
            lesson: "Adapters that emit raw webhooks stay quarantined until wrapped in JCP envelopes.".to_owned(),
            scope: "adapter conformance".to_owned(),
            source_evidence: vec![evidence("conformance.report", "sha256:memory-1")],
            freshness: "fresh".to_owned(),
            counterexamples: vec![
                "Direct webhook handlers bypass policy.".to_owned(),
                "Silent retries can duplicate side effects.".to_owned(),
            ],
            poisoning_checks: vec![
                "compare against recorded evidence".to_owned(),
                "reject unsigned lessons".to_owned(),
            ],
            promotion_policy: "shadow until replay proof lands".to_owned(),
            state: MemoryPromotionState::Proposed,
            expires_at: Some(sample_time(7 * 24 * 60)),
            created_at: sample_time(-4 * 60),
            updated_at: sample_time(-60),
        },
        MemoryRecord {
            id: uuid("44444444-4444-4444-8444-444444444442"),
            lesson: "Evidence gates need independent replay checks before schema promotion.".to_owned(),
            scope: "release policy".to_owned(),
            source_evidence: vec![evidence("replay.checkpoint", "sha256:memory-2")],
            freshness: "stable".to_owned(),
            counterexamples: vec!["A green smoke test is not sufficient without replay parity.".to_owned()],
            poisoning_checks: vec!["verify source hash".to_owned(), "cross-check against counterexample search".to_owned()],
            promotion_policy: "promote only after advisory floor stays above threshold".to_owned(),
            state: MemoryPromotionState::Shadow,
            expires_at: Some(sample_time(14 * 24 * 60)),
            created_at: sample_time(-24 * 60),
            updated_at: sample_time(-2 * 60),
        },
        MemoryRecord {
            id: uuid("44444444-4444-4444-8444-444444444443"),
            lesson: "Direct credential access inside workers remains a policy violation even when tests pass.".to_owned(),
            scope: "authority kernel".to_owned(),
            source_evidence: vec![evidence("security.finding", "sha256:memory-3")],
            freshness: "archived".to_owned(),
            counterexamples: vec!["User-visible success does not override policy.".to_owned()],
            poisoning_checks: vec!["require independent review".to_owned()],
            promotion_policy: "already promoted; keep as guardrail".to_owned(),
            state: MemoryPromotionState::Promoted,
            expires_at: None,
            created_at: sample_time(-3 * 24 * 60),
            updated_at: sample_time(-24 * 60),
        },
    ]
}

pub fn inventory_cards_sample() -> Vec<InventoryCard> {
    vec![
        InventoryCard {
            id: uuid("55555555-5555-4555-8555-555555555551"),
            kind: InventoryCardKind::Tool,
            name: "jeryu.repo.adopt".to_owned(),
            owner: "jeryu".to_owned(),
            allowed_uses: vec![
                "adopt local repositories".to_owned(),
                "publish repo metadata".to_owned(),
            ],
            disallowed_uses: vec!["write secrets".to_owned(), "bypass approvals".to_owned()],
            cost: "local git and metadata writes".to_owned(),
            tests: vec![
                "tool card unit".to_owned(),
                "conformance negative".to_owned(),
            ],
            safety_case: "All changes remain lease- and approval-gated.".to_owned(),
            health: jmcp_domain::HealthLevel::Watch,
            repo: Some("Jeryu".to_owned()),
            provider: Some("jeryu".to_owned()),
            queue: Some(1),
        },
        InventoryCard {
            id: uuid("55555555-5555-4555-8555-555555555552"),
            kind: InventoryCardKind::Tool,
            name: "jmcpd.submit".to_owned(),
            owner: "jmcpd".to_owned(),
            allowed_uses: vec![
                "submit signed envelopes".to_owned(),
                "record approval state".to_owned(),
            ],
            disallowed_uses: vec!["write around policy".to_owned()],
            cost: "SQLite writes".to_owned(),
            tests: vec!["app integration".to_owned(), "replay parity".to_owned()],
            safety_case: "All mutating paths route through app/store approval checks.".to_owned(),
            health: jmcp_domain::HealthLevel::Nominal,
            repo: Some("JMCP".to_owned()),
            provider: Some("jmcpd".to_owned()),
            queue: Some(0),
        },
        InventoryCard {
            id: uuid("55555555-5555-4555-8555-555555555553"),
            kind: InventoryCardKind::Model,
            name: "local-reasoner".to_owned(),
            owner: "jmcp".to_owned(),
            allowed_uses: vec![
                "summarize control-plane state".to_owned(),
                "draft proposals".to_owned(),
            ],
            disallowed_uses: vec!["decide approvals".to_owned(), "access secrets".to_owned()],
            cost: "local inference budget".to_owned(),
            tests: vec![
                "prompt injection negative".to_owned(),
                "false evidence negative".to_owned(),
            ],
            safety_case: "Assistant output remains advisory and never bypasses JMCP policy."
                .to_owned(),
            health: jmcp_domain::HealthLevel::Watch,
            repo: None,
            provider: Some("local".to_owned()),
            queue: Some(2),
        },
    ]
}

pub fn promotion_decisions_sample() -> Vec<PromotionDecision> {
    vec![
        PromotionDecision {
            id: uuid("66666666-6666-4666-8666-666666666661"),
            target_kind: "memory_record".to_owned(),
            target_name: "ML-219".to_owned(),
            gate: "independent replay and evidence review".to_owned(),
            verdict: PromotionVerdict::Proposed,
            verifier: "jankurai".to_owned(),
            rollback_plan: "Keep the memory record shadowed until the replay lane is green."
                .to_owned(),
            evidence_count: 3,
            created_at: sample_time(-3 * 60),
            decided_at: sample_time(-2 * 60),
        },
        PromotionDecision {
            id: uuid("66666666-6666-4666-8666-666666666662"),
            target_kind: "tool_card".to_owned(),
            target_name: "jmcpd.submit".to_owned(),
            gate: "policy and security review".to_owned(),
            verdict: PromotionVerdict::Promoted,
            verifier: "ops".to_owned(),
            rollback_plan: "Disable the route and fall back to signed-envelope intake.".to_owned(),
            evidence_count: 5,
            created_at: sample_time(-24 * 60),
            decided_at: sample_time(-20 * 60),
        },
    ]
}

pub fn incident_records_sample() -> Vec<IncidentRecord> {
    vec![
        IncidentRecord {
            id: uuid("77777777-7777-4777-8777-777777777771"),
            title: "MCP bridge remains quarantined".to_owned(),
            severity: IncidentSeverity::Major,
            state: IncidentState::Quarantined,
            quarantine_scope: "adapter/mcp".to_owned(),
            containment:
                "Keep the bridge read-only until the service card and evidence bundle are complete."
                    .to_owned(),
            related_work_orders: vec![uuid("22222222-2222-4222-8222-222222222222")],
            notes: vec![
                "bridge write lease denied".to_owned(),
                "evidence gap still open".to_owned(),
            ],
            opened_at: sample_time(-6 * 60),
            updated_at: sample_time(-30),
        },
        IncidentRecord {
            id: uuid("77777777-7777-4777-8777-777777777772"),
            title: "Voice replay safeguard is active".to_owned(),
            severity: IncidentSeverity::Warning,
            state: IncidentState::Investigating,
            quarantine_scope: "voice approval path".to_owned(),
            containment: "Require confirmation tokens for high-risk spoken approvals.".to_owned(),
            related_work_orders: Vec::new(),
            notes: vec!["low-confidence transcript rejected".to_owned()],
            opened_at: sample_time(-2 * 60),
            updated_at: sample_time(-12),
        },
    ]
}

fn evidence(kind: &str, uri: &str) -> Evidence {
    Evidence {
        kind: kind.to_owned(),
        uri: uri.to_owned(),
        captured_at: sample_time(0),
    }
}

fn uuid(value: &str) -> Uuid {
    Uuid::parse_str(value).expect("valid sample uuid")
}

fn sample_time(offset_minutes: i64) -> chrono::DateTime<Utc> {
    let base = chrono::DateTime::parse_from_rfc3339("2025-01-01T12:00:00Z")
        .expect("valid sample base")
        .with_timezone(&Utc);
    base + Duration::minutes(offset_minutes)
}
