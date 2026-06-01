# JMCP Final Engineering Specification

**Document:** `JMCP_FINAL_ENGINEERING_SPEC.md`  
**Version:** 5.0 final protocol-first architecture  
**Date:** 2026-06-01  
**Canonical product:** **JMCP** - Joint Master Control Plane  
**Canonical protocol/backbone:** **JPCM-1.0.0** - Joint Process Control Messaging  
**Normative schema:** `JPCM_PROTOCOL_V1_SCHEMA.json`  
**Primary stack:** Rust, Tokio, Axum/tonic, NATS JetStream, PostgreSQL/Timescale, graph store, vector/search index, object/CAS evidence store, Vite, TypeScript, React, WebSocket/SSE, WebRTC/voice adapters, OpenTelemetry, OPA/Cedar-style policy, Sigstore/in-toto/SLSA provenance.  
**Primary doctrine:** The user should say less, decide less often, and still have stronger control because JMCP owns authority, evidence, attention, memory governance, and promotion.

---

## 0. Final naming and scope

The earlier corpus used JMCP, JPMC, JPCM, JPCP, and CP. V5 resolves the naming permanently:

- **JMCP** is the product and supervisory control plane.
- **JPCM-1.0.0** is the mandatory protocol and communication backbone used by every service that communicates with JMCP.
- **JMCP Core** is the Rust control-plane runtime that validates JPCM, issues leases, schedules work, evaluates evidence, and decides what reaches the user.
- **Adapters** translate MCP, A2A, CLIs, CI systems, databases, browser agents, voice systems, GitHub APIs, and external agents into JPCM. Adapters are never trust roots.
- **Cells** are bounded execution domains: repo cells, research cells, production cells, sandbox cells, tool-build cells, self-improvement cells, and incident cells.

Any service that cannot produce valid JPCM messages is outside the control plane and must be wrapped, quarantined, or rejected.

---

## 1. Executive definition

**JMCP is an assured supervisory control plane for autonomous engineering, research, code production, knowledge compression, tool building, and self-improvement.** It is the primary text and voice interface for the user, the authority kernel for machines, the observability and fault-detection layer for all work, the evidence gate for promotion, and the attention firewall that protects the user from noise without hiding material risk.

JMCP is modeled like a high-end fab process-control room rather than a chatbot. Sensors report activity, controllers detect faults, schedulers optimize throughput, policies constrain authority, evidence systems verify claims, and the human sees the minimum information required to preserve intent, risk control, and strategic direction.

The strongest version of JMCP is not just an orchestrator. It is a **software fab operating system**: a live digital twin of work, repositories, agents, tools, data, lessons, risk, cost, bottlenecks, proof, memory, and user goals.

---

## 2. What JMCP is

JMCP is:

- The primary user interface for engineering intent through text, voice, and visual drilldown.
- The authority plane that grants, denies, scopes, expires, and revokes capability leases.
- The evidence plane that refuses to promote claims without reproducible receipts.
- The communication backbone that all services use through JPCM-1.0.0.
- The task and workflow kernel that decomposes, schedules, supervises, cancels, retries, rolls back, and closes work.
- The attention firewall that decides what the user should see, when, and at what level of detail.
- The tool and data awareness layer that maintains live service, tool, data, model, policy, cost, and risk inventories.
- The cognition governor that can use Jekko/ZYAL and route to external agents without trusting them.
- The repo and CI supervisor that uses Jeryu to understand code reality, graph relationships, build health, and safe promotion paths.
- The lessons and standards compiler that uses Jankurai to convert evidence-backed experience into reusable controls.
- The self-improvement governor that can improve JMCP itself only through shadow mode, measurable evals, promotion gates, and rollback.
- The automated tool-building governor that can propose, build, qualify, promote, and retire tools under strict supply-chain controls.
- The permanent audit system for user intent, machine action, evidence, decisions, and outcomes.

---

## 3. What JMCP is not

JMCP is not:

- Not a chatbot. Chat is only one user surface.
- Not an MCP host. MCP is an adapter protocol for tools and context, not the authority root.
- Not an A2A peer. A2A agents can collaborate, but JMCP remains the controller.
- Not a CI wrapper. CI is evidence input, not a sufficient promotion proof.
- Not an observability dashboard. Observability is a sensor layer; JMCP acts on it.
- Not an agent framework. Agents are replaceable workers under lease.
- Not a memory bucket. Memory is governed, scoped, evidenced, expiring, and revocable.
- Not a logging system. Logs are low-grade evidence; JMCP needs attestations, traces, tests, and independent checks.
- Not a production deployer by default. Deployment requires risk-tier authority and evidence.
- Not a self-modifying black box. Self-improvement is a governed task class.
- Not a user replacement. It suppresses noise, but it escalates material decisions.

---

## 4. Non-negotiable laws

### 4.1 Authority laws

1. No side effect without a valid lease.
2. No lease without a task, scope, actor, risk tier, expiry, constraints, and revocation path.
3. No adapter is a trust root.
4. No external agent receives ambient authority.
5. No irreversible, production, legal, credential, or expensive action happens without explicit policy authorization and, where required, user approval.
6. Equivalent-effect attacks are forbidden: if an actor cannot delete a database, it also cannot run a migration that empties it.

### 4.2 Evidence laws

1. A claim is not evidence.
2. A log is weak evidence unless signed, correlated, and independently observed.
3. CI is not sufficient when CI can be modified by the same actor that writes the code.
4. Promotion requires an evidence bundle with quality tier matching risk.
5. Every important conclusion must be drillable to source artifacts.
6. Evidence must survive replay and audit.

### 4.3 Communication laws

1. Every service must speak JPCM natively or through a constrained adapter.
2. Every command must be idempotent or guarded by an exactly-once-effect key.
3. Every message must carry identity, tenant, cell, trace, correlation, risk, schema reference, delivery class, payload hash, and signature.
4. Schema evolution must be explicit and conformance-tested.
5. Backpressure is a safety control, not an optimization.
6. Replay must not create duplicate side effects.

### 4.4 User-attention laws

1. The default is silence for safe autonomous work.
2. The user sees summaries, not raw event streams.
3. The user is interrupted only for material uncertainty, approval, risk change, irreversibility, external exposure, or incident conditions.
4. Every user-visible packet must offer drilldown.
5. Voice commands require confidence, replay protection, and confirmation policy proportional to risk.
6. The system must explain why it interrupted the user.

### 4.5 Self-improvement laws

1. JMCP may propose improvements to itself, but may not silently promote them.
2. Self-improvement requires a hypothesis, experiment, baseline, shadow mode, evaluation, rollback, and approval gate.
3. Security controls cannot be weakened by the component they constrain.
4. Memory compression cannot overwrite provenance.
5. Tool building must include build-vs-buy, owner, API, side effects, supply-chain plan, eval plan, promotion, and retirement.

---

## 5. System architecture

JMCP is composed of the following planes. Planes are logical; services may be deployed independently.

### 5.1 Authority Plane

Responsibilities:

- Evaluate policy.
- Issue and revoke capability leases.
- Enforce risk tier and autonomy tier.
- Bind every effect to task, scope, context, actor, and lease.
- Prevent equivalent-effect attacks.
- Maintain approval records.

Core services:

```text
jmcp-authorityd
jmcp-policyd
jmcp-lease-broker
jmcp-approvald
jmcp-identityd
```

### 5.2 JPCM Communication Plane

Responsibilities:

- Validate all envelopes against `JPCM_PROTOCOL_V1_SCHEMA.json`.
- Verify signatures and payload hashes.
- Enforce subject namespace and delivery classes.
- Provide durable replay for audit and recovery.
- Provide request/reply for commands.
- Provide low-latency streams for cockpit and voice.
- Apply backpressure, dead-lettering, quarantine, and replay safety.

Recommended backbone:

```text
NATS JetStream       primary durable event and command backbone
PostgreSQL          materialized state, task tables, approval records
Object/CAS store    immutable evidence bundles and artifacts
Graph store         repo/tool/data/task causality graph
Vector/search       governed memory projections and semantic retrieval
OTLP pipeline       traces, metrics, logs, GenAI spans
WebSocket/SSE       cockpit streams
WebRTC/audio bus    voice input/output with transcript evidence
```

### 5.3 Workflow Plane

Responsibilities:

- Convert intent into work orders and task DAGs.
- Assign tasks by capability, risk, cost, and confidence.
- Manage queue priority and budgets.
- Route to Jekko/ZYAL, Codex, Claude, specialized tools, or deterministic services.
- Cancel, pause, retry, roll back, and quarantine.
- Maintain task state machine.

### 5.4 Evidence Plane

Responsibilities:

- Collect evidence bundles.
- Grade evidence quality.
- Store immutable artifacts.
- Verify signatures, test results, traces, diffs, screenshots, SBOMs, provenance, and human reviews.
- Block promotion when evidence is insufficient.

### 5.5 Attention Plane

Responsibilities:

- Decide what reaches the user.
- Convert complex activity into minimum sufficient context.
- Provide drilldown.
- Support text and voice.
- Prevent alert fatigue.
- Escalate risk changes.

### 5.6 Tool and Data Awareness Plane

Responsibilities:

- Maintain service cards, tool cards, data cards, model cards, and policy cards.
- Know what technologies exist, what each can do, what they cost, what data they touch, and what risks they carry.
- Detect missing capabilities.
- Propose new tools.
- Retire redundant or unsafe tools.
- Build technology radar and build-vs-buy decisions.

### 5.7 Cognition Plane

Responsibilities:

- Use Jekko/ZYAL for structured reasoning and workflows.
- Route to external agents under lease.
- Maintain scratchpads as task-scoped artifacts.
- Prevent chain-of-thought leakage by storing safe rationales, evidence, and decision records rather than raw hidden reasoning.
- Decompose problems, compress knowledge, and generate proposals.

### 5.8 Code/CI/Graph Plane

Responsibilities:

- Use Jeryu to understand repositories, code graphs, ownership, tests, CI, and promotion paths.
- Use CI as evidence, not authority.
- Block dangerous or sloppy changes.
- Detect duplicated code, dead code, brittle tests, security smells, and architectural drift.

### 5.9 Lessons and Standards Plane

Responsibilities:

- Use Jankurai for lessons learned, best practices, audit standards, reusable proofs, and global tools.
- Convert local incidents into scoped lessons.
- Avoid false global generalization.
- Expire lessons when stale.

---

## 6. JPCM-1.0.0 normative protocol

### 6.1 Design goals

JPCM exists to make every action explainable by this tuple:

```text
intent -> work order -> task -> actor -> context contract -> capability lease -> command -> evidence -> decision -> user attention outcome
```

A service is JPCM-compliant only if it can:

1. Register a service card.
2. Declare tool/data/model capabilities.
3. Receive commands through JPCM.
4. Emit observations and evidence through JPCM.
5. Reject commands without valid leases.
6. Use idempotency keys for side effects.
7. Preserve trace/correlation across calls.
8. Pass conformance tests.

### 6.2 Mandatory envelope

Every message must include:

- `jpcm_version`: `1.0.0`
- `message_id`: UUIDv7 or ULID
- `message_family`: intent, command, event, observation, task, lease, evidence, attention, approval, memory, service, tool, data, model, voice, policy, protocol, conformance, audit, or error
- `message_type`: lower-case semantic type
- `source`: service id
- `subject`: routable subject beginning with `jpcm.`
- `issued_at`: RFC 3339 timestamp
- `tenant`, `cell`
- `trace_id`, `correlation_id`, optional `causation_id`
- `actor`
- `authority`
- `risk`
- `data_class`
- `delivery`
- optional `context_contract_ref`, `lease_ref`, `work_order_ref`, `task_ref`
- `schema_ref`
- `payload_hash`
- `signature`
- `payload`

The full schema is in `JPCM_PROTOCOL_V1_SCHEMA.json`.

### 6.3 Subject namespace

```text
jpcm.intent.<tenant>.<cell>
jpcm.command.<target_service>.<capability>
jpcm.event.<domain>.<entity>
jpcm.observation.<service>.<signal>
jpcm.task.<work_order_id>.<task_id>
jpcm.lease.<lease_id>
jpcm.evidence.<task_id>.<bundle_id>
jpcm.attention.<user_or_group>
jpcm.approval.<approval_id>
jpcm.memory.<scope>.<memory_type>
jpcm.service.<service_id>
jpcm.tool.<tool_id>
jpcm.data.<data_id>
jpcm.model.<model_id>
jpcm.voice.<session_id>
jpcm.policy.<policy_id>
jpcm.protocol.<version>
jpcm.conformance.<subject_id>
jpcm.audit.<tenant>.<cell>
jpcm.error.<source>
```

### 6.4 Delivery classes

| Class | Use | Rule |
|---|---|---|
| `telemetry-at-most-once` | high-volume observations | May drop under load; never drives irreversible decisions alone. |
| `event-at-least-once` | state changes and evidence | Consumers must de-duplicate by message id. |
| `command-exactly-once-effect` | side effects | Requires idempotency key and effect ledger. |
| `audit-append-only` | authority, approvals, promotion | Immutable, retained, replayable. |
| `ui-low-latency` | cockpit streams | May coalesce; must preserve final state. |
| `voice-low-latency` | speech sessions | Must bind audio, transcript, confidence, and confirmation policy. |

### 6.5 Versioning

- Major version changes may break compatibility and require protocol-change task.
- Minor versions may add optional fields.
- Patch versions may clarify validation and conformance.
- Services must declare supported versions in service cards.
- Adapters must down-convert only when no authority semantics are lost.
- Unknown required fields must fail closed.

### 6.6 Error model

Errors are JPCM messages, not side channels. Required categories:

- `schema_invalid`
- `signature_invalid`
- `lease_missing`
- `lease_expired`
- `lease_scope_violation`
- `context_contract_violation`
- `policy_denied`
- `risk_tier_exceeded`
- `evidence_insufficient`
- `idempotency_conflict`
- `backpressure`
- `dependency_unavailable`
- `adapter_quarantined`
- `voice_confidence_low`
- `human_approval_required`
- `unknown`

### 6.7 Conformance levels

| Level | Name | Required capability |
|---|---|---|
| C0 | Wrapped | Legacy system behind adapter; no direct authority. |
| C1 | Envelope | Emits valid signed envelopes. |
| C2 | Lease-aware | Rejects commands without leases. |
| C3 | Evidence-aware | Emits evidence bundles and trace correlation. |
| C4 | Replay-safe | Idempotent commands, effect ledger, recovery tests. |
| C5 | Full control participant | Service/tool/data/model cards, leases, evidence, policy, conformance, fault injection passed. |

No production side effect may depend on a service below C3. R4/R5 actions require C4 or C5.

---

## 7. Task management

### 7.1 Work order lifecycle

```text
draft -> accepted -> planned -> active -> verifying -> complete -> archived
                    |          |        |-> awaiting-user
                    |          |        |-> blocked
                    |          |        |-> failed -> rollback -> archived
                    |          |-> canceled
                    |-> rejected
```

### 7.2 Task lifecycle

```text
proposed -> accepted -> planned -> leased -> queued -> running
   -> awaiting-evidence -> verifying -> completed -> archived
   -> awaiting-user
   -> blocked
   -> failed -> rolled-back -> archived
   -> canceled
   -> quarantined
```

### 7.3 Task classes

JPCM-1.0 explicitly supports:

- research and web/paper/source investigation
- knowledge compression and memory proposal
- architecture design
- code change, refactor, bug hunt, security scan, dependency update, test generation
- repo creation, release, deploy, rollback, and incident diagnosis
- data query, data migration, memory read/write proposal
- lesson publication to Jankurai
- policy change and protocol change
- adapter registration and service onboarding
- tool evaluation, tool-build proposal, tool-build execution, tool retirement
- self-improvement proposal, experiment, and promotion
- model routing, agent delegation, browser work, voice interaction
- user approval, cost optimization, observability rule creation, conformance testing, red-team work

### 7.4 Scheduling

The scheduler ranks tasks by:

```text
priority_score = user_value + risk_reduction + deadline_pressure + unblock_value + learning_value
                 - cost_penalty - risk_penalty - uncertainty_penalty - user_attention_penalty
```

The scheduler must keep always-useful background priorities, including:

- Find and triage bugs.
- Reduce redundant code.
- Detect security regressions.
- Compress lessons into scoped memories.
- Improve tests.
- Improve tool inventory.
- Remove dead tools.
- Detect bottlenecks.
- Build missing observability.
- Propose new global tools only when build-vs-buy clears threshold.

---

## 8. Text, voice, and the attention firewall

### 8.1 Interaction model

The user may communicate through:

- Text chat.
- Voice chat.
- UI cockpit commands.
- Approval cards.
- Drilldown queries.
- Strategic goals and standing policies.

JMCP responds with:

- Silent autonomous progress.
- Digest summaries.
- Heads-up notices.
- Decision packets.
- Urgent incident packets.
- Voice confirmations.
- Drilldown reports.

### 8.2 Minimum-disclosure policy

JMCP should share the bare minimum that satisfies user control:

| Situation | User visibility |
|---|---|
| Safe local work, no material risk | Silent or digest |
| Progress on user-requested work | Short status card |
| Ambiguity that changes outcome | Decision packet |
| External side effect | Approval unless pre-authorized |
| Production/security/legal/cost risk | Approval or urgent packet |
| Incident | Immediate summary with options |
| Repeated low-value alerts | Suppress and tune policy |

### 8.3 Voice-specific controls

Voice is high-leverage and high-risk. Requirements:

- Store original audio artifact or approved redacted equivalent.
- Store transcript and redacted transcript.
- Track speaker confidence and intent confidence.
- Confirm ambiguous instructions.
- Confirm any action above R2 unless explicit policy allows otherwise.
- Reject voice commands under replay suspicion.
- Do not expose secrets through speech unless policy permits.
- Use short verbal summaries and send detailed drilldown to UI.

---

## 9. Tool and data awareness

JMCP must know not only what tools exist, but what they are safe to do.

### 9.1 Inventory objects

- **ServiceCard:** runtime identity, endpoints, capabilities, sandbox, schemas, limits, health.
- **ToolCard:** actions, side effects, data access, qualification, known failures, build-vs-buy score.
- **DataCard:** store type, owner, classification, access modes, retention, lineage, quality.
- **ModelCard:** provider, model class, allowed uses, disallowed uses, evals, cost, data residency.
- **PolicyCard:** policy id, scope, version, owner, tests.
- **AdapterCard:** translated protocol, authority limits, conformance level, quarantine state.

### 9.2 Technology radar

JMCP maintains a radar with four rings:

- **Adopt:** proven, qualified, cost-effective.
- **Trial:** promising, limited risk.
- **Assess:** unknown, research needed.
- **Hold:** unsafe, redundant, stale, or too costly.

Automated tool-building enters only after:

1. Existing tool search.
2. Build-vs-buy score.
3. Threat model.
4. Owner assignment.
5. API contract.
6. Sandbox profile.
7. Supply-chain plan.
8. Eval plan.
9. Promotion plan.
10. Retirement plan.

---

## 10. Self-improvement and automated tool building

### 10.1 Self-improvement pipeline

```text
observation -> hypothesis -> proposal -> shadow experiment -> evaluation -> review -> canary -> promotion -> monitoring -> rollback if degraded
```

Self-improvement examples:

- Better routing policy.
- Better task decomposition.
- Better memory retrieval.
- Better evidence grading.
- Better user-attention threshold.
- Better cost control.
- Better conformance tests.
- Better Jankurai lessons.
- Better Jeryu repo graph queries.
- Better Jekko/ZYAL workflows.

Forbidden self-improvement:

- Weakening authority checks.
- Reducing evidence requirements without approval.
- Expanding tool access silently.
- Removing audit logs.
- Hiding user-visible risk.
- Promoting memory without provenance.
- Modifying protocol compatibility without migration.

### 10.2 Automated tool-building pipeline

```text
capability gap -> alternatives search -> build-vs-buy -> spec -> threat model -> implementation in sandbox -> tests -> red-team -> service card -> tool card -> C-level conformance -> shadow use -> promotion -> retirement plan
```

Tool-building tasks must be isolated from production cells. A tool built by an agent cannot be used for production side effects until independent verification passes.

---

## 11. Security model

### 11.1 Threats

JMCP assumes:

- Agents hallucinate.
- Agents optimize for local success.
- Agents can be prompt-injected.
- Tools can be malicious or compromised.
- Logs can lie.
- CI can be faked.
- Memory can be poisoned.
- Voice can be replayed or misunderstood.
- Adapters can launder trust.
- Dependencies can be compromised.
- Users can be overloaded.
- Policies can drift.
- Distributed systems fail partially.

### 11.2 Required controls

- mTLS/SPIFFE-style workload identity for services.
- Signed JPCM messages with payload hashes.
- Capability leases checked at every side-effect boundary.
- Context contracts for allowed and forbidden sources.
- Sandboxing by risk tier: process, container, microVM, wasm, browser isolate, remote runner.
- Secrets brokered only through lease-bound scopes.
- Network deny-by-default with allowlisted egress.
- SBOM, provenance, and signed release artifacts for tools.
- Policy-as-code with tests.
- Quarantine for suspicious adapters, tools, memories, and evidence.
- Independent verification for high-risk promotions.
- Audit append-only event streams.
- Red-team suites for prompt injection, memory poisoning, tool poisoning, voice replay, and false evidence.

---

## 12. Evidence quality tiers

| Tier | Meaning | Use |
|---|---|---|
| E0 | Claim | Never enough for promotion. |
| E1 | Log or transcript | Useful for diagnosis, weak for proof. |
| E2 | Test or local check | Enough for low-risk local changes. |
| E3 | Independent check | Required for meaningful repo changes. |
| E4 | Reproducible attestation | Required for release/prod/security changes. |
| E5 | Formal or human-approved | Required for irreversible, legal, policy, protocol, or critical security changes. |

---

## 13. Frontend architecture

Stack: Vite, TypeScript, React, TanStack Query, Zustand or Jotai for local state, WebSocket/SSE for live state, WebRTC for voice, WASM modules for local visualization where useful.

Main surfaces:

- **Command/voice rail:** user intent entry, transcript, confirmations.
- **Attention inbox:** only decisions and meaningful summaries.
- **System map:** services, tools, data stores, agents, cells, health.
- **Work graph:** work orders, tasks, dependencies, blockers.
- **Evidence drilldown:** tests, logs, traces, diffs, attestations.
- **Risk cockpit:** risk tiers, approvals, leases, policies.
- **Tool/data radar:** available technologies, missing tools, build-vs-buy.
- **Memory browser:** active memories, provenance, expiry, contradictions.
- **Incident view:** timeline, blast radius, options, rollback.

Performance requirements:

- Cockpit visible within 1.5 seconds on normal workstation.
- Live state updates under 250 ms p95 for UI events.
- Voice partial transcript under 500 ms p95.
- Drilldown initial result under 1 second p95 for indexed data.
- No unbounded frontend event lists; use materialized summaries and windowed virtualization.

---

## 14. File tree

```text
jmcp/
  crates/
    jmcp-core/
    jmcp-authority/
    jmcp-protocol/
    jmcp-schema/
    jmcp-scheduler/
    jmcp-evidence/
    jmcp-attention/
    jmcp-memory/
    jmcp-tool-registry/
    jmcp-data-registry/
    jmcp-model-registry/
    jmcp-policy/
    jmcp-voice/
    jmcp-conformance/
    jmcp-adapter-sdk/
  services/
    authorityd/
    jpcm-gateway/
    schedulerd/
    evidenced/
    attentiond/
    memoryd/
    registryd/
    conformance-runner/
    voice-gateway/
    cockpit-api/
  adapters/
    mcp-adapter/
    a2a-adapter/
    jeryu-adapter/
    jankurai-adapter/
    jekko-zyal-adapter/
    github-adapter/
    ci-adapter/
    sql-adapter/
    nosql-adapter/
    browser-adapter/
    codex-adapter/
    claude-adapter/
  frontend/
    apps/cockpit/
    packages/ui/
    packages/jpcm-client/
    packages/voice-client/
    packages/graph-view/
  schemas/
    jpcm/1.0.0/JPCM_PROTOCOL_V1_SCHEMA.json
  policies/
    authority/
    attention/
    promotion/
    memory/
    tool-building/
  tests/
    conformance/
    redteam/
    replay/
    fault-injection/
    e2e/
  deploy/
    docker-compose/
    k8s/
    helm/
    terraform/
  docs/
    architecture/
    protocol/
    operations/
    threat-model/
```

---

## 15. Acceptance criteria

V5 is not done until:

- All services communicate through JPCM-1.0.0 or constrained adapters.
- Schema validation is mandatory at ingress and egress.
- Capability leases are enforced at every side-effect boundary.
- Task state machine is materialized and replayable.
- Evidence bundles are immutable and drillable.
- Voice commands produce audio/transcript evidence and risk-based confirmations.
- Tool/data/model registries are live and queryable.
- Self-improvement and tool-building tasks require promotion gates.
- Frontend attention inbox shows only decision-worthy packets by default.
- Conformance suite can reject non-compliant adapters.
- Red-team suite covers prompt injection, tool poisoning, memory poisoning, voice replay, false evidence, and CI forgery.
- Disaster recovery can replay audit streams without duplicating side effects.

---

## 16. Final ambition vector

JMCP can become:

1. A personal engineering command center.
2. A repo process-control system.
3. A cross-repo standards and lessons compiler.
4. A software fab with autonomous work cells.
5. A proof-carrying agent marketplace where agents compete under evidence requirements.
6. A digital twin of engineering work, tools, risk, cost, and knowledge.
7. A self-hardening organization operating system.
8. A strategic R&D foundry that discovers gaps, builds tools, creates repos, validates designs, and learns safely.
9. A trust fabric where every machine-generated effect has provenance, risk, authority, evidence, and user-intent lineage.

The ambition is extreme, but the path is controlled: **authority first, protocol first, evidence first, attention first.**
