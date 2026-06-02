use anyhow::Result;
use jcp_core::{Envelope, LocalSigner, Subject};
use jmcp_app::{telegram_actor, AppState, ApprovalDecisionError};
use jmcp_domain::{ApprovalDecision, WorkOrder};
use std::str::FromStr;
use uuid::Uuid;

pub(crate) fn emit_structured_event(level: &str, event: &str, fields: serde_json::Value) {
    let record = structured_event_record(level, event, fields);
    match level {
        "error" => eprintln!("{}", record),
        _ => println!("{}", record),
    }
}

pub(crate) fn structured_event_record(
    level: &str,
    event: &str,
    fields: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "eventId": uuid::Uuid::new_v4(),
        "event": event,
        "level": level,
        "component": "jmcpd",
        "timestamp": chrono::Utc::now(),
        "fields": fields,
    })
}

pub(crate) fn submit_from_telegram(
    state: &AppState,
    subject: &str,
    kind: &str,
    payload: &str,
) -> Result<WorkOrder> {
    let payload = serde_json::from_str(payload)?;
    let signer = LocalSigner::load_or_create_default()?;
    let envelope = signer.sign(Envelope::new(
        Subject::from_str(subject)?,
        kind.to_owned(),
        payload,
    ));
    Ok(state.submit_envelope(envelope)?)
}

pub(crate) fn status_from_telegram(state: &AppState, id: Uuid) -> String {
    match state.work_order(id) {
        Ok(Some(work_order)) => {
            let attention = if work_order.attention.is_empty() {
                "none".to_owned()
            } else {
                work_order
                    .attention
                    .iter()
                    .map(|item| item.reason.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            format!(
                "JMCP work order {}: {:?}; attention: {}; evidence: {}.",
                work_order.id,
                work_order.status,
                attention,
                work_order.evidence.len()
            )
        }
        Ok(None) => "JMCP status rejected: unknown work order id.".to_owned(),
        Err(_) => "JMCP status unavailable: state could not be read.".to_owned(),
    }
}

pub(crate) fn decide_from_telegram(
    state: &AppState,
    token: &str,
    user_id: i64,
    chat_id: i64,
    decision: ApprovalDecision,
) -> String {
    match state.decide_approval_by_token(token.trim(), telegram_actor(user_id, chat_id), decision) {
        Ok(outcome) => format!(
            "JMCP approval {:?} for work order {}.",
            outcome.approval.decision.unwrap_or(decision),
            outcome.work_order.id
        ),
        Err(ApprovalDecisionError::UnknownToken) => {
            "JMCP approval rejected: unknown token.".to_owned()
        }
        Err(ApprovalDecisionError::Expired) => "JMCP approval rejected: expired token.".to_owned(),
        Err(ApprovalDecisionError::AlreadyUsed) => {
            "JMCP approval rejected: token already used.".to_owned()
        }
        Err(ApprovalDecisionError::WrongApprover) => {
            "JMCP approval rejected: wrong Telegram approver.".to_owned()
        }
        Err(ApprovalDecisionError::UnavailableState(_)) => {
            "JMCP approval unavailable: state could not be updated.".to_owned()
        }
    }
}
