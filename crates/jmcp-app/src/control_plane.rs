use crate::control_plane_samples::{
    attention_inbox_sample, incident_records_sample, inventory_cards_sample, memory_records_sample,
    promotion_decisions_sample, voice_sessions_sample,
};
use crate::microtasks::{local_model_roots, MICROTASK_COUNT};
use crate::runtime_health::{
    local_gpu_inventory_health, local_model_inventory_health, local_speech_inventory_health,
};
use crate::{AppResult, AppState};
use jmcp_domain::{
    AttentionPacket, IncidentRecord, InventoryCard, InventoryCardKind, MemoryRecord,
    PromotionDecision, VoiceSession,
};
use uuid::Uuid;

impl AppState {
    pub fn voice_sessions(&self) -> AppResult<Vec<VoiceSession>> {
        let sessions = self
            .store
            .lock()
            .expect("store lock")
            .list_voice_sessions()?;
        if sessions.is_empty() {
            Ok(voice_sessions_sample())
        } else {
            Ok(sessions)
        }
    }

    pub fn attention_inbox(&self) -> AppResult<Vec<AttentionPacket>> {
        let packets = self
            .store
            .lock()
            .expect("store lock")
            .list_attention_packets()?;
        if packets.is_empty() {
            Ok(attention_inbox_sample())
        } else {
            Ok(packets)
        }
    }

    pub fn memory_records(&self) -> AppResult<Vec<MemoryRecord>> {
        let records = self
            .store
            .lock()
            .expect("store lock")
            .list_memory_records()?;
        if records.is_empty() {
            Ok(memory_records_sample())
        } else {
            Ok(records)
        }
    }

    pub fn inventory_cards(&self) -> AppResult<Vec<InventoryCard>> {
        let cards = self
            .store
            .lock()
            .expect("store lock")
            .list_inventory_cards()?;
        if cards.is_empty() {
            Ok(with_runtime_inventory(inventory_cards_sample()))
        } else {
            Ok(with_runtime_inventory(cards))
        }
    }

    pub fn promotion_decisions(&self) -> AppResult<Vec<PromotionDecision>> {
        let decisions = self
            .store
            .lock()
            .expect("store lock")
            .list_promotion_decisions()?;
        if decisions.is_empty() {
            Ok(promotion_decisions_sample())
        } else {
            Ok(decisions)
        }
    }

    pub fn incident_records(&self) -> AppResult<Vec<IncidentRecord>> {
        let incidents = self
            .store
            .lock()
            .expect("store lock")
            .list_incident_records()?;
        if incidents.is_empty() {
            Ok(incident_records_sample())
        } else {
            Ok(incidents)
        }
    }

    pub fn record_voice_session(&self, session: &VoiceSession) -> AppResult<()> {
        self.store
            .lock()
            .expect("store lock")
            .record_voice_session(session)?;
        Ok(())
    }

    pub fn record_attention_packet(&self, packet: &AttentionPacket) -> AppResult<()> {
        self.store
            .lock()
            .expect("store lock")
            .record_attention_packet(packet)?;
        Ok(())
    }

    pub fn record_memory_record(&self, record: &MemoryRecord) -> AppResult<()> {
        self.store
            .lock()
            .expect("store lock")
            .record_memory_record(record)?;
        Ok(())
    }

    pub fn record_inventory_card(&self, card: &InventoryCard) -> AppResult<()> {
        self.store
            .lock()
            .expect("store lock")
            .record_inventory_card(card)?;
        Ok(())
    }

    pub fn record_promotion_decision(&self, decision: &PromotionDecision) -> AppResult<()> {
        self.store
            .lock()
            .expect("store lock")
            .record_promotion_decision(decision)?;
        Ok(())
    }

    pub fn record_incident_record(&self, incident: &IncidentRecord) -> AppResult<()> {
        self.store
            .lock()
            .expect("store lock")
            .record_incident_record(incident)?;
        Ok(())
    }
}

fn with_runtime_inventory(mut cards: Vec<InventoryCard>) -> Vec<InventoryCard> {
    for card in runtime_inventory_cards() {
        if !cards.iter().any(|existing| existing.name == card.name) {
            cards.push(card);
        }
    }
    cards
}

fn runtime_inventory_cards() -> Vec<InventoryCard> {
    let gpu = local_gpu_inventory_health();
    let models = local_model_inventory_health();
    let speech = local_speech_inventory_health();
    vec![
        InventoryCard {
            id: uuid("55555555-5555-4555-8555-555555555554"),
            kind: InventoryCardKind::Tool,
            name: "jmcp.microtask-planner".to_owned(),
            owner: "jmcp".to_owned(),
            allowed_uses: vec![
                "catalog bounded evidence microtasks".to_owned(),
                "queue signed work orders".to_owned(),
            ],
            disallowed_uses: vec![
                "bypass envelope signatures".to_owned(),
                "start scheduler loops".to_owned(),
            ],
            cost: "SQLite work-order submissions".to_owned(),
            tests: vec![
                "microtask catalog unit".to_owned(),
                "API submit route".to_owned(),
            ],
            safety_case: "Every microtask is submitted through the signed JMCP work-order path."
                .to_owned(),
            health: jmcp_domain::HealthLevel::Nominal,
            repo: Some("JMCP".to_owned()),
            provider: Some("jmcp".to_owned()),
            queue: Some(MICROTASK_COUNT),
        },
        InventoryCard {
            id: uuid("55555555-5555-4555-8555-555555555555"),
            kind: InventoryCardKind::Model,
            name: "local-gpu.inventory".to_owned(),
            owner: "jmcp".to_owned(),
            allowed_uses: vec!["record local GPU inventory".to_owned()],
            disallowed_uses: vec![
                "load model weights".to_owned(),
                "start GPU inference".to_owned(),
            ],
            cost: "local nvidia-smi probe when available".to_owned(),
            tests: vec!["GPU inventory degradation unit".to_owned()],
            safety_case: gpu.detail.clone(),
            health: gpu.health,
            repo: None,
            provider: Some("local".to_owned()),
            queue: Some(0),
        },
        InventoryCard {
            id: uuid("55555555-5555-4555-8555-555555555556"),
            kind: InventoryCardKind::Model,
            name: "local-model.roots".to_owned(),
            owner: "jmcp".to_owned(),
            allowed_uses: vec!["record configured local model roots".to_owned()],
            disallowed_uses: vec![
                "install model binaries".to_owned(),
                "download model weights".to_owned(),
            ],
            cost: "local filesystem metadata only".to_owned(),
            tests: vec!["model root parsing unit".to_owned()],
            safety_case: format!("configured roots: {}", local_model_roots().join(", ")),
            health: models.health,
            repo: None,
            provider: Some("local".to_owned()),
            queue: Some(0),
        },
        InventoryCard {
            id: uuid("55555555-5555-4555-8555-555555555557"),
            kind: InventoryCardKind::Tool,
            name: "local-speech.inventory".to_owned(),
            owner: "jmcp".to_owned(),
            allowed_uses: vec!["record local ASR/TTS command availability".to_owned()],
            disallowed_uses: vec![
                "install speech models".to_owned(),
                "emit user-visible audio".to_owned(),
            ],
            cost: "local command detection only".to_owned(),
            tests: vec!["speech inventory degradation unit".to_owned()],
            safety_case: speech.detail,
            health: speech.health,
            repo: None,
            provider: Some("local".to_owned()),
            queue: Some(0),
        },
    ]
}

fn uuid(value: &str) -> Uuid {
    Uuid::parse_str(value).expect("valid runtime inventory uuid")
}
