# Joint Master Control Plane (JMCP) and Joint Process/Control Messaging (JPCM) Final Engineering Specification

**Version:** JMCP-FINAL-1.0 / JPCM-1.0.0  
**Status:** Build-ready control-plane standard  
**Date:** 2026-06-01  
**Primary implementation stack:** Rust backend, TypeScript/Vite/React control room, signed JSON event protocol, SQL/NoSQL/graph/vector stores, Jankurai, Jeryu, Jekko/ZYAL, MCP/A2A adapters, isolated external agent runners.

## 1. Final Definition

JMCP is the user's sovereign control plane for software, agents, tools, data, incidents, learning, and autonomous work. It is not merely an automation dashboard and not merely a chat assistant. It is the authority, observability, evidence, task, and user-attention layer that decides what work may happen, what proof is required, what the user must see, and what can remain below the user's attention threshold.

JPCM is the mandatory protocol used by every JMCP-controlled service. A service is compliant only when it can describe itself with a service card, emit and consume signed JPCM envelopes, honor capability leases, bind work to task state, attach replayable evidence, and obey user-attention policy.

## 2. What This Is

JMCP is:

1. **A control plane:** It coordinates distributed tools, agents, datastores, CI systems, repositories, user interfaces, and voice interfaces.
2. **An evidence plane:** It accepts no high-effect claim without verifiable evidence.
3. **A protocol authority:** It makes JPCM the shared language for tasks, tools, data, evidence, policy, voice, incidents, and self-improvement.
4. **A human-attention reducer:** It shares the minimum information needed for the user to remain safe, informed, and in control.
5. **A learning system:** It compresses lessons from repos, incidents, bugs, and agent mistakes into quarantined knowledge and policy.
6. **A tool-aware and data-aware platform:** It knows what capabilities it has, where data lives, what data is stale, and which tools should be built next.
7. **A fab-style process-control system for knowledge work:** It detects faults, bottlenecks, drift, waste, and unsafe process conditions.

## 3. What This Is Not

JMCP is not:

1. A permissionless agent swarm.
2. A replacement for CI, security scanners, source control, or policy engines.
3. A chat UI that directly lets models mutate production.
4. A log aggregator with a nicer dashboard.
5. A memory store that trusts everything it reads.
6. A universal promise that automation is always safer than human review.
7. A single giant model. Models and agents are replaceable workers under JMCP authority.

## 4. Non-Negotiable Design Laws

- **One protocol:** Every service communicates through JPCM or through a JPCM adapter.
- **Signed envelopes:** No unsigned event may influence authority, memory, task state, evidence, or user attention.
- **Lease before effect:** Any effect beyond observation requires a capability lease.
- **Evidence before trust:** High-effect completion, deployment, policy promotion, and self-improvement require evidence bundles.
- **User attention is scarce:** The user sees decisions, incidents, risk deltas, and summaries, not raw system noise.
- **Agents are untrusted workers:** Jekko, Codex, Claude, OpenClaw-like assistants, MCP servers, and A2A agents are bounded by leases and evidence obligations.
- **Self-improvement is quarantined:** The system may improve itself only through shadow/canary evaluation and rollback.
- **Tool building is supply-chain governed:** Generated tools require reuse search, sandboxed build, SBOM/provenance, evaluation, and staged promotion.

## 5. JPCM Protocol Overview

JPCM-1.0.0 defines a canonical signed event envelope. The envelope is the unit of causality, replay, audit, transport, and policy. All JPCM messages contain: schema version, envelope ID, event type, timestamp, producer, authority context, task reference, routing metadata, payload, evidence, observability, attention, privacy, and integrity.

### 5.1 Event Families

JPCM event families are:

- service card and health events
- capability lease events
- task lifecycle events
- tool invocation events
- data query events
- evidence attestation/challenge events
- user and voice events
- policy lesson events
- self-improvement events
- automated tool-build events
- incident and disaster-mode events

### 5.2 Service Cards

Every service must declare a signed service card before it can participate. The service card contains service identity, protocol versions, capabilities, supported effect classes, data surfaces, health endpoint, schema hash, trust domain, and conformance level.

A service card is not marketing metadata. It is an admission contract. If a service claims it can write repositories, it must also declare the exact effect classes, evidence obligations, lease constraints, sandbox constraints, and failure semantics for that capability.

### 5.3 Capability Leases

Capability leases are scoped, expiring grants. They bind a subject to capabilities, data classes, network permissions, filesystem permissions, cost ceilings, runtime ceilings, revocation topics, and policy references.

No worker may rely on ambient authority. Tokens, environment variables, repository write access, cloud credentials, and external network calls must be mediated by a lease.

### 5.4 Task Lifecycle

A JMCP task has these canonical states:

`proposed -> accepted -> planning -> queued -> running -> blocked/waiting_human/waiting_external -> verifying -> completed/failed/cancelled/superseded/rolled_back`.

All work is a task. This includes user requests, autonomous bug hunts, refactors, incident response, data research, knowledge compression, policy promotion, self-improvement, voice assistance, and tool building.

## 6. Communication Backbone

The backbone has three planes:

1. **Command plane:** strict, lease-checked, effectively-once commands for effects.
2. **Event plane:** append-only signed telemetry and state changes.
3. **Read-model plane:** optimized materialized views for the React control room, voice assistant, and user summaries.

The command plane fails closed. The event plane degrades by priority. The read-model plane may lag but must show freshness and degraded-state markers.

### 6.1 Suggested Backbone Stack

- **NATS JetStream or Redpanda/Kafka** for event streams.
- **PostgreSQL** for task state, leases, and durable read models.
- **Graph database** for code/entity dependency maps and causal relationships.
- **Object store** for evidence artifacts.
- **Vector store** for memory retrieval, always taint-labeled.
- **OpenTelemetry-compatible traces/metrics/logs** converted into JPCM evidence and observability references.

### 6.2 Priority Lanes

- `realtime`: voice, human decisions, safety interrupts.
- `high`: incidents, lease revocations, security findings, production regressions.
- `normal`: ordinary task progress.
- `bulk`: low-priority background analysis, duplicate detection, refactor candidates.
- `archive`: replay and historical compression.

### 6.3 Replay

JPCM must be replayable. Replay reconstructs task state, authority decisions, evidence validity, and attention decisions from the append-only ledger. Replay is mandatory for incident response and conformance.

## 7. User Interaction Model

The user communicates by text, voice, and UI drill-down. JMCP responds with the smallest sufficient signal:

- **No interruption:** background work is safe, low-risk, and progressing.
- **Digest:** useful summary, no decision needed.
- **Decision:** user must choose between options.
- **Urgent:** risk is rising or a deadline exists.
- **Incident:** user must know because blast radius, cost, production, security, or irreversible state is involved.

Voice interactions require session binding, transcript confidence, and explicit readback for high-effect actions. A casual utterance must never become a repository write, deployment, secret access, or policy change without confirmation.

## 8. Tool and Data Awareness

JMCP maintains a live capability map containing:

- service cards and conformance levels
- available tools, effect classes, and costs
- data stores, schemas, freshness, owners, and sensitivity
- known repo graphs and code ownership
- model/agent strengths, weaknesses, and failure histories
- bottlenecks and unmet capability gaps

The map is not static documentation. It is continuously validated by health reports, evidence receipts, Jeryu graph updates, Jankurai audit outcomes, and task performance.

## 9. Automated Tool Building

JMCP may build tools when a capability gap is proven. The tool-building lifecycle is:

1. detect repeated capability gap
2. search existing tools and repos
3. propose a tool-build task
4. acquire sandboxed build lease
5. implement in quarantine
6. generate SBOM and provenance
7. run malicious-input and regression tests
8. evaluate cost, correctness, security, and reuse
9. promote to limited canary
10. promote to standard tool or reject/deprecate

Tool building cannot bypass supply-chain controls. A generated tool is more dangerous than an ordinary script because it may be trusted by future automation.

## 10. Self-Improvement

JMCP self-improvement includes prompts, policies, schemas, routing, memory, UI, evaluations, tools, and code. It follows this promotion ladder:

`proposal -> quarantine -> shadow -> canary -> staging -> production -> periodic revalidation`.

Authority-changing improvements require human approval. Memory promotion requires taint checks. Policy promotion requires counterexamples and expiry. Schema changes require semver, compatibility analysis, and conformance fixture updates.

## 11. Integration Responsibilities

- **Jankurai:** lesson quarantine/promotion, anti-vibe coding checks, proof gates, global best practices, audit receipts.
- **Jeryu:** repo graph, CI control, git/PR/release/runner integration, proof-backed code state, production promotion gates.
- **Jekko/ZYAL:** cognitive workflows, agent scripting, code exploration, multi-step reasoning, local coding execution under leases.
- **MCP adapters:** external tools and context are translated into JPCM events and leases.
- **A2A adapters:** external agent cards, tasks, and artifacts are translated into JPCM service cards, tasks, and evidence bundles.
- **External agents:** Codex, Claude, and others operate only as sandboxed workers with scoped inputs, explicit objectives, and evidence obligations.

## 12. React Control Room

The Vite/React/TypeScript frontend is a control room, not a chat shell. It shows:

- current user-attention queue
- active tasks and state transitions
- evidence graph and replay controls
- service health and conformance
- capability map and tool inventory
- incidents and disaster mode
- voice transcript/confirmation state
- autonomous improvement proposals
- drill-down paths from summary -> task -> evidence -> raw artifact

The UI must remain fast under high telemetry volume by reading materialized views, not raw streams.

## 13. Conformance Levels

- **C0 Adapter:** can wrap a legacy tool and emit minimal JPCM events.
- **C1 Observable:** emits signed health, task, and evidence events.
- **C2 Lease-Aware:** refuses effects without valid capability leases.
- **C3 Evidence-Complete:** emits replayable evidence for all high-effect work.
- **C4 Fault-Managed:** supports revocation, degraded mode, replay, and incident hooks.
- **C5 Autonomous-Ready:** supports self-improvement/tool-building quarantine, canary promotion, and full conformance tests.

## 14. Failure Modes and Required Controls

| Failure | Required control |
|---|---|
| Protocol drift | Schema validation, signed conformance receipts, reject unknown required semantics |
| Authority confusion | Effect classes, explicit approval state, enforcement at runner and bus |
| False evidence | Content-addressed evidence, verifier identity, replayable commands, quorum checks |
| Context poisoning | Source taint labels, role separation, context firewall, quoted untrusted material |
| Lease overbreadth | Least-privilege lease compiler and expiring scoped tokens |
| Recursive self-modification | Shadow mode, canary rollout, quarantine, rollback, human approval for authority changes |
| User-attention flooding | Attention budgets, severity thresholds, digesting, decision bundling |
| User under-involvement | Mandatory human gates for high-effect classes and unknown risk |
| Voice ambiguity | Confirm high-effect actions, voice intent confidence, readback, session binding |
| Backbone overload | Tiered streams, sampling, priority lanes, backpressure, local buffering |
| Split brain | Consensus lease holder, epoch fencing, monotonic sequence numbers |
| Data exfiltration | Secret scanners, egress policy, redaction, data-classified leases |
| Supply-chain compromise | SBOM, SLSA provenance, signatures, sandboxed build, reproducible checks |
| Policy capture | Jankurai lesson quarantine, challenge/appeal process, expiry and confidence |
| Observability theater | Evidence graph, trace-task correlation, missing-evidence alerts |
| Planner hallucination | Capability registry, cost model, dry-run, feasibility probes |
| Adversarial repo | Hermetic sandbox, read-only mounts, network denial, resource caps |
| Tool sprawl | Tool inventory, reuse scoring, deprecation, capability map |
| Semantic mismatch | Adapters translate into JPCM effects and evidence taxonomy |
| Stale knowledge graph | Freshness stamps, invalidation on commit, stale-source penalties |
| Runaway optimization | Goal charter, user-value KPIs, periodic alignment review |
| Silent degraded mode | Health leases, degraded-state badges, fail-closed for high effects |
| Temporal inconsistency | Version pinning, schema epochs, rebasing checkpoints |
| Cross-tenant bleed | Tenant isolation, anonymized promotion pipeline, access proofs |
| Unsafe automation | Promotion ladder, staged effects, CI veto, Jankurai proof gate |
| Ambiguous ownership | Every event has responsible authority and owner service |
| Inadequate incident reconstruction | Append-only event log, causal IDs, evidence bundles, retention policy |
| Prompt/tool identity spoofing | mTLS, JWS signatures, service-card registry, key rotation |
| Schema ossification | Semver, capability negotiation, extension namespaces, deprecation windows |
| Over-centralization | Control/read-plane separation, local autonomous execution, replicated read models |


## 15. Acceptance Tests

A build is not acceptable until it passes:

1. schema validation for golden and malicious envelopes
2. signature verification and replay tests
3. lease denial tests for every effect class
4. task-state transition fuzzing
5. false-evidence injection tests
6. context poisoning tests
7. voice ambiguity and confirmation tests
8. high-volume event backpressure tests
9. degraded-mode and split-brain tests
10. self-improvement quarantine and rollback tests
11. automated tool-building supply-chain tests
12. UI drill-down freshness and latency tests

## 16. File Tree

```text
jmcp/
  crates/
    jmcp-core/                 # authority engine, task kernel, user-attention broker
    jpcm/                      # protocol types, schema validation, signing, replay
    jpcm-bus/                  # NATS/Redpanda adapters, priority lanes, backpressure
    jpcm-ledger/               # append-only evidence and decision ledger
    jpcm-policy/               # OPA/Cedar/Jankurai policy adapters
    jpcm-sandbox/              # worker isolation, network/filesystem/cost limits
    jmcp-memory/               # episodic stream, semantic compression, quarantine
    jmcp-toolsmith/            # automated tool discovery/build/eval/promote
    jmcp-voice/                # STT/TTS adapters, confirmation semantics
    jmcp-ui-api/               # GraphQL/gRPC/WebSocket read models for React
  adapters/
    mcp-server-adapter/
    a2a-agent-adapter/
    jankurai-adapter/
    jeryu-adapter/
    jekko-zyal-adapter/
    codex-adapter/
    claude-adapter/
    github-adapter/
    sql-adapter/
    graph-adapter/
    vector-adapter/
  ui/
    jmcp-control-room/         # Vite/React/TypeScript frontend
  schemas/
    jpcm/1.0.0/schema.json
    service-card/1.0.0/schema.json
    conformance/1.0.0/*.json
  policies/
    leases/
    attention/
    self-improvement/
    tool-building/
  tests/
    golden-envelopes/
    malicious-services/
    replay/
    voice/
    disaster-drills/
```

## 17. Build Phases

### Phase 0: Protocol Freeze

Publish `JPCM_FINAL_PROTOCOL_SCHEMA_v1.0.0.json`, service-card schema, golden fixtures, malicious fixtures, and conformance CLI.

### Phase 1: Backbone and Ledger

Implement signed envelope ingest, validation, append-only ledger, replay, and materialized task/attention/service views.

### Phase 2: Authority Kernel

Implement capability leases, effect classes, policy gates, Jankurai/Jeryu/Jekko adapters, and sandbox worker runners.

### Phase 3: Control Room and Voice

Implement React control room, attention queue, drill-down, voice session binding, and readback confirmation.

### Phase 4: Autonomous Work

Enable background bug hunts, duplicate-code reduction, code graph maintenance, research tasks, incident triage, and user digests.

### Phase 5: Self-Improvement and Toolsmith

Enable quarantined self-improvement and automated tool building with proof, provenance, SBOM, canary, and rollback.

## 18. Extreme Future Vector

The extreme version of JMCP becomes a sovereign operating layer for technical organizations: a continuously learning, evidence-backed, human-aligned process-control fabric for all knowledge work. It discovers capability gaps, builds new tools, creates repos, compresses lessons into standards, governs agent swarms, predicts bottlenecks, prevents repeated mistakes, and lets the user operate from strategic altitude. The user no longer manages logs, tickets, flaky agents, scattered repos, or untrusted automation directly. The user manages intent, values, exceptions, and irreversible decisions.

The highest ambition is not full automation. The highest ambition is **trusted autonomy with proof**, where the system does more while asking less, but becomes more accountable as its power increases.
