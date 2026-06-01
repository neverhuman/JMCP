# JMCP FINAL Engineering Specification and JPCM-1.0.0 Protocol Standard

**Status:** Final critique-hardened build specification  
**Date:** 2026-06-01  
**System:** JMCP - Joint Master Control Plane  
**Protocol:** JPCM-1.0.0 - Joint Process Control Messaging  
**Required protocol artifact:** `JMCP_FINAL_jpcm_1_0_0.schema.json`  
**Primary stack:** Rust, Tokio, Axum, tonic/gRPC, NATS JetStream, optional Redpanda/Kafka profile, PostgreSQL/Timescale, object/CAS store, graph store, vector/search projections, OpenTelemetry, OPA/Cedar-style policy, Vite, TypeScript, React, Web Workers, WebSocket/SSE, WebRTC voice/text channels.  
**Core integrations:** Jankurai, Jeryu, Jekko/ZYAL, Jnoccio/router, external coding agents, MCP servers, A2A agents, CI runners, GitHub/GitLab-compatible APIs, SQL/noSQL datastores, local/remote tool adapters, text and voice surfaces.

### Final decision

JMCP is the sovereign supervisory control plane for autonomous engineering. JPCM is the mandatory communication protocol and event grammar every service must speak to interact with JMCP. If a fact, action, claim, tool call, approval, task state, voice command, memory write, data query, self-improvement change, or tool-building action is not represented as a valid JPCM envelope, JMCP treats it as unauthenticated noise.

The earlier drafts correctly identified the vision: a fab-level process-control system for software and knowledge work. The final specification hardens that vision by making the communication protocol, task model, authority model, evidence model, user-attention model, and self-improvement model explicit enough to implement, test, certify, and evolve.

## 1. What JMCP is

JMCP is an assured supervisory control plane. It receives minimal human intent; decomposes that intent into governed work; routes work to tools, agents, repositories, datastores, and external systems; enforces leases; collects proof; detects faults; improves standards; and interrupts the user only when human judgment is actually needed.

JMCP owns six things that ordinary agent frameworks do not own: authority, work definition, evidence promotion, cross-system observability, memory governance, and user attention. It is the primary interface for text and voice. The user should not need to monitor logs, CI consoles, agent transcripts, package-manager output, dashboard noise, or repo-specific issue storms. JMCP turns that raw activity into a ranked, evidence-backed stream of decisions, summaries, and interventions.

JMCP should feel like an engineering fab control room. It knows which work cells are active, which processes are yielding value, which steps are bottlenecked, which tools are untrusted, which repos are accumulating jank, which lessons have been learned, which actions are safe to perform autonomously, and which decisions need the user.


## 2. What JMCP is not

JMCP is not a chatbot, not a dashboard, not an MCP host, not an A2A router, not a CI replacement, not a log aggregator, not a model router, not a generic workflow engine, not a thin shell around coding agents, and not a permission prompt machine. It may contain adapters for those systems, but the adapters do not define the control plane.

MCP is an interoperability protocol for exposing tools, resources, prompts, and client features to LLM applications. A2A is an interoperability protocol for agent discovery, messaging, tasks, and artifacts. They are useful subordinate protocols. They are not the authority plane, evidence plane, attention governor, or task-law kernel. JMCP must be able to call MCP and A2A systems, but no MCP server or A2A agent can grant itself authority inside JMCP.

JMCP is also not an unbounded self-modifying agent. It may improve itself, generate tools, and update policies, but those changes must go through the same evidence, lease, experiment, rollback, and promotion machinery as any other high-risk change.


## 3. Non-negotiable laws

1. **No JPCM, no trust.** Side effects, claims, tool calls, task updates, evidence, memory writes, and user decisions outside JPCM are not authoritative.
2. **No lease, no side effect.** Every side-effecting adapter must verify a scoped, revocable capability lease at the moment of action.
3. **No proof, no promotion.** Work cannot be promoted, merged, globally learned, or used as a standard without evidence bundles and policy receipts.
4. **No silent high-risk action.** Irreversible, legal, credential, production, financial, customer, or control-plane changes require explicit policy allowance and often human decision.
5. **No ambient trust in agents.** Agents are untrusted workers. Their outputs are claims, not facts.
6. **No memory without provenance.** Memories are proposals backed by evidence; they can be rejected, tombstoned, quarantined, and expired.
7. **No self-improvement without experiment.** JMCP cannot promote changes to its own scheduler, policies, prompts, workflows, tools, schema, or memory rules without evaluation and rollback.
8. **No attention spam.** User attention is a scarce safety resource. The system must minimize disclosure while avoiding false quiet.
9. **No private side channels.** Direct queues, raw webhooks, untracked subprocess calls, and unregistered adapters are non-conformant.
10. **No exactly-once fantasy.** JPCM assumes at-least-once delivery; idempotency and dedupe are protocol requirements.


## 4. Hostile failure catalogue

- **Authority drift:** a service gains practical ability to act outside its lease because an adapter bypasses JPCM.
- **Protocol drift:** teams add raw webhooks, local queues, or direct MCP calls that never become auditable JPCM events.
- **Exactly-once illusion:** duplicate events or tool calls cause repeated side effects when idempotency is missing.
- **Evidence theater:** CI badges and screenshots are accepted as proof despite being stale, forged, or irrelevant.
- **Memory poisoning:** a compromised repo, document, or agent plants false lessons that become global policy.
- **User-attention failure:** the system suppresses a rare but critical decision because it optimizes for quietness.
- **Voice command ambiguity:** speech recognition turns a low-risk instruction into a high-risk mutation.
- **Confused deputy:** a low-trust tool routes action through a high-trust service without preserving original authority.
- **Sandbox escape:** an agent exploits language runtime, package manager, browser, or kernel behavior.
- **Supply-chain compromise:** a tool, dependency, model package, or MCP server changes behavior after approval.
- **Silent cost explosion:** background agents recursively spawn work or run high-cost models without budget enforcement.
- **Rollback fantasy:** a change is treated as reversible even though external systems, users, or data migrations make it hard to unwind.
- **Observability overload:** telemetry exists but is sampled, uncorrelated, or too expensive for agents to query continuously.
- **Privacy leakage:** context packets include more data than needed or are retained longer than permitted.
- **Self-improvement corruption:** JMCP changes its own policies, prompts, scheduler, or tools without independent evaluation.
- **Tool-building risk:** generated tools are useful but under-tested, over-permissioned, or globally registered too quickly.
- **Human trust erosion:** too many alerts teach the user to rubber-stamp; too few alerts hide unacceptable autonomy.
- **Protocol version lock-in:** v1 becomes impossible to evolve because compatibility and extension rules were not defined.
- **Disaster recovery gap:** event streams restore tasks but not leases, object evidence, graph state, or UI decisions coherently.
- **Adversarial incentives:** agents optimize visible metrics while degrading maintainability, safety, or long-term yield.

The final design does not claim these failures disappear. It makes them visible, bounded, testable, and recoverable. The control plane is only as good as the weakest adapter boundary, so conformance testing is mandatory before a service may receive non-observe leases.


## 5. Canonical planes

JMCP is divided into planes with hard boundaries.

| Plane | Responsibility | Canonical services |
|---|---|---|
| Authority Plane | Identity, leases, policy, veto, approvals, risk, budgets | `jmcp-authority`, `jmcp-policy`, `jmcp-lease` |
| Communication Plane | JPCM envelopes, durable streams, request/reply, subscriptions, replay, schema registry | `jmcp-bus`, `jmcp-router`, `jmcp-schema` |
| Workflow Plane | Intent intake, work orders, task DAGs, scheduling, cancellation, rollback | `jmcp-workflow`, `jmcp-scheduler` |
| Evidence Plane | Evidence bundles, verifiers, provenance, promotion gates, proof search | `jmcp-evidence`, `jmcp-promotion` |
| Observability Plane | Traces, metrics, logs, incidents, fault detection, bottleneck analysis | `jmcp-observe`, `jmcp-fault` |
| Tool/Data Awareness Plane | Service cards, tool cards, data cards, capability graph, technology scout | `jmcp-inventory`, `jmcp-tech-scout` |
| Memory/Experience Plane | Memory proposals, compaction, tombstones, lessons learned, internal experience stream | `jmcp-memory`, `jmcp-experience` |
| User Plane | Text chat, voice chat, cockpit, decision cards, drill-down, summaries | `jmcp-ui`, `jmcp-voice`, `jmcp-attention` |
| Integration Plane | Jankurai, Jeryu, Jekko/ZYAL, MCP, A2A, CI, datastores, external agents | `jmcp-adapter-*` |


## 6. JPCM-1.0.0 protocol overview

JPCM-1.0.0 is a signed envelope protocol and task-control grammar. It is not tied to a single wire transport. The canonical schema is JSON Schema Draft 2020-12 in `JMCP_FINAL_jpcm_1_0_0.schema.json`.

JPCM has nine normative layers:

1. **Envelope layer:** identity, type, subject, trace, authority, risk, privacy, delivery, hash, signature, payload.
2. **Subject layer:** hierarchical routing subjects beginning with `jpcm.*`.
3. **Delivery layer:** durable/event, command, audit, evidence, attention, and voice realtime classes.
4. **Authority layer:** capability leases, policy decisions, vetoes, waivers, budgets, and risk tiers.
5. **Task layer:** user intents, work orders, tasks, DAGs, scheduling, cancellation, rollback, and quarantine.
6. **Evidence layer:** claims, artifacts, verifiers, attestations, provenance, reproducibility, and promotion.
7. **Attention layer:** minimum useful summaries, decision packets, escalation, suppression, and drill-down.
8. **Tool/data awareness layer:** service cards, tool cards, data cards, technology findings, tool-build proposals.
9. **Self-improvement layer:** proposals, experiments, evaluation results, promotion, rejection, and rollback.

Every JPCM message is immutable. Corrections are new messages that causally reference older messages. Consumers must tolerate duplicate messages and out-of-order delivery across ordering keys.


## 7. Required envelope

Every message must contain the following required fields:

```json
{
  "jpcm_version": "1.0.0",
  "id": "018ff0b5-...",
  "type": "task.completed",
  "subject": "jpcm.task.event.repo.jeryu.wo_123.task_456",
  "time": "2026-06-01T12:00:00Z",
  "source": "service:jeryu.git",
  "producer": { "id": "jeryu-adapter", "kind": "adapter", "trust_domain": "jmcp.local" },
  "tenant": "neverhuman",
  "cell": "prod-us-west-1",
  "trace": { "trace_id": "...", "span_id": "..." },
  "correlation_id": "wo_123",
  "actor": { "actor_id": "agent.codex.17", "actor_kind": "agent", "display_name": "Codex Worker 17" },
  "authority": { "decision_id": "pd_9", "mode": "pr_only", "lease_ids": ["lease_7"], "policy_version": "2026.06.01", "allowed": true },
  "risk": { "tier": "r2_repo_change", "score": 0.37, "blast_radius": "repo", "reversibility": "rollback_available" },
  "privacy": { "data_classes": ["internal"], "retention": "audit", "purpose": "task evidence", "egress_allowed": false },
  "delivery": { "class": "durable", "ack_required": true, "retry_policy": { "max_attempts": 8 } },
  "schema_ref": "https://schemas.jmcp.dev/jpcm/1.0.0/envelope.schema.json#/$defs/Task",
  "payload_hash": { "alg": "sha256", "value": "..." },
  "signature": { "alg": "Ed25519", "kid": "jmcp-prod-2026-06", "value": "...", "covered_fields": ["..."] },
  "payload": {}
}
```

Signatures cover the canonicalized envelope excluding transport headers. `payload_hash` is computed over canonical JSON for the payload. Implementations should use Ed25519 or DSSE/JWS profiles and should support key rotation, certificate chains, and transparency-log references.


## 8. Message families

The schema defines message families for service health, user intent, text, voice, work orders, tasks, leases, context, tools, data, evidence, promotion, rollback, policy, memory, technology scouting, tool building, self-improvement, incidents, and audit.

The most important rule is that **all work is a task or a task-producing intent**. Even self-improvement, automated tool building, repo creation, research, code review, data migration, and incident response are work orders with task DAGs, leases, budgets, evidence, and rollback plans.

JPCM message types are versioned by schema, not by ad hoc strings. Additive payload fields are allowed within a minor-compatible schema. Removing fields, changing semantics, or broadening authority requires a major version and migration bridge.


## 9. Communication backbone

The default backbone is NATS JetStream because JMCP needs durable streams, replay, work queues, subject routing, pull consumers, backpressure, and edge-to-core deployment. Redpanda/Kafka is an acceptable high-throughput profile when topic partitioning, retention, and replay semantics are mapped to JPCM conformance tests. gRPC is used for low-latency internal request/reply. WebSocket/SSE expose UI streams. WebRTC supports voice and realtime UI telemetry. MCP, A2A, CLI, and database protocols are adapter inputs that must normalize into JPCM.

Backbone requirements:

- At-least-once delivery is assumed. Every side-effect command needs an idempotency key.
- Durable command streams use explicit acknowledgments and dead-letter subjects.
- Audit and evidence streams use retention policies appropriate to legal/audit needs.
- Voice realtime frames may be ephemeral, but normalized command candidates must become durable JPCM messages.
- Ordering is guaranteed only within an ordering key. Work orders, tasks, leases, and evidence bundles must declare ordering keys.
- Consumers must support replay from a stream sequence, timestamp, or audit checkpoint.
- Backpressure is a control signal, not a transport accident. Saturated services publish `service.degraded` and receive fewer leases.
- Poison messages are quarantined with full parse/validation errors and cannot be silently dropped.
- Disaster recovery restores stream state, schema registry, object evidence, policy versions, leases, task state, projections, and UI decisions as one consistency set.


## 10. Service registration and conformance

Every service publishes a `ServiceCard`. Every tool publishes a `ToolCard`. Every datastore or important dataset publishes a `DataCard`. These cards are signed, versioned, and subject to conformance testing.

A service begins as `untrusted`, can enter `probation`, and may become `trusted_for_scope`. Trust is not global. A service can be trusted to read a repository but not write it; trusted to run static analysis but not access credentials; trusted to propose a memory but not promote it.

Conformance gates:

1. Schema validation for every emitted message.
2. Signature verification and key rotation test.
3. Lease refusal test: service must reject side effects without valid lease.
4. Idempotency test: duplicate commands must not duplicate side effects.
5. Replay test: service projections must rebuild from event streams.
6. Backpressure test: service must shed or queue load safely.
7. Fault injection test: lost acks, duplicate events, reordered updates, and poison payloads.
8. Security test: prompt injection, tool metadata poisoning, malicious artifacts, credential egress.
9. Attention test: high-risk action must produce decision packet.
10. Rollback test: service must prove rollback or label action irreversible.


## 11. Work model

The work hierarchy is: **Intent -> WorkOrder -> Task DAG -> Tool Calls / Agent Runs -> Evidence -> Promotion / Rollback / Memory**.

A user may provide a tiny instruction such as "clean this repo up" or "find the bottleneck." JMCP expands it into a work order only after defining non-goals, risk tier, acceptance criteria, required evidence, budget, and autonomy tier. The user should not need to see the full task graph unless they ask or a decision is required.

Canonical task states:

`created -> scheduled -> dispatched -> claimed -> running -> awaiting_evidence -> awaiting_approval -> succeeded -> promoted`

Failure states:

`blocked`, `failed`, `cancelled`, `quarantined`, `rollback_pending`, `rolled_back`.

A task cannot jump from `running` to `promoted`. It must pass through evidence and policy. A task that touches production, credentials, legal/compliance, external money, customer data, or the control plane must have an explicit risk tier and decision path.


## 12. Task taxonomy including self-improvement and tool building

JPCM-1.0.0 defines task types for research, concept search, knowledge compression, architecture design, repo creation, feature work, bug hunting, bug fixing, refactoring, deduplication, performance work, security review, vulnerability fixes, dependency updates, CI fixes, tests, documentation, release, deployment, data migration, incident response, root cause analysis, tool building, tool qualification, model evaluation, policy update, protocol update, self-improvement, memory maintenance, UX review, voice flow, cost optimization, compliance review, and code-graph analysis.

Automated tool building is a first-class work order. JMCP may decide a missing tool would make all repos safer or more efficient. It must create a `technology.build.proposal`, build the tool in a sandbox, generate tests and documentation, run security and conformance checks, place the tool in probation, measure value, and only then promote the `ToolCard` to approved. Global tools should generally be contributed to Jankurai or another standards/tooling repo after proof.

Self-improvement is also a first-class work order. JMCP may propose changes to policies, prompts, workflow templates, model routing, memory compaction, evidence verifiers, scheduler weights, attention filters, or protocol schemas. It cannot silently promote such changes. The self-improvement proposal must declare hypothesis, guardrails, evaluation plan, rollback plan, scope, risk, and whether human approval is required. `can_self_promote` is false by schema.


## 13. Authority and capability leases

A lease is a non-transferable, signed, scoped permission to perform actions for a limited time, cost, resource set, and risk tier. Leases are checked at the adapter boundary, not just by the planner. A compromised or hallucinating agent cannot rely on prompt text as authority.

A lease includes issuer, recipient, scopes, valid time window, risk tier, budgets, constraints, revocation subject, and renewal policy. Lease scopes are resource-specific: repo, branch, file, directory, database, table, bucket, API, tool, model, secret, CI, cloud account, UI, voice session, memory store, knowledge graph, network, or control plane.

Lease rules:

- Leases are non-transferable.
- Leases expire quickly by default.
- Leases can be revoked asynchronously.
- A lease for read does not imply write.
- A lease for branch write does not imply merge.
- A lease for tool execution does not imply network access.
- A lease for model use does not imply data egress.
- A lease for self-improvement proposal does not imply self-promotion.
- Adapter enforcement failures are severity incidents.


## 14. Evidence and promotion

Evidence bundles are structured proof objects, not informal logs. A bundle contains claims, artifacts, digests, verifiers, quality dimensions, traces, provenance, and policy receipts. Evidence quality is scored on independence, freshness, reproducibility, completeness, and relevance.

Promotion gates:

1. Work-order acceptance criteria satisfied.
2. Required evidence classes present.
3. Evidence verifier is independent of worker where risk requires it.
4. Artifacts have stable references and digests.
5. Policy decision allows promotion.
6. Jankurai standards/veto lane passes when code quality or global lessons are involved.
7. Jeryu CI/Git/graph reality matches the claimed code state.
8. Rollback plan exists or irreversibility is explicitly accepted.
9. User decision is present when risk tier requires it.

Conflicting evidence does not average out. It opens an evidence conflict, blocks promotion, and asks JMCP to drill down before bothering the user unless the conflict itself is high-risk.


## 15. User attention firewall

JMCP must protect the user from excess information while preventing false quiet. It should show the bare minimum needed for the user to exercise judgment.

Attention classes:

- `silent`: no user-facing event; available in drill-down.
- `digest`: batched summary.
- `notify`: worth knowing, no decision needed.
- `decision`: user choice required.
- `break_glass`: urgent interruption.

A decision packet contains summary, why now, recommended action, options, risk delta, evidence references, deadline, and drill-down links. It does not dump agent transcripts unless requested. Drill-down is progressive: summary -> evidence -> task DAG -> tool calls -> logs/traces -> raw artifacts.

False-quiet tests are mandatory. JMCP should periodically evaluate whether the attention governor would suppress scenarios that users later consider important. Optimizing for quietness alone is unsafe.


## 16. Text and voice interaction

Text chat is the default canonical user channel. Voice is a convenience and realtime control surface, not a privileged authority source. Voice input must pass through session boundary detection, transcription, normalization, risk scoring, and confirmation rules.

Voice rules:

- A voice segment is not a command until it becomes `voice.command.candidate`.
- High-risk voice commands require confirmation, preferably with a text-visible decision packet.
- Voice biometrics may increase confidence but cannot bypass leases or policy.
- Ambiguous pronouns, unclear scope, or destructive verbs increase risk and require clarification.
- Barge-in stop must immediately publish cancellation intent for pending low-risk tasks and pause high-risk tasks for policy review.
- Audio is retained only according to privacy policy; transcripts and normalized intents become JPCM records.


## 17. Tool and data awareness

JMCP maintains a live capability graph of services, tools, data stores, models, repositories, CI lanes, policies, evidence verifiers, and user surfaces. It knows what it has, what it lacks, what is stale, what is risky, what is expensive, and what should be built.

The inventory is not a static catalog. It is a process-control instrument. JMCP uses it to route work, detect bottlenecks, discover redundant tools, retire unsafe adapters, propose new global tools, choose model routes, understand data freshness, and generate technology radar reports.

Every tool has a ToolCard with input schema, output schema, side effects, risk tier, leases required, dry-run support, rollback support, conformance suite, and status. Every dataset has a DataCard with sensitivity, access patterns, retention, lineage, freshness SLO, quality score, and purpose constraints.


## 18. Memory and internal experience

JMCP should have an internal stream of experience, but that stream is not hidden authority. It is an event-sourced record of observations, decisions, failures, lessons, and hypotheses. It can be queried, compressed, evaluated, and improved.

The memory system has four layers:

1. **Raw experience stream:** append-only JPCM events, traces, incidents, and task outcomes.
2. **Working memory:** active work-order context and recent state.
3. **Project memory:** repo-specific lessons, architecture facts, commands, risks, and standards.
4. **Global memory:** cross-repo lessons that survived evidence and Jankurai review.

Memories enter through proposals. They may be accepted, rejected, compacted, tombstoned, quarantined, or expired. Global lessons require stronger evidence than local session notes. Negative examples are valuable but dangerous; they must include scope, source, and anti-overgeneralization warnings.


## 19. Integration contracts

### Jankurai
Jankurai is the standards, lessons, jank-reduction, proof-lane, and veto layer. JMCP pushes durable lessons and reusable tools to Jankurai only after evidence. Jankurai returns standards, negative examples, conformance tests, and vetoes. It is the global quality memory for code practices.

### Jeryu
Jeryu is the code, Git, CI, and graph reality layer. JMCP asks Jeryu to inspect code, map dependencies, run CI, stage changes, create branches, open PRs, analyze relationships, and provide evidence. Jeryu must never accept direct agent writes that bypass JPCM leases.

### Jekko/ZYAL
Jekko is the native reasoning/workflow engine. ZYAL scripts can implement repeatable agent workflows, smarter memory operations, scoped reasoning, and multi-agent execution patterns. Jekko receives work only through JPCM work orders and leases.

### External agents
Codex, Claude, OpenHands, SWE-agent, and other workers are untrusted executors. They may produce patches, explanations, tests, and research. JMCP supervises them through sandboxes, leases, evidence requirements, and independent verification.

### MCP and A2A
MCP servers and A2A agents are subordinate integrations. Adapters translate their operations into JPCM envelopes. MCP/A2A metadata is untrusted until validated. Tool descriptions are never authority.


## 20. Frontend cockpit

The React/Vite frontend is not a dashboard of raw noise. It is a cockpit for supervising a process-control system.

Required views:

- **Now:** what JMCP thinks matters now; active decisions; risk heat.
- **Work:** work orders, task DAGs, schedules, blockers, budgets, rollback state.
- **Proof:** evidence bundles, policy receipts, CI, provenance, verifier status.
- **Systems:** service health, leases, adapters, backpressure, incidents, conformance.
- **Tools/Data:** tool cards, data cards, capability graph, technology gaps.
- **Memory:** accepted memories, proposals, tombstones, poisoning alerts.
- **Voice/Text:** active sessions, decision cards, transcript-derived command candidates.
- **Drill-down:** progressive disclosure from answer to raw trace.

Performance requirements: virtualized lists, Web Workers for local projections, WASM validators for schema checks where helpful, bounded memory caches, streaming updates, local-first UI state, and strict separation between UI rendering and authority decisions.


## 21. Security architecture

JMCP assumes hostile context, compromised tools, confused agents, malicious documents, poisoned dependencies, and user mistakes. Security is distributed across identity, leases, sandboxes, policy, evidence, observability, and attention.

Required controls:

- Workload identity with mTLS/SPIFFE-style identities.
- Signed JPCM envelopes and key rotation.
- OPA/Cedar-style policy decisions with receipts.
- Sandboxed workers using containers, gVisor/Firecracker profiles, or equivalent isolation.
- Network egress allowlists per lease.
- Secret brokering with zero secret exposure to models when possible.
- Dependency provenance, SLSA-style build controls, Sigstore/in-toto-style attestations.
- Runtime detection for sandbox escape, credential access, network anomaly, and filesystem abuse.
- Prompt/tool/memory poisoning tests in conformance suites.
- Incident response playbooks and break-glass revocation.


## 22. Self-improvement governor

JMCP should always improve, but never by silently rewriting its own values or control boundaries. Self-improvement is managed as a portfolio of experiments.

A self-improvement work order must specify:

- Target subsystem.
- Hypothesis.
- Expected value.
- Risk tier.
- Evaluation dataset or simulation.
- Guardrails.
- Rollback plan.
- Evidence required.
- Promotion scope.
- Human approval requirement.

Examples: improve scheduler priority weights, add a new evidence verifier, compress noisy memory, build a new cross-repo duplicate detector, update a Jekko/ZYAL workflow, add an adapter for a new CI system, harden the voice confirmation flow, or revise the JPCM schema. Control-plane self-modification is risk tier `r6_self_modifying_control_plane` and cannot self-promote.


## 23. Implementation file tree

```text
jmcp/
  Cargo.toml
  crates/
    jpcm-core/              # envelope, schema, canonicalization, signing, ids
    jpcm-bus/               # NATS/Redpanda/gRPC transport bindings
    jmcp-authority/         # identities, leases, risk, budgets
    jmcp-policy/            # OPA/Cedar adapters, policy receipts, vetoes
    jmcp-workflow/          # work orders, task DAGs, scheduler, rollback
    jmcp-evidence/          # evidence bundles, verifiers, provenance
    jmcp-memory/            # memory proposals, compaction, tombstones
    jmcp-inventory/         # service/tool/data cards, capability graph
    jmcp-attention/         # minimal disclosure, decision packets, digests
    jmcp-voice/             # WebRTC sessions, transcription, command candidates
    jmcp-observe/           # traces, metrics, incidents, fault detection
    jmcp-adapter-jankurai/
    jmcp-adapter-jeryu/
    jmcp-adapter-jekko/
    jmcp-adapter-mcp/
    jmcp-adapter-a2a/
    jmcp-adapter-github/
    jmcp-adapter-ci/
  ui/
    app/                    # Vite/React cockpit
    packages/jpcm-client/    # generated TS client and validators
    packages/graph-view/
    packages/voice-console/
  schemas/
    jpcm/1.0.0/envelope.schema.json
  policies/
    authority.rego
    promotion.rego
    attention.rego
    self_improvement.rego
  zyal/
    workflows/
    verifiers/
    scouts/
  conformance/
    fixtures/
    replay/
    adversarial/
    property_tests/
  docs/
    protocol/
    architecture/
    operations/
    safety-cases/
  ops/
    docker-compose.yaml
    k8s/
    systemd/
    dashboards/
```


## 24. Build phases

1. **Protocol kernel:** implement schema validation, canonicalization, signatures, subjects, message registry, generated Rust/TypeScript types, conformance fixtures.
2. **Backbone MVP:** NATS JetStream subjects, durable streams, command request/reply, replay, poison queue, audit stream.
3. **Authority MVP:** identities, leases, policy decisions, adapter-side lease checks.
4. **Workflow MVP:** user intent, work order, task DAG, scheduler, cancellation, rollback labels.
5. **Evidence MVP:** evidence bundles, CI/test/static-analysis verifiers, promotion gate.
6. **Jeryu/Jankurai/Jekko adapters:** code reality, standards/veto, and ZYAL workflow execution.
7. **Cockpit MVP:** Now, Work, Proof, Systems, Tools/Data, Memory, Voice/Text views.
8. **Voice MVP:** WebRTC session, transcript, command candidate, confirmation, stop/pause.
9. **Tool/data awareness:** ServiceCard/ToolCard/DataCard inventory and capability graph.
10. **Self-improvement/tool-building:** proposal, sandbox build, eval, promotion, rollback.
11. **Hardening:** adversarial tests, chaos, disaster recovery, red-team prompts, supply-chain gates.
12. **Scale:** multi-cell deployment, event archival, distributed projections, privacy partitions.


## 25. Evaluation and acceptance criteria

A build is not acceptable until it passes the following:

- 100% schema validation on generated fixtures.
- 100% side-effect adapters reject missing/expired/wrong-scope leases.
- Duplicate command replay produces no duplicate external side effects.
- Event replay reconstructs work-order/task state and evidence projections.
- Prompt injection tests cannot cause unauthorized tool use.
- Tool metadata poisoning tests cannot escalate authority.
- Voice ambiguity tests trigger confirmation for high-risk commands.
- Memory poisoning tests quarantine unsupported global lessons.
- Evidence forgery tests block promotion.
- Disaster recovery restores audit/evidence/task/lease state within defined RTO/RPO.
- User attention tests measure false-positive alerting and false-quiet misses.
- Self-improvement tests prove rollback and require human approval for `r6` changes.


## 26. Extreme future vector

The final ambition is larger than an agent dashboard. JMCP can become an autonomous engineering operating system.

**Horizon 1 - Personal control room:** one user supervises all repos, tools, agents, and knowledge work through text/voice and cockpit views.

**Horizon 2 - Autonomous software fab:** JMCP operates work cells that continuously reduce jank, find bugs, improve tests, remove duplication, and raise engineering yield.

**Horizon 3 - Cross-repo learning engine:** lessons learned in one repo become evidence-backed standards and tools for all repos through Jankurai.

**Horizon 4 - Engineering digital twin:** JMCP maintains a live causal model of code, teams, tools, bottlenecks, defects, incidents, costs, and value flows.

**Horizon 5 - Autonomous R&D foundry:** JMCP can explore new concepts, compare technologies, build prototypes, create repos, write tools, run experiments, and compress knowledge into reusable capabilities.

**Horizon 6 - Proof-carrying agent marketplace:** agents and tools compete for work by presenting verifiable service cards, capability claims, cost/yield histories, and conformance receipts.

**Horizon 7 - Self-hardening organization OS:** JMCP becomes the control system that continuously improves engineering process, security posture, data quality, toolchains, memory, and user leverage while preserving human sovereignty.

**Horizon 8 - Civilization-scale trust fabric:** the same protocol ideas could govern autonomous work across organizations, where proof, authority, provenance, and minimal human attention matter more than raw agent capability.


## 27. Final acceptance definition

JMCP is ready when the user can say a minimal instruction by text or voice, the system can plan and execute bounded work through untrusted agents and tools, every side effect is lease-checked, every claim is evidence-backed, every important issue is escalated with minimum useful context, every routine detail is suppressed but drillable, every lesson is governed, every tool is known, every missing tool can be proposed and built safely, and every control-plane improvement is itself controlled.
