use crate::control_plane_samples::{
    attention_inbox_sample, incident_records_sample, inventory_cards_sample, memory_records_sample,
    promotion_decisions_sample, voice_sessions_sample,
};
use crate::{AppResult, AppState};
use jmcp_domain::{
    AttentionPacket, IncidentRecord, InventoryCard, MemoryRecord, PromotionDecision, VoiceSession,
};

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
            Ok(inventory_cards_sample())
        } else {
            Ok(cards)
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
