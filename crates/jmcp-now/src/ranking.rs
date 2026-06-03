use chrono::{DateTime, Utc};

use crate::contract::{RankFactorKind, RankFactors, RankReason};

pub const RISK_WEIGHT: f64 = 0.30;
pub const ACTIONABILITY_WEIGHT: f64 = 0.20;
pub const FRESHNESS_WEIGHT: f64 = 0.15;
pub const BLAST_RADIUS_WEIGHT: f64 = 0.20;
pub const LEASE_PRESSURE_WEIGHT: f64 = 0.10;
pub const USER_RELEVANCE_WEIGHT: f64 = 0.05;

const FRESHNESS_WINDOW_MINUTES: i64 = 60;
const LEASE_PRESSURE_WINDOW_MINUTES: i64 = 60;

#[derive(Clone, Debug, PartialEq)]
pub struct RankInput {
    pub id: String,
    pub subject: String,
    pub risk: f64,
    pub actionability: f64,
    pub updated_at: DateTime<Utc>,
    pub blast_radius: f64,
    pub lease_expires_at: Option<DateTime<Utc>>,
    pub user_relevance: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RankedInput {
    pub input: RankInput,
    pub reason: RankReason,
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

pub fn rank_reason(input: &RankInput, now: DateTime<Utc>) -> RankReason {
    let factors = RankFactors {
        risk: normalized(input.risk),
        actionability: normalized(input.actionability),
        freshness: freshness_factor(input.updated_at, now),
        blast_radius: normalized(input.blast_radius),
        lease_pressure: lease_pressure_factor(input.lease_expires_at, now),
        user_relevance: normalized(input.user_relevance),
    };
    let score = weighted_score(factors);
    let dominant_factor = dominant_factor(factors);
    RankReason {
        score,
        factors,
        summary: summary(&input.subject, dominant_factor, score),
        dominant_factor,
    }
}

pub fn weighted_score(factors: RankFactors) -> f64 {
    RISK_WEIGHT * factors.risk
        + ACTIONABILITY_WEIGHT * factors.actionability
        + FRESHNESS_WEIGHT * factors.freshness
        + BLAST_RADIUS_WEIGHT * factors.blast_radius
        + LEASE_PRESSURE_WEIGHT * factors.lease_pressure
        + USER_RELEVANCE_WEIGHT * factors.user_relevance
}

pub fn freshness_factor(updated_at: DateTime<Utc>, now: DateTime<Utc>) -> f64 {
    let age = now.signed_duration_since(updated_at);
    if age.num_seconds() <= 0 {
        return 1.0;
    }
    normalized(1.0 - age.num_minutes() as f64 / FRESHNESS_WINDOW_MINUTES as f64)
}

pub fn lease_pressure_factor(expires_at: Option<DateTime<Utc>>, now: DateTime<Utc>) -> f64 {
    let Some(expires_at) = expires_at else {
        return 0.0;
    };
    let remaining = expires_at.signed_duration_since(now);
    if remaining.num_seconds() <= 0 {
        return 1.0;
    }
    normalized(1.0 - remaining.num_minutes() as f64 / LEASE_PRESSURE_WINDOW_MINUTES as f64)
}

fn dominant_factor(factors: RankFactors) -> RankFactorKind {
    [
        (RankFactorKind::Risk, RISK_WEIGHT * factors.risk),
        (
            RankFactorKind::Actionability,
            ACTIONABILITY_WEIGHT * factors.actionability,
        ),
        (
            RankFactorKind::Freshness,
            FRESHNESS_WEIGHT * factors.freshness,
        ),
        (
            RankFactorKind::BlastRadius,
            BLAST_RADIUS_WEIGHT * factors.blast_radius,
        ),
        (
            RankFactorKind::LeasePressure,
            LEASE_PRESSURE_WEIGHT * factors.lease_pressure,
        ),
        (
            RankFactorKind::UserRelevance,
            USER_RELEVANCE_WEIGHT * factors.user_relevance,
        ),
    ]
    .into_iter()
    .max_by(|left, right| left.1.total_cmp(&right.1))
    .map(|(factor, _)| factor)
    .unwrap_or(RankFactorKind::Risk)
}

fn summary(subject: &str, dominant_factor: RankFactorKind, score: f64) -> String {
    let driver = match dominant_factor {
        RankFactorKind::Risk => "risk is highest",
        RankFactorKind::Actionability => "there is a ready next step",
        RankFactorKind::Freshness => "the signal is recent",
        RankFactorKind::BlastRadius => "downstream impact is high",
        RankFactorKind::LeasePressure => "the lease window is tight",
        RankFactorKind::UserRelevance => "it matches the current question",
    };
    format!("{subject} ranks here because {driver}; score {score:.2}.")
}

fn normalized(value: f64) -> f64 {
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
            actionability: 0.5,
            updated_at: now() - Duration::minutes(30),
            blast_radius: 0.25,
            lease_expires_at: Some(now() + Duration::minutes(30)),
            user_relevance: 0.8,
        };

        let reason = rank_reason(&input, now());

        let expected = 0.30 * 1.0 + 0.20 * 0.5 + 0.15 * 0.5 + 0.20 * 0.25 + 0.10 * 0.5 + 0.05 * 0.8;
        assert_eq!(reason.score, expected);
        assert_eq!(reason.dominant_factor, RankFactorKind::Risk);
    }

    #[test]
    fn rank_sort_is_score_desc_then_id_asc() {
        let inputs = vec![
            RankInput {
                id: "b".to_owned(),
                subject: "B".to_owned(),
                risk: 0.5,
                actionability: 0.5,
                updated_at: now(),
                blast_radius: 0.5,
                lease_expires_at: None,
                user_relevance: 0.5,
            },
            RankInput {
                id: "a".to_owned(),
                subject: "A".to_owned(),
                risk: 0.5,
                actionability: 0.5,
                updated_at: now(),
                blast_radius: 0.5,
                lease_expires_at: None,
                user_relevance: 0.5,
            },
            RankInput {
                id: "c".to_owned(),
                subject: "C".to_owned(),
                risk: 1.0,
                actionability: 1.0,
                updated_at: now(),
                blast_radius: 1.0,
                lease_expires_at: None,
                user_relevance: 1.0,
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
