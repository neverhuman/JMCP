use chrono::{DateTime, Utc};
use jmcp_domain::{DeckRankFactors, DeckRankReason};

pub const RISK_WEIGHT: f32 = 0.25;
pub const BLOCKEDNESS_WEIGHT: f32 = 0.20;
pub const APPROVAL_EXPIRY_PRESSURE_WEIGHT: f32 = 0.10;
pub const LEASE_PRESSURE_WEIGHT: f32 = 0.10;
pub const ADAPTER_DEGRADED_WEIGHT: f32 = 0.05;
pub const EVIDENCE_GAP_WEIGHT: f32 = 0.10;
pub const USER_QUERY_RELEVANCE_WEIGHT: f32 = 0.05;
pub const FRESHNESS_WEIGHT: f32 = 0.05;
pub const DOWNSTREAM_BLAST_RADIUS_WEIGHT: f32 = 0.10;

const FRESHNESS_WINDOW_MINUTES: i64 = 60;
const LEASE_PRESSURE_WINDOW_MINUTES: i64 = 60;
const APPROVAL_EXPIRY_WINDOW_MINUTES: i64 = 60;

#[derive(Clone, Debug, PartialEq)]
pub struct RankInput {
    pub id: String,
    pub subject: String,
    pub risk: f32,
    pub blockedness: f32,
    pub approval_expires_at: Option<DateTime<Utc>>,
    pub lease_expires_at: Option<DateTime<Utc>>,
    pub adapter_degraded_weight: f32,
    pub evidence_gap_weight: f32,
    pub user_query_relevance: f32,
    pub updated_at: DateTime<Utc>,
    pub downstream_blast_radius: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RankedInput {
    pub input: RankInput,
    pub reason: DeckRankReason,
}

pub fn rank_inputs(inputs: Vec<RankInput>, now: DateTime<Utc>) -> Vec<RankedInput> {
    let mut ranked = inputs
        .into_iter()
        .map(|input| {
            let reason = rank_reason(&input, now);
            RankedInput { input, reason }
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .reason
            .score
            .total_cmp(&left.reason.score)
            .then_with(|| left.input.id.cmp(&right.input.id))
    });
    ranked
}

pub fn rank_reason(input: &RankInput, now: DateTime<Utc>) -> DeckRankReason {
    let factors = DeckRankFactors {
        risk: normalized(input.risk),
        blockedness: normalized(input.blockedness),
        approval_expiry_pressure: approval_expiry_pressure_factor(input.approval_expires_at, now),
        lease_pressure: lease_pressure_factor(input.lease_expires_at, now),
        adapter_degraded_weight: normalized(input.adapter_degraded_weight),
        evidence_gap_weight: normalized(input.evidence_gap_weight),
        user_query_relevance: normalized(input.user_query_relevance),
        freshness: freshness_factor(input.updated_at, now),
        downstream_blast_radius: normalized(input.downstream_blast_radius),
    };
    let score = weighted_score(&factors);
    let dominant_factor = dominant_factor(&factors);
    DeckRankReason {
        score,
        factors,
        explanation: explanation(&input.subject, dominant_factor, score),
    }
}

pub fn weighted_score(factors: &DeckRankFactors) -> f32 {
    RISK_WEIGHT * factors.risk
        + BLOCKEDNESS_WEIGHT * factors.blockedness
        + APPROVAL_EXPIRY_PRESSURE_WEIGHT * factors.approval_expiry_pressure
        + LEASE_PRESSURE_WEIGHT * factors.lease_pressure
        + ADAPTER_DEGRADED_WEIGHT * factors.adapter_degraded_weight
        + EVIDENCE_GAP_WEIGHT * factors.evidence_gap_weight
        + USER_QUERY_RELEVANCE_WEIGHT * factors.user_query_relevance
        + FRESHNESS_WEIGHT * factors.freshness
        + DOWNSTREAM_BLAST_RADIUS_WEIGHT * factors.downstream_blast_radius
}

pub fn freshness_factor(updated_at: DateTime<Utc>, now: DateTime<Utc>) -> f32 {
    let age = now.signed_duration_since(updated_at);
    if age.num_seconds() <= 0 {
        return 1.0;
    }
    normalized(1.0 - age.num_minutes() as f32 / FRESHNESS_WINDOW_MINUTES as f32)
}

pub fn lease_pressure_factor(expires_at: Option<DateTime<Utc>>, now: DateTime<Utc>) -> f32 {
    expiry_pressure_factor(expires_at, now, LEASE_PRESSURE_WINDOW_MINUTES)
}

pub fn approval_expiry_pressure_factor(
    expires_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> f32 {
    expiry_pressure_factor(expires_at, now, APPROVAL_EXPIRY_WINDOW_MINUTES)
}

fn expiry_pressure_factor(
    expires_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
    window_minutes: i64,
) -> f32 {
    let Some(expires_at) = expires_at else {
        return 0.0;
    };
    let remaining = expires_at.signed_duration_since(now);
    if remaining.num_seconds() <= 0 {
        return 1.0;
    }
    normalized(1.0 - remaining.num_minutes() as f32 / window_minutes as f32)
}

fn dominant_factor(factors: &DeckRankFactors) -> &'static str {
    [
        ("risk", RISK_WEIGHT * factors.risk),
        ("blockedness", BLOCKEDNESS_WEIGHT * factors.blockedness),
        (
            "approval expiry pressure",
            APPROVAL_EXPIRY_PRESSURE_WEIGHT * factors.approval_expiry_pressure,
        ),
        (
            "lease pressure",
            LEASE_PRESSURE_WEIGHT * factors.lease_pressure,
        ),
        (
            "adapter degradation",
            ADAPTER_DEGRADED_WEIGHT * factors.adapter_degraded_weight,
        ),
        (
            "evidence gap",
            EVIDENCE_GAP_WEIGHT * factors.evidence_gap_weight,
        ),
        (
            "user query relevance",
            USER_QUERY_RELEVANCE_WEIGHT * factors.user_query_relevance,
        ),
        ("freshness", FRESHNESS_WEIGHT * factors.freshness),
        (
            "downstream blast radius",
            DOWNSTREAM_BLAST_RADIUS_WEIGHT * factors.downstream_blast_radius,
        ),
    ]
    .into_iter()
    .max_by(|left, right| left.1.total_cmp(&right.1))
    .map(|(factor, _)| factor)
    .unwrap_or("risk")
}

fn explanation(subject: &str, dominant_factor: &str, score: f32) -> String {
    let driver = match dominant_factor {
        "risk" => "risk is highest",
        "blockedness" => "queue progress is blocked",
        "approval expiry pressure" => "the approval window is tight",
        "lease pressure" => "the lease window is tight",
        "adapter degradation" => "adapter health is degraded",
        "evidence gap" => "evidence is missing",
        "user query relevance" => "it matches the current question",
        "freshness" => "the signal is recent",
        "downstream blast radius" => "downstream impact is high",
        _ => "risk is highest",
    };
    format!("{subject} ranks here because {driver}; score {score:.2}.")
}

fn normalized(value: f32) -> f32 {
    if value.is_nan() {
        return 0.0;
    }
    value.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone};

    use super::*;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2025, 1, 1, 12, 0, 0)
            .single()
            .expect("valid test time")
    }

    #[test]
    fn weighted_sum_uses_named_weights() {
        let input = RankInput {
            id: "b".to_owned(),
            subject: "Queue item".to_owned(),
            risk: 1.0,
            blockedness: 0.5,
            approval_expires_at: Some(now() + Duration::minutes(15)),
            lease_expires_at: Some(now() + Duration::minutes(30)),
            adapter_degraded_weight: 0.2,
            evidence_gap_weight: 0.6,
            user_query_relevance: 0.8,
            updated_at: now() - Duration::minutes(30),
            downstream_blast_radius: 0.25,
        };

        let reason = rank_reason(&input, now());

        let expected = 0.25 * 1.0
            + 0.20 * 0.5
            + 0.10 * 0.75
            + 0.10 * 0.5
            + 0.05 * 0.2
            + 0.10 * 0.6
            + 0.05 * 0.8
            + 0.05 * 0.5
            + 0.10 * 0.25;
        assert!((reason.score - expected).abs() < f32::EPSILON);
        assert!(reason.explanation.contains("risk is highest"));
    }

    #[test]
    fn rank_sort_is_score_desc_then_id_asc() {
        let inputs = vec![
            RankInput {
                id: "b".to_owned(),
                subject: "B".to_owned(),
                risk: 0.5,
                blockedness: 0.5,
                approval_expires_at: None,
                lease_expires_at: None,
                adapter_degraded_weight: 0.5,
                evidence_gap_weight: 0.5,
                user_query_relevance: 0.5,
                updated_at: now(),
                downstream_blast_radius: 0.5,
            },
            RankInput {
                id: "a".to_owned(),
                subject: "A".to_owned(),
                risk: 0.5,
                blockedness: 0.5,
                approval_expires_at: None,
                lease_expires_at: None,
                adapter_degraded_weight: 0.5,
                evidence_gap_weight: 0.5,
                user_query_relevance: 0.5,
                updated_at: now(),
                downstream_blast_radius: 0.5,
            },
            RankInput {
                id: "c".to_owned(),
                subject: "C".to_owned(),
                risk: 1.0,
                blockedness: 1.0,
                approval_expires_at: None,
                lease_expires_at: None,
                adapter_degraded_weight: 1.0,
                evidence_gap_weight: 1.0,
                user_query_relevance: 1.0,
                updated_at: now(),
                downstream_blast_radius: 1.0,
            },
        ];

        let ranked = rank_inputs(inputs, now());

        assert_eq!(
            ranked
                .iter()
                .map(|ranked| ranked.input.id.as_str())
                .collect::<Vec<_>>(),
            vec!["c", "a", "b"]
        );
    }

    #[test]
    fn freshness_decays_to_zero_at_one_hour() {
        assert_eq!(freshness_factor(now(), now()), 1.0);
        assert_eq!(freshness_factor(now() - Duration::minutes(30), now()), 0.5);
        assert_eq!(freshness_factor(now() - Duration::minutes(60), now()), 0.0);
        assert_eq!(freshness_factor(now() - Duration::minutes(90), now()), 0.0);
    }
}
