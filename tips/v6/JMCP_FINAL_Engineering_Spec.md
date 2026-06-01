# JMCP Final Engineering Specification

**Document:** `JMCP_FINAL_Engineering_Spec.md`  
**Canonical product:** **JMCP - Joint Master Control Plane**  
**Canonical wire protocol:** **JCP/1.0.0 - Joint Control Protocol**  
**Canonical backbone profile:** **JPCM - Joint Control Plane Messaging**, the event/messaging implementation profile that carries JCP envelopes.  
**Status:** Final critique-hardened build specification  
**Date:** 2026-06-01  
**Primary stack:** Rust, Tokio, Axum, tonic/gRPC, NATS JetStream, optional Redpanda/Kafka profile, PostgreSQL/Timescale, object/CAS store, graph store, vector/search index, OpenTelemetry, Vite, TypeScript, React, Web Workers, WebSocket/SSE, WebRTC voice, signed schemas, signed service cards, and policy-as-code.

---

## 0. Final naming decision

The prior drafts used JMCP, JPMC, JPCM, JPCP, and JCP. This must stop before implementation.

- **JMCP** is the system: the Joint Master Control Plane.
- **JCP/1.0.0** is the protocol: the Joint Control Protocol. All services, agents, tools, data adapters, user interfaces, voice gateways, and workflow workers must speak it.
- **JPCM** is the default communication backbone profile: the durable event, request/reply, replay, and stream fabric that transports JCP envelopes.
- **Jankurai** is the standards, lessons-learned, global tool, and cross-repo practice refinery.
- **Jeryu** is the code, CI, git, graph-knowledge, and production-promotion safety system.
- **Jekko/ZYAL** is the cognition and workflow scripting substrate used when JMCP needs smarter planning, memory, or agent workflows.

Any future document that changes these names must be treated as an incompatible governance proposal, not a casual edit.

---

## 1. Executive definition

JMCP is the primary authority, observability, task, evidence, tool-awareness, data-awareness, memory, and user-attention control plane for autonomous engineering and high-value knowledge work.

It is not merely a chatbot, dashboard, MCP host, A2A peer, CI wrapper, task queue, or agent framework. It is the system that decides:

1. what work exists;
2. what risk class the work belongs to;
3. what authority is required;
4. what data and tools are available;
5. what worker or agent may be delegated to;
6. what evidence is required;
7. what must be shown to the user;
8. what can proceed silently;
9. what must be blocked, quarantined, escalated, or rolled back;
10. what lessons should be retained or promoted into shared standards.

JMCP's defining doctrine is:

> **Intent is human-sparse; authority is explicit; execution is delegated; evidence is mandatory; memory is governed; communication is typed; user attention is sacred.**

---

## 2. What JMCP is

JMCP is:

- **A supervisory control plane** for software and knowledge work.
- **A process-control kernel** for autonomous engineering, comparable in spirit to fab-level control and fault detection rather than simple automation.
- **A user-facing command surface** that supports text and voice while protecting the user from operational noise.
- **A task authority** that decomposes user intent into work orders, tasks, leases, commands, evidence requirements, and promotion gates.
- **An evidence authority** that refuses to promote work without proof.
- **A tool and data awareness engine** that knows what tools, models, datastores, services, repositories, tests, costs, and capabilities exist.
- **A technology scout** that identifies missing tools and proposes, builds, quarantines, evaluates, and promotes new tools when the system would become stronger.
- **A learning refinery** that turns repeated failures into standards, tests, Jankurai lessons, prompts, policies, and reusable tools.
- **A safety governor** over Jekko/ZYAL, Jeryu, Jankurai, external agents, MCP servers, A2A agents, CI systems, databases, and deployment surfaces.
- **A minimal-disclosure interface** that tells the user the smallest useful truth, while never hiding irreversible, dangerous, costly, security-sensitive, or value-defining decisions.

---

## 3. What JMCP is not

JMCP is not:

- A chatbot with tools.
- A generic agent framework.
- An MCP server.
- An A2A agent.
- A CI/CD replacement.
- A log viewer.
- A code-search tool.
- A vector-memory demo.
- A planner that trusts its own plans.
- A system that allows agents to self-authorize.
- A system that treats passing tests as sufficient proof.
- A system that hides important decisions in the name of saving attention.
- A self-modifying authority kernel.

JMCP may use chat, agents, MCP, A2A, CI, observability, vector memory, and code tools, but it must remain above them as the authority-and-evidence layer.

---

## 4. Non-negotiable laws

### 4.1 Authority laws

1. No side effect without a valid capability lease.
2. No ambient authority.
3. No worker, model, tool, or service may grant itself authority.
4. Every lease is scoped, signed, time-limited, risk-limited, budget-limited, and revocable.
5. Every side-effect boundary must re-check the lease.
6. Break-glass authority must be explicit, user-visible, auditable, and time-boxed.
7. Authority policy changes require governance workflow, evidence, review, and rollback.

### 4.2 Protocol laws

1. Every service must communicate through JCP/1.0.0 envelopes.
2. Raw side channels are non-conformant.
3. Every envelope must include producer identity, trace context, policy epoch, authority context, data classification, payload hash, and signature.
4. All command handling must be idempotent.
5. Schema compatibility must be negotiated; unknown critical fields fail closed.
6. A service whose ServiceCard is unsigned, stale, or revoked is untrusted.

### 4.3 Evidence laws

1. No proof, no promotion.
2. Evidence must be linked to claims.
3. Negative evidence must be preserved.
4. Evidence must include provenance, reproducibility, logs, and scope.
5. Evidence from the executor alone is insufficient for high-risk work.
6. Conflicting evidence blocks promotion until resolved or explicitly accepted by the user.

### 4.4 Memory laws

1. Nothing enters durable memory directly from an agent output.
2. All durable memory starts as a MemoryProposal.
3. Memory must include source, confidence, scope, retention, contradiction checks, and decay policy.
4. Lessons learned must be validated before promotion into Jankurai.
5. Memory cannot expand authority.

### 4.5 User-attention laws

1. The user should see the minimum useful packet, not raw operational flood.
2. JMCP must notify or ask when risk, cost, security, privacy, public exposure, legal exposure, irreversible action, or strategy changes cross thresholds.
3. Quiet operation is a privilege that must be tested through false-quiet checks.
4. The UI must always allow drill-down from summary to evidence.
5. Voice commands require transcript, confidence, replay protection, and confirmation policy.

### 4.6 Self-improvement laws

1. JMCP may improve workflows, policies, tools, prompts, tests, and memory, but may not silently alter the authority kernel.
2. Self-improvement changes require proposals, sandboxing, shadow mode, benchmark gates, red-team tests, promotion decisions, and rollback plans.
3. Automated tool building is a first-class workflow, not an ad hoc code generation path.
4. No self-improvement may reduce evidence requirements for its own promotion.

---

## 5. Reference architecture

JMCP is built from explicit planes.

```
User text/voice/API
        |
        v
User Attention Firewall
        |
        v
Authority Core ---- Policy Engine ---- Identity/Attestation
        |
        v
Workflow Kernel ---- Scheduler ---- Budget Governor
        |
        v
JCP/JPCM Backbone
        |
        +-- Jeryu adapter: code, CI, git, graph knowledge, production gates
        +-- Jankurai adapter: lessons, standards, reusable global tools
        +-- Jekko/ZYAL adapter: cognitive workflows and agent scripts
        +-- MCP adapter: tool/context servers
        +-- A2A adapter: external agent delegation
        +-- Data adapters: SQL, NoSQL, graph, vector, object stores
        +-- UI/voice adapters: React cockpit, WebRTC, transcripts
        |
        v
Evidence, Memory, Observability, Fault Detection
```

### 5.1 Authority Core

The Authority Core owns leases, policies, risk tiers, autonomy tiers, user approval requirements, and vetoes. It is the only component allowed to issue capability leases.

### 5.2 Workflow Kernel

The Workflow Kernel owns intents, work orders, task DAGs, scheduling, retry, cancellation, quarantine, rollback, and promotion.

### 5.3 Evidence Kernel

The Evidence Kernel owns evidence bundles, claim-evidence mapping, sufficiency assessment, provenance, reproducibility, evidence conflict detection, and promotion readiness.

### 5.4 Communication Kernel

The Communication Kernel validates JCP envelopes, schema versions, service cards, signatures, sequence constraints, idempotency keys, delivery classes, and dead-letter behavior.

### 5.5 Tool and Data Awareness Engine

This engine maintains the live catalog of tools, services, agents, models, data stores, schemas, cost profiles, test coverage, risk classes, and capability overlaps.

### 5.6 User Attention Firewall

The Attention Firewall decides what the user sees, when, and in what form. It produces decision packets, summaries, voice confirmations, and drill-down links.

### 5.7 Learning Refinery

The Learning Refinery turns incidents, repeated bugs, failed tasks, user corrections, and successful patterns into candidate memories, tests, policies, tools, and Jankurai lessons.

---

## 6. JCP/1.0.0 protocol standard

### 6.1 Purpose

JCP/1.0.0 is the mandatory communication protocol for all participants in the JMCP system. It is not merely an event format; it is the authority, evidence, task, and attention contract.

A conforming JCP participant must support:

- signed ServiceCards;
- schema validation;
- envelope validation;
- idempotent command handling;
- capability lease checks;
- policy epoch checks;
- trace propagation;
- evidence references;
- data classification;
- revocation and quarantine handling;
- conformance reports.

### 6.2 Envelope

Every message uses the final JSON schema `JCP_1_0_0_Protocol.schema.json`. Required envelope fields are:

- `jcp_version`
- `kind`
- `message_id`
- `message_type`
- `producer`
- `subject`
- `time`
- `trace`
- `authority`
- `policy`
- `data`
- `payload_hash`
- `payload`
- `signature`

Optional but recommended fields include ordering, delivery, risk, attention, evidence references, attachments, redaction, encryption, and extension fields.

### 6.3 Message families

JCP/1.0.0 defines these families:

| Family | Purpose |
|---|---|
| `intent` | User, API, scheduled, or system intent intake. |
| `work_order` | High-level work orchestration. |
| `task` | Task DAG creation, state transitions, blocking, quarantine, promotion, rollback. |
| `command` | Side-effecting or read-only command request/result. |
| `lease` | Capability request, issue, renew, revoke, expire, violation. |
| `context` | Context contract request/grant/expiration/redaction. |
| `tool` | Tool card publication and tool call lifecycle. |
| `data` | Data card publication, query, lineage, access denial. |
| `agent` | Agent cards, delegation, results. |
| `evidence` | Evidence append, rejection, sufficiency, conflict. |
| `policy` | Policy decisions, epochs, exceptions. |
| `promotion` | Promotion request, approval, rejection, revocation. |
| `attention` | User-facing packet, escalation, resolution, false-quiet alert. |
| `voice` | Voice segment, transcript, command candidate, confirmation. |
| `memory` | Memory proposal, acceptance, rejection, decay, contradiction. |
| `self_improvement` | Self-improvement proposal, shadow, evaluation, promotion, rollback. |
| `tool_build` | Automated tool proposal, specification, implementation, testing, quarantine, promotion. |
| `fault` | Fault detection, triage, mitigation, postmortem, watchdog veto. |
| `telemetry` | Metrics, SLO breach, cost breach. |
| `service` | Service cards, heartbeat, health, degradation, decommissioning. |
| `audit` | Append-only audit records. |
| `conformance` | Conformance test reports. |
| `governance` | Protocol freeze, votes, major changes. |

### 6.4 Delivery classes

| Class | Behavior | Example |
|---|---|---|
| `ephemeral` | May be dropped. | Live UI cursor position. |
| `durable_at_least_once` | Stored, replayable, idempotent consumer required. | Task events. |
| `durable_ordered` | Per-subject ordering required. | Task state transitions. |
| `command_requires_ack` | Command must receive accepted/denied/result or timeout. | Tool call. |
| `audit_append_only` | Immutable audit stream. | Lease issuance, promotion decision. |

Exactly-once delivery is not assumed. Correctness comes from idempotency keys, sequence checks, leases, and replay.

### 6.5 Transport profiles

- **JPCM.NATS.JetStream:** default durable event backbone.
- **JCP.gRPC:** low-latency service-to-service request/reply and streaming.
- **JCP.HTTP:** compatibility and simple adapters.
- **JCP.WebSocket/SSE:** cockpit updates and interactive surfaces.
- **JCP.WebRTC.Voice:** voice media and low-latency transcripts; control messages still use signed JCP envelopes.
- **JCP.ObjectPointer:** large artifacts stored in CAS/object store and referenced by hash.

### 6.6 Topic taxonomy

Recommended JPCM subjects:

```
jcp.v1.<tenant>.<cell>.intent.*
jcp.v1.<tenant>.<cell>.work_order.*
jcp.v1.<tenant>.<cell>.task.<task_id>.*
jcp.v1.<tenant>.<cell>.lease.*
jcp.v1.<tenant>.<cell>.command.<target>.*
jcp.v1.<tenant>.<cell>.tool.<tool_id>.*
jcp.v1.<tenant>.<cell>.data.<data_id>.*
jcp.v1.<tenant>.<cell>.evidence.<task_id>.*
jcp.v1.<tenant>.<cell>.attention.<user_id>.*
jcp.v1.<tenant>.<cell>.voice.<session_id>.*
jcp.v1.<tenant>.<cell>.memory.*
jcp.v1.<tenant>.<cell>.self_improvement.*
jcp.v1.<tenant>.<cell>.tool_build.*
jcp.v1.<tenant>.<cell>.fault.*
jcp.v1.<tenant>.<cell>.service.<service_id>.*
jcp.v1.<tenant>.<cell>.audit.*
jcp.v1.<tenant>.<cell>.conformance.*
```

### 6.7 Versioning

- Patch versions may add optional non-critical fields.
- Minor versions may add message types or payload definitions.
- Major versions may change required semantics.
- Unknown critical fields fail closed.
- Services must publish supported versions in their ServiceCard.
- A protocol upgrade requires a governance work order, conformance suite update, migration plan, replay test, and rollback plan.

---

## 7. ServiceCard standard

Every participant must publish a signed ServiceCard before it can receive authority. The ServiceCard must include:

- service identity;
- version;
- owner;
- runtime;
- endpoints;
- supported transport profiles;
- supported JCP versions;
- capabilities;
- data classes requested;
- sandbox class;
- egress policy;
- cost model;
- SLOs;
- SBOM reference;
- provenance/attestation references;
- conformance level;
- public signing key or key reference;
- revocation subject.

A service whose card is unsigned, expired, revoked, or non-conformant may only run in quarantine with no side effects.

---

## 8. ToolCard, DataCard, and AgentCard standards

### 8.1 ToolCard

A ToolCard describes a callable tool. Required fields include tool id, owner, version, description, input schema, output schema, side-effect class, risk tier, required permissions, egress policy, tests, and evidence expectations.

### 8.2 DataCard

A DataCard describes a data source. Required fields include data id, owner, classification, schema, retention, allowed uses, lineage, freshness SLO, query limits, and redaction policy.

### 8.3 AgentCard

An AgentCard describes a delegate agent or model runtime. Required fields include provider, model/runtime, capabilities, context policy, sandbox requirements, risk limits, known failure modes, evaluation references, and maximum autonomy tier.

---

## 9. Work model

### 9.1 Intent

An intent is the user's or system's minimal statement of desired outcome. Intent is not authority. Intent becomes a WorkOrder only after triage.

### 9.2 WorkOrder

A WorkOrder is a governed unit of value. It contains objective, owner, status, task list, success criteria, negative constraints, budgets, attention strategy, risk tier, autonomy tier, and evidence requirements.

### 9.3 Task

Tasks are typed nodes in a DAG. Task kinds include research, planning, coding, code review, test generation, CI triage, bug fix, refactor, security review, dependency update, release, deployment, incident response, observability analysis, data query, knowledge compression, memory maintenance, tool inventory, tool build, self-improvement, policy update, documentation, user communication, voice command, external agent delegation, governance, cost optimization, architecture design, experiment, benchmark, conformance test, backup/restore, migration, and legal/compliance.

### 9.4 Canonical task states

`received -> triaged -> planned -> scoped -> queued -> leased -> running -> awaiting_evidence -> validating -> ready_for_promotion -> promoted -> archived`

Additional states: `awaiting_context`, `blocked`, `quarantined`, `escalating`, `waiting_user`, `paused`, `canceling`, `canceled`, `failed`, `retrying`, `recovered`, `rejected`, `rolled_back`.

Illegal state transitions must be rejected by the Workflow Kernel and recorded as faults.

### 9.5 Scheduling objective

The scheduler optimizes expected user value subject to constraints:

- risk ceiling;
- budget ceiling;
- authority availability;
- evidence requirements;
- dependency ordering;
- user attention budget;
- service health;
- cost and latency;
- learning value;
- strategic priority;
- rollback feasibility.

### 9.6 Cancellation and rollback

Every non-read-only task must either have a rollback plan or be marked irreversible. Irreversible tasks require explicit user attention unless a pre-approved emergency policy applies.

---

## 10. Capability leases

A CapabilityLease is the only way to authorize side effects. It must define:

- lease id;
- issued to;
- issued by;
- permissions;
- target scope;
- risk ceiling;
- autonomy ceiling;
- expiration;
- budget limits;
- egress limits;
- data classes;
- secret access policy;
- allowed tools;
- required evidence;
- revocation subject;
- signature.

Leases are checked at:

1. task start;
2. context grant;
3. tool call;
4. data query;
5. file write;
6. git operation;
7. CI operation;
8. network egress;
9. deployment;
10. memory write;
11. policy change;
12. tool build;
13. self-improvement promotion.

---

## 11. Evidence architecture

Evidence is not logging. Evidence is the proof basis for promotion.

Evidence classes:

- observation;
- unit test;
- integration test;
- regression test;
- static analysis;
- dynamic analysis;
- security scan;
- fuzzing;
- human review;
- provenance;
- reproduction;
- benchmark;
- red-team result;
- negative evidence;
- audit artifact.

Evidence sufficiency levels:

- insufficient;
- partial;
- sufficient for review;
- sufficient for promotion;
- conflicting.

Promotion requires evidence appropriate to risk. A low-risk doc edit may need diff and lint evidence. A security-sensitive dependency change may need provenance, SBOM, vulnerability scan, tests, review, and rollback evidence.

---

## 12. User Attention Firewall

### 12.1 Principle

JMCP should share the bare minimum useful information with the user. It should not stream everything. It should not hide material decisions.

### 12.2 Attention classes

- `silent`: no user-visible message.
- `digest`: include in periodic digest.
- `notify`: show non-blocking update.
- `ask`: request user decision.
- `interrupt`: interrupt current user flow.
- `block_until_user`: cannot proceed without user action.
- `emergency`: immediate, high-salience alert.

### 12.3 Mandatory user escalation

JMCP must escalate when:

- risk tier rises above the approved tier;
- cost budget will be exceeded;
- data classification escalates;
- credential access is requested;
- public or external communication is requested;
- production deployment is requested;
- irreversible change is requested;
- evidence is conflicting;
- the system is uncertain about user intent;
- a policy exception is requested;
- a self-improvement would change authority, evidence, memory, or routing behavior;
- a false-quiet check fails.

### 12.4 Decision packet

A decision packet contains headline, one-paragraph summary, recommended action, alternatives, risk, evidence links, default action, and deadline.

---

## 13. Text and voice interfaces

### 13.1 Text

Text is the default high-fidelity user interface. It supports intent intake, decision packets, explanations, drill-down, search, and command history.

### 13.2 Voice

Voice is a first-class interface but a constrained control surface. A voice command must produce:

- voice segment;
- transcript;
- transcript confidence;
- speaker/session identity;
- command candidate;
- anti-replay nonce;
- confirmation status;
- attention packet if risk requires.

Voice may not directly authorize R4+ actions without confirmation. Voice cannot be used as the only evidence for irreversible actions.

---

## 14. Tool and data awareness

JMCP maintains a live capability graph:

- services;
- tools;
- data sources;
- repositories;
- CI jobs;
- models;
- agents;
- schemas;
- policies;
- tests;
- costs;
- SLOs;
- known failures;
- owners;
- credentials;
- safety cases.

### 14.1 Technology scout loop

JMCP continuously asks:

1. What repeated work is expensive or error-prone?
2. What tool would reduce risk or cost?
3. Does such a tool already exist?
4. Should the tool be built locally, added to Jankurai, or bought/adapted?
5. What evidence proves the tool is safe and general?
6. What scopes should the tool be allowed to operate in?
7. What failures would force deprecation?

---

## 15. Automated tool-building workflow

Automated tool building is a governed workflow:

1. Detect repeated need or bottleneck.
2. Create ToolBuildProposal.
3. Produce specification and threat model.
4. Generate implementation in sandbox.
5. Run tests, fuzzing, static analysis, and red-team probes.
6. Run shadow mode against historical tasks.
7. Quarantine initial use.
8. Promote to repo-local or Jankurai only with evidence.
9. Monitor drift and regressions.
10. Deprecate or rollback on failures.

A tool that can affect code, data, secrets, money, external systems, or deployments must never be promoted by the same agent that built it without independent verification.

---

## 16. Self-improvement workflow

Self-improvement applies to prompts, ZYAL workflows, scheduling heuristics, router rules, evidence requirements, memory policies, UI summarization, tool recommendations, and tests.

It does not apply silently to the authority kernel.

Required flow:

1. SelfImprovementProposal.
2. Safety case.
3. Evaluation plan.
4. Shadow deployment.
5. Metrics comparison.
6. Red-team test.
7. User or governance approval for high-risk changes.
8. Promotion decision.
9. Rollback plan.
10. Post-promotion monitoring.

Forbidden unilateral changes:

- reducing evidence requirements;
- increasing autonomy tier;
- widening authority leases;
- weakening sandboxing;
- suppressing user attention thresholds;
- changing policy engine semantics;
- altering audit retention;
- changing identity trust roots.

---

## 17. Integration contracts

### 17.1 Jankurai

JMCP writes to Jankurai only through lesson/tool proposals with evidence. Jankurai returns standards, best practices, reusable tools, and bad-behavior lessons.

### 17.2 Jeryu

JMCP delegates code understanding, CI, git, graph knowledge, and production promotion checks to Jeryu. Jeryu does not receive ambient authority. Each operation requires a lease.

### 17.3 Jekko/ZYAL

JMCP uses Jekko/ZYAL for advanced reasoning workflows, memory workflows, and agent scripts. ZYAL scripts are versioned, sandboxed, tested, and subject to evidence gates.

### 17.4 External agents

External agents are untrusted workers. They receive minimum necessary context and no direct authority. Their outputs are evidence candidates, not truth.

### 17.5 MCP and A2A

MCP and A2A are adapters below JMCP. MCP exposes tools/resources/prompts. A2A exposes agent interoperability and task communication. Neither protocol replaces JMCP authority, evidence, leases, memory governance, or attention firewall.

---

## 18. Security model

### 18.1 Threat assumptions

JMCP assumes:

- agents hallucinate;
- tools lie or drift;
- service cards may be poisoned;
- context can be prompt-injected;
- logs can be incomplete;
- CI can be flaky;
- tests can be gamed;
- memory can be poisoned;
- voice can be spoofed or ambiguous;
- credentials can leak;
- policy can be misconfigured;
- the control plane itself can have bugs;
- cost can be attacked;
- the user can be overloaded or under-informed.

### 18.2 Controls

- Workload identity with short-lived credentials.
- Signed envelopes and service cards.
- Capability leases.
- Policy-as-code with policy epochs.
- Sandbox classes.
- Network egress control.
- Secret zeroization and brokered access.
- Provenance and SBOM requirements.
- Evidence sufficiency gates.
- Immutable audit logs.
- Watchdog veto service.
- Disaster replay.
- Break-glass mode.
- Quarantine mode.
- False-quiet tests.
- Red-team harness.

---

## 19. Communication backbone

The default backbone is NATS JetStream because it supports lightweight subject-oriented messaging, durable streams, replay, and request/reply patterns. Redpanda/Kafka may be used for high-volume event analytics, but the JCP envelope remains the authority layer.

### 19.1 Required backbone features

- Durable streams for state-changing events.
- Per-subject ordering for task state.
- Dead-letter queues.
- Poison-message quarantine.
- Replay into a clean environment.
- Schema registry.
- Signature verification.
- Backpressure and admission control.
- Tenant/cell isolation.
- Audit stream immutability.

### 19.2 Partition behavior

During partition, services degrade to the lowest safe mode. No new high-risk lease may be issued without authority quorum. Read-only cached views may continue with stale-data warnings.

---

## 20. Data architecture

| Store | Purpose |
|---|---|
| PostgreSQL | Authority, tasks, work orders, leases, policies, user settings. |
| Timescale/PostgreSQL partitions | Time-series operational metrics. |
| Object/CAS store | Evidence, artifacts, logs, transcripts, reports, diffs. |
| Graph store | Code, dependency, tool, data, task, and capability relationships. |
| Vector/search index | Retrieval over docs, logs, code, lessons, memories. |
| Event store | JCP envelope streams and replay. |
| Secret broker | Credential access through leases, never raw storage in memory. |

All durable data must have a DataCard or be covered by a system DataCard.

---

## 21. Frontend cockpit

The frontend is a Vite/TypeScript/React application optimized for low-latency drill-down and memory efficiency.

Required views:

1. **Now View:** what matters now, minimal status, active decisions.
2. **Work View:** work orders, task DAGs, priority, blockers.
3. **Evidence View:** claims, evidence bundles, promotion state.
4. **Systems View:** services, health, leases, backbone, faults.
5. **Tools/Data View:** capability graph, tool cards, data cards.
6. **Memory View:** memory proposals, accepted lessons, contradictions.
7. **Voice/Text View:** conversation, transcripts, confirmations.
8. **Replay View:** reconstruct incidents and task histories.

Performance rules:

- Virtualize large lists.
- Stream updates incrementally.
- Keep event payloads out of React state when large.
- Use Web Workers for graph layout and log filtering.
- Use server-side aggregation for massive traces.
- Every user-visible summary must link to source evidence.

---

## 22. File tree

```text
jmcp/
  Cargo.toml
  crates/
    jcp-schema/                  # generated Rust/TS bindings for JCP/1.0.0
    jcp-validate/                # envelope/schema/signature validators
    jmcp-authority/              # leases, policy, risk, approval
    jmcp-workflow/               # intents, work orders, task DAGs
    jmcp-evidence/               # evidence bundles, sufficiency, provenance
    jmcp-memory/                 # memory proposals, contradiction, decay
    jmcp-attention/              # user attention firewall
    jmcp-tool-registry/          # service/tool/data/agent cards
    jmcp-tool-builder/           # automated tool build workflow
    jmcp-self-improvement/       # governed improvement pipeline
    jmcp-backbone/               # NATS/Redpanda/gRPC transport adapters
    jmcp-observability/          # OTel, SLO, fault detection
    jmcp-security/               # identity, secrets, sandbox, signatures
    adapters/
      jankurai/
      jeryu/
      jekko_zyal/
      mcp/
      a2a/
      github/
      sql/
      nosql/
      graph/
      vector/
      voice/
  apps/
    cockpit/                     # Vite/TS/React frontend
    cli/                         # admin and conformance CLI
  schemas/
    jcp/1.0.0/JCP_1_0_0_Protocol.schema.json
  policies/
    rego/
  proto/
  migrations/
  tests/
    conformance/
    adversarial/
    replay/
    fixtures/
  docs/
    architecture/
    operations/
    security/
```

---

## 23. Build phases

### Phase 0 - Protocol freeze

- Freeze JCP/1.0.0 schema.
- Generate Rust/TypeScript types.
- Build envelope validator.
- Build conformance CLI.
- Create ServiceCard signing flow.

### Phase 1 - Read-only control room

- Ingest repo, CI, service, and data metadata.
- Build cockpit Now/Work/System views.
- No write authority except audit streams.

### Phase 2 - Leased execution

- Implement capability leases.
- Add Jeryu/Jankurai/Jekko adapters.
- Support low-risk reversible tasks.
- Evidence bundles required for promotion.

### Phase 3 - Attention and voice

- Add attention firewall.
- Add voice segmentation, transcript, confirmation.
- Add false-quiet tests.

### Phase 4 - Multi-agent supervision

- Add MCP/A2A adapters.
- Add external-agent sandboxing.
- Add delegation and evaluation workflows.

### Phase 5 - Learning refinery

- Add memory proposals.
- Add Jankurai lesson promotion.
- Add tool/data awareness scoring.

### Phase 6 - Automated tool building

- Add ToolBuildProposal workflow.
- Add sandboxed implementation, quarantine, benchmark, and promotion.
- Add Jankurai global-tool publishing.

### Phase 7 - Governed self-improvement

- Add SelfImprovementProposal workflow.
- Add shadow mode, comparative evaluation, red-team harness, rollback.

### Phase 8 - Autonomous engineering fab

- Add portfolio optimization, cross-repo process yield, digital twin, and federated control-plane trust exchange.

---

## 24. Acceptance criteria

V5 is accepted only if:

1. All services speak JCP/1.0.0 or are isolated behind conforming adapters.
2. The schema validates all core payload types.
3. No side-effecting command can execute without a lease.
4. Task transitions reject illegal moves.
5. Evidence bundles are required for promotion.
6. The user can drill from summary to evidence.
7. Voice commands produce replayable transcripts and confirmation artifacts.
8. Self-improvement cannot mutate authority without governance.
9. Tool-building outputs remain quarantined until independent evidence passes.
10. A full task can be replayed from the event store into a clean environment.
11. Conformance failures revoke service authority.
12. False-quiet tests detect under-notification.
13. Cost and rate budgets are enforced.
14. Memory proposals can be contradicted, decayed, rejected, or scoped.
15. A break-glass incident produces user-visible audit artifacts.

---

## 25. Extreme future vector

JMCP can become far more than an engineering assistant.

### Horizon 1 - Personal engineering control room

JMCP becomes the single interface for repos, CI, issues, code search, logs, tools, lessons, and agents.

### Horizon 2 - Autonomous engineering fab

JMCP manages a continuous portfolio of bug-finding, refactoring, testing, documentation, dependency maintenance, and tool-building tasks with minimal user interruption.

### Horizon 3 - Cross-repo yield optimizer

JMCP learns which patterns waste time, create defects, or reduce velocity across repositories, then pushes standards and tools through Jankurai.

### Horizon 4 - Engineering digital twin

JMCP maintains a live graph model of code, people, tools, services, risk, cost, data, incidents, and delivery flow. It predicts bottlenecks before they surface.

### Horizon 5 - Autonomous R&D foundry

JMCP runs research loops: search, summarize, hypothesize, prototype, test, compress knowledge, build tools, and retire weak ideas.

### Horizon 6 - Proof-carrying agent economy

External agents compete to complete tasks, but JMCP only accepts outputs with leases, evidence, provenance, and evaluation. Trust is earned by proof, not brand.

### Horizon 7 - Federated control-plane mesh

Multiple JMCP instances exchange signed lessons, tools, evidence, and trust attestations without sharing private data.

### Horizon 8 - Cognitive process-control OS

JMCP becomes a general operating system for high-value cognition: not replacing the user, but raising the user's operating altitude by turning raw work into governed, observable, evidence-carrying processes.

---

## 26. Final position

The strongest JMCP is not more autonomous because it is less constrained. It is more autonomous because it is more constrained in the right places: protocol, authority, evidence, memory, attention, and promotion. JCP/1.0.0 is the contract that makes that possible.
