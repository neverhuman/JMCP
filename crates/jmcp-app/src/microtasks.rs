use std::str::FromStr;

use jcp_core::{Envelope, LocalSigner, Subject};
use jmcp_domain::{
    MicrotaskCard, MicrotaskInputDefaults, MicrotaskKind, MicrotaskOverrides,
    MicrotaskResourceIntent, MicrotaskResourceScope, MicrotaskSafetyDefaults, MicrotaskSubject,
    MicrotaskWorkOrderKind, WorkOrder,
};
use serde_json::{json, Map, Value};

use crate::{autonomous_actions::validate_autonomous_action_id, AppError, AppResult, AppState};

mod input_selection;

use input_selection::{
    default_model_roots, microtask_concept, microtask_model_roots, microtask_repo, optional_label,
};

pub(crate) const MICROTASK_COUNT: u32 = MICROTASKS.len() as u32;

const SUBMITTED_BY: &str = "jmcp.microtask_planner";
const JANKURAI_PROOF_KIND: &str = "jankurai.proof";
const JANKURAI_DIFF_AUDIT_KIND: &str = "jankurai.diff-audit";
const JEKKO_REASON_KIND: &str = "reason";

#[derive(Clone, Copy)]
struct MicrotaskDefinition {
    id: &'static str,
    title: &'static str,
    summary: &'static str,
    kind: &'static str,
    subject: &'static str,
    work_order_kind: &'static str,
    resource: StaticResourceIntent,
    max_stages: u32,
    timeout_secs: u64,
    accepts_repo: bool,
    default_concept: Option<&'static str>,
    accepts_model_root: bool,
}

#[derive(Clone, Copy)]
struct StaticResourceIntent {
    network: MicrotaskResourceScope,
    gpu: MicrotaskResourceScope,
    speech: MicrotaskResourceScope,
    durable_mutation: MicrotaskResourceScope,
    evidence_goal: &'static str,
}

const LOCAL_ONLY: MicrotaskResourceScope = MicrotaskResourceScope::LocalOnly;
const INVENTORY_ONLY: MicrotaskResourceScope = MicrotaskResourceScope::InventoryOnly;
const EVIDENCE_ONLY: MicrotaskResourceScope = MicrotaskResourceScope::EvidenceOnly;
const NO_RESOURCE: MicrotaskResourceScope = MicrotaskResourceScope::None;

const MICROTASKS: &[MicrotaskDefinition] = &[
    MicrotaskDefinition {
        id: "jankurai.repo-refresh-audit",
        title: "Jankurai Repo Refresh Audit",
        summary: "Run a bounded local Jankurai proof pass over the current repository.",
        kind: "jankurai.repo_refresh_audit",
        subject: "jmcp/jankurai/repo-refresh-audit",
        work_order_kind: JANKURAI_PROOF_KIND,
        resource: StaticResourceIntent {
            network: NO_RESOURCE,
            gpu: NO_RESOURCE,
            speech: NO_RESOURCE,
            durable_mutation: NO_RESOURCE,
            evidence_goal: "local repository proof digest",
        },
        max_stages: 2,
        timeout_secs: 600,
        accepts_repo: true,
        default_concept: None,
        accepts_model_root: false,
    },
    MicrotaskDefinition {
        id: "jankurai.changed-path-audit",
        title: "Jankurai Changed Path Audit",
        summary: "Audit changed paths with the local Jankurai diff-audit adapter.",
        kind: "jankurai.changed_path_audit",
        subject: "jmcp/jankurai/changed-path-audit",
        work_order_kind: JANKURAI_DIFF_AUDIT_KIND,
        resource: StaticResourceIntent {
            network: NO_RESOURCE,
            gpu: NO_RESOURCE,
            speech: NO_RESOURCE,
            durable_mutation: NO_RESOURCE,
            evidence_goal: "changed-path digest and review hints",
        },
        max_stages: 2,
        timeout_secs: 600,
        accepts_repo: true,
        default_concept: None,
        accepts_model_root: false,
    },
    MicrotaskDefinition {
        id: "research.concept-scan",
        title: "Research Concept Scan",
        summary: "Queue a guarded concept scan that records evidence goals before any online work.",
        kind: "research.concept_scan",
        subject: "jmcp/jekko/research-concept-scan",
        work_order_kind: JEKKO_REASON_KIND,
        resource: StaticResourceIntent {
            network: EVIDENCE_ONLY,
            gpu: NO_RESOURCE,
            speech: NO_RESOURCE,
            durable_mutation: NO_RESOURCE,
            evidence_goal: "bounded concept evidence map",
        },
        max_stages: 2,
        timeout_secs: 900,
        accepts_repo: false,
        default_concept: Some("JMCP/JCP/JPCM microtask queue design"),
        accepts_model_root: false,
    },
    MicrotaskDefinition {
        id: "router.tool-build-probe",
        title: "Router Tool Build Probe",
        summary:
            "Probe jnoccio-router tool-building readiness without installing or mutating tools.",
        kind: "router.tool_build_probe",
        subject: "jmcp/jekko/router-tool-build-probe",
        work_order_kind: JEKKO_REASON_KIND,
        resource: StaticResourceIntent {
            network: LOCAL_ONLY,
            gpu: NO_RESOURCE,
            speech: NO_RESOURCE,
            durable_mutation: NO_RESOURCE,
            evidence_goal: "local router capability inventory",
        },
        max_stages: 2,
        timeout_secs: 900,
        accepts_repo: true,
        default_concept: None,
        accepts_model_root: false,
    },
    MicrotaskDefinition {
        id: "router.open-model-reasoning-survey",
        title: "Open Model Reasoning Survey",
        summary: "Survey open model reasoning options as bounded evidence, not a download plan.",
        kind: "router.open_model_reasoning_survey",
        subject: "jmcp/jekko/open-model-reasoning-survey",
        work_order_kind: JEKKO_REASON_KIND,
        resource: StaticResourceIntent {
            network: EVIDENCE_ONLY,
            gpu: INVENTORY_ONLY,
            speech: NO_RESOURCE,
            durable_mutation: NO_RESOURCE,
            evidence_goal: "candidate model fit and risk notes",
        },
        max_stages: 2,
        timeout_secs: 900,
        accepts_repo: false,
        default_concept: Some("20B-30B local reasoning models for JMCP evidence tasks"),
        accepts_model_root: true,
    },
    MicrotaskDefinition {
        id: "local-model.inventory-20b-30b",
        title: "Local Model Inventory 20B-30B",
        summary: "Inventory local 20B-30B model readiness without loading weights or using GPU.",
        kind: "local_model.inventory_20b_30b",
        subject: "jmcp/jekko/local-model-inventory",
        work_order_kind: JEKKO_REASON_KIND,
        resource: StaticResourceIntent {
            network: NO_RESOURCE,
            gpu: INVENTORY_ONLY,
            speech: NO_RESOURCE,
            durable_mutation: NO_RESOURCE,
            evidence_goal: "local model root and GPU capacity inventory",
        },
        max_stages: 1,
        timeout_secs: 600,
        accepts_repo: false,
        default_concept: None,
        accepts_model_root: true,
    },
    MicrotaskDefinition {
        id: "local-speech.inventory-asr-tts",
        title: "Local Speech Inventory ASR/TTS",
        summary: "Inventory local ASR and TTS support without installing models or emitting audio.",
        kind: "local_speech.inventory_asr_tts",
        subject: "jmcp/jekko/local-speech-inventory",
        work_order_kind: JEKKO_REASON_KIND,
        resource: StaticResourceIntent {
            network: NO_RESOURCE,
            gpu: INVENTORY_ONLY,
            speech: INVENTORY_ONLY,
            durable_mutation: NO_RESOURCE,
            evidence_goal: "local ASR/TTS capability inventory",
        },
        max_stages: 1,
        timeout_secs: 600,
        accepts_repo: false,
        default_concept: None,
        accepts_model_root: false,
    },
];

impl AppState {
    pub fn list_microtasks(&self) -> AppResult<Vec<MicrotaskCard>> {
        Ok(MICROTASKS.iter().map(microtask_card).collect())
    }

    pub fn submit_microtask(
        &self,
        id: &str,
        overrides: MicrotaskOverrides,
    ) -> AppResult<WorkOrder> {
        let definition = microtask_by_id(id)?;
        let signer = LocalSigner::load_or_create_default()?;
        let envelope = signed_microtask_envelope(definition, overrides, None, &signer)?;
        self.submit_envelope(envelope)
    }

    pub fn queue_autonomous_action_microtasks(
        &self,
        action_id: &str,
        overrides: MicrotaskOverrides,
    ) -> AppResult<Vec<WorkOrder>> {
        validate_autonomous_action_id(action_id)?;
        let signer = LocalSigner::load_or_create_default()?;
        let envelopes = MICROTASKS
            .iter()
            .map(|definition| {
                signed_microtask_envelope(definition, overrides.clone(), Some(action_id), &signer)
            })
            .collect::<AppResult<Vec<_>>>()?;

        envelopes
            .into_iter()
            .map(|envelope| self.submit_envelope(envelope))
            .collect()
    }
}

pub(crate) fn local_model_roots() -> Vec<String> {
    if let Ok(configured) = std::env::var("JMCP_LOCAL_MODEL_ROOTS") {
        let roots = parse_model_roots(&configured);
        if !roots.is_empty() {
            return roots;
        }
    }

    let mut roots = Vec::new();
    if let Ok(home) = std::env::var("HOME") {
        push_unique(&mut roots, format!("{home}/.ollama/models"));
        push_unique(&mut roots, format!("{home}/.cache/huggingface"));
    }
    for root in ["/opt/models", "/mnt/models", "/models"] {
        push_unique(&mut roots, root.to_owned());
    }
    roots
}

fn parse_model_roots(value: &str) -> Vec<String> {
    let mut roots = Vec::new();
    for root in value.split([',', ';']) {
        let trimmed = root.trim();
        if !trimmed.is_empty() {
            push_unique(&mut roots, trimmed.to_owned());
        }
    }
    roots
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn microtask_by_id(id: &str) -> AppResult<&'static MicrotaskDefinition> {
    MICROTASKS
        .iter()
        .find(|microtask| microtask.id == id)
        .ok_or_else(|| AppError::State(format!("unknown microtask: {id}")))
}

fn microtask_card(definition: &MicrotaskDefinition) -> MicrotaskCard {
    MicrotaskCard {
        id: definition.id.to_owned(),
        title: definition.title.to_owned(),
        summary: definition.summary.to_owned(),
        kind: MicrotaskKind(definition.kind.to_owned()),
        subject: MicrotaskSubject(definition.subject.to_owned()),
        work_order_kind: MicrotaskWorkOrderKind(definition.work_order_kind.to_owned()),
        resource_intent: resource_intent(definition),
        safety: safety_defaults(definition),
        inputs: input_defaults(definition),
    }
}

fn input_defaults(definition: &MicrotaskDefinition) -> MicrotaskInputDefaults {
    MicrotaskInputDefaults {
        repo: definition.accepts_repo.then(|| ".".to_owned()),
        concept: definition.default_concept.map(str::to_owned),
        model_roots: default_model_roots(definition),
    }
}

fn safety_defaults(definition: &MicrotaskDefinition) -> MicrotaskSafetyDefaults {
    MicrotaskSafetyDefaults {
        evidence_oriented: true,
        live: false,
        allow_network: false,
        allow_gpu: false,
        allow_external_durable_mutation: false,
        max_stages: definition.max_stages,
        timeout_secs: definition.timeout_secs,
        submitted_by: SUBMITTED_BY.to_owned(),
    }
}

fn resource_intent(definition: &MicrotaskDefinition) -> MicrotaskResourceIntent {
    MicrotaskResourceIntent {
        network: definition.resource.network,
        gpu: definition.resource.gpu,
        speech: definition.resource.speech,
        durable_mutation: definition.resource.durable_mutation,
        evidence_goal: definition.resource.evidence_goal.to_owned(),
    }
}

fn signed_microtask_envelope(
    definition: &MicrotaskDefinition,
    overrides: MicrotaskOverrides,
    parent_action_id: Option<&str>,
    signer: &LocalSigner,
) -> AppResult<Envelope> {
    let payload = microtask_payload(definition, overrides, parent_action_id)?;
    let subject = Subject::from_str(definition.subject)?;
    Ok(signer.sign(Envelope::new(subject, definition.work_order_kind, payload)))
}

fn microtask_payload(
    definition: &MicrotaskDefinition,
    overrides: MicrotaskOverrides,
    parent_action_id: Option<&str>,
) -> AppResult<Value> {
    reject_guarded_true(overrides.live, "live")?;
    reject_guarded_true(overrides.allow_network, "allowNetwork")?;
    reject_guarded_true(overrides.allow_gpu, "allowGpu")?;
    reject_guarded_true(
        overrides.allow_external_durable_mutation,
        "allowExternalDurableMutation",
    )?;

    let max_stages = match bounded_u32(overrides.max_stages, definition.max_stages, "maxStages")? {
        Some(value) => value,
        None => definition.max_stages,
    };
    let timeout_secs = match bounded_u64(
        overrides.timeout_secs,
        definition.timeout_secs,
        "timeoutSecs",
    )? {
        Some(value) => value,
        None => definition.timeout_secs,
    };
    let repo = microtask_repo(definition, overrides.repo);
    let concept = microtask_concept(definition, overrides.concept);
    let model_roots = microtask_model_roots(definition, overrides.model_root);

    let mut inputs = Map::new();
    if let Some(repo) = &repo {
        inputs.insert("repo".to_owned(), json!(repo));
    }
    if let Some(concept) = &concept {
        inputs.insert("concept".to_owned(), json!(concept));
    }
    if !model_roots.is_empty() {
        inputs.insert("model_roots".to_owned(), json!(model_roots));
    }

    let mut payload = json!({
        "prompt": prompt_for(definition, repo.as_deref(), concept.as_deref(), &model_roots),
        "live": false,
        "evidence_oriented": true,
        "allow_network": false,
        "allow_gpu": false,
        "allow_external_durable_mutation": false,
        "max_stages": max_stages,
        "timeout_secs": timeout_secs,
        "resource_intent": resource_intent(definition),
        "inputs": inputs,
        "metadata": {
            "microtask": true,
            "microtask_id": definition.id,
            "microtask_kind": definition.kind,
            "parent_action_id": parent_action_id,
            "submitted_by": SUBMITTED_BY,
            "safety": safety_defaults(definition)
        }
    });

    let run_id = match overrides.run_id {
        Some(run_id) => run_id,
        None => format!("jmcp-microtask-{}", definition.id),
    };
    payload["run_id"] = json!(run_id);
    if let Some(repo) = repo {
        payload["cwd"] = json!(repo);
    }
    if let Some(metadata) = overrides.metadata {
        payload["metadata"]["overrides"] = metadata;
    }
    Ok(payload)
}

fn prompt_for(
    definition: &MicrotaskDefinition,
    repo: Option<&str>,
    concept: Option<&str>,
    model_roots: &[String],
) -> String {
    match definition.id {
        "jankurai.repo-refresh-audit" => format!(
            "Run a bounded local Jankurai proof audit for {}. Record only evidence digests.",
            optional_label(repo, ".")
        ),
        "jankurai.changed-path-audit" => format!(
            "Run a bounded local Jankurai changed-path audit for {}. Record only evidence digests.",
            optional_label(repo, ".")
        ),
        "research.concept-scan" => format!(
            "Produce an evidence-only concept scan for `{}`. Do not access the network, install tools, mutate durable state, or decide approvals.",
            optional_label(concept, "JMCP")
        ),
        "router.tool-build-probe" => format!(
            "Probe local router tool-building readiness for {}. Do not install tools, call external endpoints, or mutate durable state.",
            optional_label(repo, ".")
        ),
        "router.open-model-reasoning-survey" => format!(
            "Survey open reasoning model candidates for `{}` as bounded evidence. Do not download models or use GPU.",
            optional_label(concept, "JMCP local reasoning")
        ),
        "local-model.inventory-20b-30b" => format!(
            "Inventory local 20B-30B model readiness under [{}]. Do not load weights, use GPU, install runtimes, or call the network.",
            model_roots.join(", ")
        ),
        "local-speech.inventory-asr-tts" => {
            "Inventory local ASR and TTS readiness. Do not install models, emit audio, call the network, or mutate durable state.".to_owned()
        }
        _ => definition.summary.to_owned(),
    }
}

fn reject_guarded_true(value: Option<bool>, field: &str) -> AppResult<()> {
    if value == Some(true) {
        return Err(AppError::State(format!(
            "microtask override {field}=true requires a guarded payload policy"
        )));
    }
    Ok(())
}

fn bounded_u32(value: Option<u32>, cap: u32, field: &str) -> AppResult<Option<u32>> {
    match value {
        Some(value) if value == 0 || value > cap => Err(AppError::State(format!(
            "microtask override {field} must be between 1 and {cap}"
        ))),
        other => Ok(other),
    }
}

fn bounded_u64(value: Option<u64>, cap: u64, field: &str) -> AppResult<Option<u64>> {
    match value {
        Some(value) if value == 0 || value > cap => Err(AppError::State(format!(
            "microtask override {field} must be between 1 and {cap}"
        ))),
        other => Ok(other),
    }
}
