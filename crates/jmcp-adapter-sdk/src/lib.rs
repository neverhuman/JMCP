use anyhow::Result;
use async_trait::async_trait;
use jmcp_domain::{Evidence, Lease, ServiceCard, WorkOrder};

#[async_trait]
pub trait Adapter: Send + Sync {
    fn service_card(&self) -> ServiceCard;
    async fn execute(&self, work_order: &WorkOrder) -> Result<Vec<Evidence>>;
}

/// An AGENT-FRIENDLY typed adapter error.
///
/// Unlike a bare `anyhow::anyhow!`, this carries the explicit, machine- and
/// agent-readable context an autonomous worker needs to recover without
/// guessing: *what* was being attempted ([`purpose`](Self::purpose)), *why* it
/// failed ([`reason`](Self::reason)), a short list of [`common_fixes`] to try,
/// a [`docs_url`] for the full runbook, and a concrete [`repair_hint`] naming
/// where to rerun once the fix is applied.
///
/// It implements [`std::error::Error`] and [`std::fmt::Display`] by hand (no
/// extra dependency), and converts into [`anyhow::Error`] via `?`/`.into()` so
/// existing fail-closed call sites keep compiling unchanged.
///
/// [`common_fixes`]: Self::common_fixes
/// [`docs_url`]: Self::docs_url
/// [`repair_hint`]: Self::repair_hint
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdapterError {
    /// What the adapter was trying to accomplish when it failed.
    pub purpose: String,
    /// Why it failed, in human-readable terms.
    pub reason: String,
    /// Ordered list of concrete things an operator/agent can try.
    pub common_fixes: Vec<&'static str>,
    /// URL of the runbook / docs for this failure class.
    pub docs_url: &'static str,
    /// Concrete next step naming exactly where to rerun once fixed.
    pub repair_hint: String,
}

impl std::fmt::Display for AdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // One self-contained, agent-parseable line per field so the next agent
        // knows what was attempted, why it failed, how to fix it, and where to
        // rerun -- without needing the source.
        write!(
            f,
            "adapter error: purpose={}; reason={}",
            self.purpose, self.reason
        )?;
        if !self.common_fixes.is_empty() {
            write!(f, "; common_fixes=[{}]", self.common_fixes.join(", "))?;
        }
        write!(f, "; docs_url={}", self.docs_url)?;
        write!(f, "; repair_hint={}", self.repair_hint)
    }
}

impl std::error::Error for AdapterError {}

impl AdapterError {
    /// Build the fail-closed (adapter not configured) error for `service`.
    ///
    /// Centralizes the agent-friendly context so every adapter's fail-closed
    /// path produces the same recoverable shape.
    pub fn fail_closed(service: &str) -> Self {
        Self {
            purpose: format!("execute a work order via the {service} adapter"),
            reason: format!(
                "the {service} adapter is not configured for this work order; failing closed"
            ),
            common_fixes: vec![
                "confirm the work-order kind/subject is one this adapter handles",
                "set the adapter's required environment (e.g. *_BASE_URL / *_API_KEY)",
                "route the work order to the adapter that owns its subject",
            ],
            docs_url: "https://docs.jmcp.dev/adapters/fail-closed",
            repair_hint: format!(
                "fix the configuration/routing above, then re-run the work order against the {service} adapter"
            ),
        }
    }
}

/// Construct the agent-friendly fail-closed error for `service` as an
/// [`anyhow::Error`], so existing `Err(fail_closed("..."))` call sites in the
/// adapters keep compiling unchanged.
pub fn fail_closed(service: &str) -> anyhow::Error {
    AdapterError::fail_closed(service).into()
}

pub fn require_valid_lease(work_order: &WorkOrder, lease: &Lease, holder: &str) -> Result<()> {
    lease.validate_for(work_order.id, holder)?;
    Ok(())
}

pub async fn execute_with_lease<A>(
    adapter: &A,
    work_order: &WorkOrder,
    lease: &Lease,
    holder: &str,
) -> Result<Vec<Evidence>>
where
    A: Adapter,
{
    require_valid_lease(work_order, lease, holder)?;
    adapter.execute(work_order).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use jmcp_domain::Lease;
    use serde_json::json;

    #[test]
    fn rejects_missing_matching_lease_before_adapter_side_effects() {
        let work_order = WorkOrder::submit("t/s/e", "demo", json!({}));
        let lease = Lease {
            work_order_id: work_order.id,
            holder: "other".to_owned(),
            expires_at: Utc::now() + Duration::minutes(5),
        };

        assert!(require_valid_lease(&work_order, &lease, "adapter").is_err());
    }

    #[test]
    fn adapter_error_display_carries_agent_context() {
        let err = AdapterError::fail_closed("jeryu");
        let rendered = err.to_string();
        // The Display must surface every recovery field so the next agent can
        // act without reading the source.
        assert!(
            rendered.contains(&err.purpose),
            "purpose missing: {rendered}"
        );
        assert!(rendered.contains(&err.reason), "reason missing: {rendered}");
        assert!(
            rendered.contains(&err.repair_hint),
            "repair_hint missing: {rendered}"
        );
        assert!(
            rendered.contains(err.docs_url),
            "docs_url missing: {rendered}"
        );
        for fix in &err.common_fixes {
            assert!(
                rendered.contains(fix),
                "common_fix `{fix}` missing: {rendered}"
            );
        }
        assert!(rendered.contains("jeryu"));
    }

    #[test]
    fn fail_closed_preserves_typed_error_through_anyhow() {
        let err = fail_closed("jekko");
        // The agent-friendly typed error survives the `.into()` into anyhow.
        let typed = err
            .downcast_ref::<AdapterError>()
            .expect("fail_closed yields a typed AdapterError");
        assert!(typed.purpose.contains("jekko"));
        assert!(!typed.common_fixes.is_empty());
        assert!(typed.repair_hint.contains("re-run"));
        // And the rendered anyhow chain still shows the recovery context.
        let rendered = err.to_string();
        assert!(rendered.contains("docs_url="));
        assert!(rendered.contains("repair_hint="));
    }
}
