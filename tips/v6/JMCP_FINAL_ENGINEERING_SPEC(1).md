# JMCP Final Engineering Specification

**Artifact status:** final V5 critique-hardened build specification.  
**Protocol:** JPCM-1.0.0, the mandatory communication protocol for every service, agent, adapter, tool, data source, voice surface, UI, and user-attention surface that interacts with the Joint Master Control Plane.  
**Primary implementation stack:** Rust control services, NATS/JetStream or equivalent message backbone, SQL + object store + graph/NoSQL indexes, Vite/TypeScript/React cockpit, sandboxed adapters for Jankurai, Jeryu, Jekko, MCP tools, A2A agents, OpenClaw-like user channels, and external model providers.

## 0. Executive decision

JMCP is not an assistant, not a coding agent, not a dashboard, not a CI wrapper, not an MCP server, and not a generic automation platform. JMCP is the **sovereign authority, evidence, attention, and process-control plane** for autonomous engineering and research work. It owns the right to create work orders, issue and revoke leases, accept or reject evidence, decide whether a user must be interrupted, decide which tools exist, decide which tools should be built next, and decide whether any agent output may become durable code, memory, policy, or production state.

JPCM-1.0.0 is the protocol that makes this enforceable. Every service speaks JPCM. Native tools speak JPCM. Bridges translate MCP, A2A, OpenClaw channels, Jankurai, Jeryu, Jekko, SQL stores, noSQL stores, voice surfaces, browsers, CI systems, model routers, and repo-local tools into JPCM. A service that cannot emit valid JPCM envelopes is not part of the controlled system; it is quarantined external input.

The core design goal is radical user leverage: **the user sees the minimum safe amount of information**. JMCP watches everything, drills down before asking, summarizes only what matters, and escalates only decisions where silence is more dangerous than interruption.

## 1. Hostile review rubric

| Criterion | Points | Perfect answer requires |
|---|---:|---|
| Definition and negative definition | 8 | States exactly what JMCP owns, what it refuses to be, and which authority boundaries are non-negotiable. |
| Authority, leases, and containment | 10 | All side effects are non-ambient, revocable, scoped, budgeted, policy checked, and fail closed. |
| Protocol normativity and interoperability | 15 | Versioned schema, envelope rules, transports, ordering, retries, idempotency, errors, conformance classes, and compatibility rules. |
| Task, workflow, and self-improvement model | 12 | Typed work orders, DAGs, state machines, locks, budgets, rollback, escalation, self-improvement, and tool-building loops. |
| Security and adversarial design | 12 | Prompt/tool/memory poisoning, compromised agents, supply chain, sandbox escape, voice spoofing, confused deputy, and control-plane compromise. |
| Evidence, provenance, and certification | 10 | Receipts, reproducibility, independent verification, evidence quality, signed bundles, no-proof-no-promotion, and certification tests. |
| Tool and data awareness | 8 | Registry for tools, data, models, costs, latency, reliability, risk, ownership, build-vs-buy, and technology radar. |
| User attention, text, and voice UX | 8 | Bare-minimum escalation, attention packets, risk-of-silence scoring, voice confirmation, and user trust calibration. |
| Observability, performance, and backpressure | 6 | Machine telemetry, causal traces, bottlenecks, adaptive sampling, replay, latency budgets, and overload behavior. |
| Implementation buildability | 5 | Concrete Rust/Vite/TypeScript/React file tree, milestones, tests, storage, deployment, and operating model. |
| Research grounding and ambition | 4 | Accurate adjacent-work positioning and a future vector that becomes product requirements, not vague futurism. |
| Clarity and maintainability | 2 | Readable enough that multiple teams can implement compatible services without oral tradition. |

## 2. Scorecard for the uploaded V5 artifact bundle

| Artifact | Kind | Score | Strongest part | Critical feedback |
|---|---:|---:|---|---|
| `JMCP_V4_ENGINEERING_SPEC.md` | Engineering spec | 87 | Strong executive framing, is/is-not boundary, and early protocol model. | Still too permissive around schema extensions, lacks exhaustive payload definitions, and does not fully specify self-improvement/tool-building tasks. |
| `JMCP_V4_engineering_spec(1).md` | Engineering spec | 89 | Best plane decomposition and authority framing among the non-protocol-first specs. | Needs a harder conformance story, adapter certification, and mandatory message semantics for every service class. |
| `jmcp_v4_protocol_first_engineering_spec.md` | Engineering spec | 92 | Strongest V4 spec: scorecard, hostile critique, JPCM-1 positioning, and task workflow coverage. | Needs a complete schema, explicit task-kind universe, semantic constraints not expressible in JSON Schema, and a more formal voice/user-attention contract. |
| `jmcp_v4_protocol_first_engineering_spec(1).md` | Engineering spec | 90 | Good protocol-first structure, future-vector staircase, and core envelope semantics. | Less complete than the main protocol-first spec; weaker scorecard and thinner conformance/evaluation plan. |
| `JMCP_V4_ieee_whitepaper.tex` | Paper | 86 | Clear critique narrative and good separation of JMCP from MCP/A2A/OpenClaw. | Too short for a final reference paper; protocol is described but not made fully normative, and diagrams/evaluation depth are thin. |
| `jmcp_v4_ieee_white_paper.tex` | Paper | 88 | Strong related-work span, useful failure catalogue, and improved communication-backbone framing. | Needs final schema alignment, more figures, deeper task semantics, and stronger claims about what would falsify the architecture. |
| `jmcp_v4_protocol_first_ieee_paper.tex` | Paper | 91 | Best V4 paper: protocol-first thesis, threat model, leases, evidence, attention, and tool/data awareness. | Still 10 pages rather than the requested final 20-30; references and diagrams are not enough for a definitive standard paper. |
| `jmcp_v4_whitepaper.tex` | Paper | 89 | Good detailed object model and protocol object discussion. | Needs tighter normative language, more adversarial evaluation, final JSON schema mapping, and less overlap with engineering-spec prose. |
| `jmcp_v4_scorecard.md` | Scorecard | 82 | Useful rubric and risk list that forced V4 in the right direction. | Not a buildable artifact; does not score the V5 bundle, and does not convert every criticism into enforceable requirements. |
| `jmcp_cp_v1_envelope.schema.json` | JSON Schema | 71 | Valuable seed for signed envelopes and core event families. | Too envelope-only. Missing service manifests, work orders, tasks, workflows, evidence bundles, attention packets, voice segments, tool/data registry, self-improvement, tool-building, and conformance metadata. |
| `jmcp_v4_jpcp_v1_envelope_schema.json` | JSON Schema | 73 | Improves the envelope and risk/delivery fields. | Still under-specified as a protocol standard. It cannot validate the actual universe of JPCM control objects or task lifecycle events. |

## 3. Cross-artifact verdict

The V5 bundle is strong enough to define the product category but not yet sufficient to define the protocol ecosystem. The best paper and engineering spec correctly identify JMCP as an authority-and-evidence control plane rather than a model-router, assistant, or dashboard. They also correctly elevate leases, evidence bundles, attention minimization, and tool/data awareness. However, the prior schema remains mostly an envelope. A real standard must validate the objects that the envelope carries: work orders, tasks, workflows, leases, service manifests, tool manifests, evidence bundles, attention packets, voice segments, data assets, memory proposals, policy decisions, self-improvement proposals, and tool-building proposals.

The decisive V5 correction is therefore: **JPCM-1.0.0 is not just an envelope. It is the contract for the entire controlled universe.**

## 4. What JMCP is

JMCP is:

- A supervisory control plane for autonomous engineering, research, repo operations, tooling, and knowledge work.
- A policy-enforced authority kernel that grants leases instead of ambient permissions.
- An evidence operating system that treats every claim as untrusted until supported by reproducible proof.
- An attention firewall that decides what the user must see, what can be summarized, and what should be silently handled.
- A communication backbone consumer and producer that makes all service activity observable, replayable, and governed.
- A task/workflow manager for parallel autonomous work, including bugs, features, incidents, research, repo creation, tool building, and self-improvement.
- A tool/data/model awareness system that knows what it has, how well it works, what it costs, what it risks, and what it should build next.
- A memory and knowledge-compression system with provenance, TTL, scope, confidence, counterexamples, and rollback.
- A user interface spanning text, voice, and React drilldown surfaces.

## 5. What JMCP is not

JMCP is not:

- A chatbot. It may expose chat, but chat is only one surface into the control plane.
- A voice assistant. Voice is an input modality, not an authority boundary.
- A coding agent. Jekko, Codex, Claude, OpenHands-like agents, and other workers may code; JMCP supervises them.
- A CI system. Jeryu and CI execute checks; JMCP decides which evidence is sufficient.
- A repo standard checker. Jankurai supplies proof lanes and standards; JMCP decides when those receipts matter globally.
- An MCP server. MCP tools are imported through a bridge and treated as untrusted until certified.
- An A2A agent. A2A agents are external workers with manifests and leases, not peers to the authority kernel.
- A log dashboard. Logs are raw material. JMCP produces causal diagnosis, attention packets, evidence, and decisions.
- A self-modifying authority without brakes. Self-improvement is allowed only through shadow, canary, evidence, rollback, and approval gates.

## 6. Exhaustive hostile critique: everything that can go wrong

### Authority collapse
- A service bypasses JMCP and writes directly to git, SQL, cloud, or production.
- A privileged adapter accumulates ambient credentials and becomes a shadow root.
- Emergency pause does not propagate to running agents.
- A self-improvement task modifies the policy engine that judges it.
- Leases are checked at dispatch but not at every side-effect boundary.
- A stale lease is replayed against a different repo, tenant, or policy epoch.

### Protocol failure
- Services implement lookalike envelopes with missing fields.
- Unversioned payloads cause silent semantic drift.
- Duplicate messages create duplicate PRs, migrations, or tool builds.
- Ordering assumptions fail under retries, partitions, or replay.
- Backpressure causes the bus to drop the evidence required for later audit.
- Schema permissiveness lets malicious extensions smuggle commands.

### Agent/model failure
- A model follows instructions embedded in code, logs, web pages, tickets, or tool descriptions.
- The router chooses a cheaper model for a high-risk job.
- Agents collude accidentally through shared memory or hidden context.
- A tool result is summarized into a false claim that survives into memory.
- A long-horizon task drifts from the user intent.
- Multi-agent decomposition creates duplicated, conflicting work.

### Evidence failure
- Green tests prove only the changed happy path.
- Screenshots and logs become evidence theater.
- CI is spoofed by a compromised runner.
- Coverage drops while the change still passes.
- Evidence is not bound to code hash, environment, input, policy epoch, and lease.
- A proof bundle cannot be replayed after dependencies disappear.

### Memory and learning failure
- A poisoned lesson contaminates every future repo.
- A local workaround is promoted as a global standard.
- Old lessons persist after the underlying toolchain changes.
- Contradictory lessons are compressed into a misleading summary.
- The system forgets why a decision was made.
- Memory confidence grows without new evidence.

### Tool/data awareness failure
- The registry says a tool exists but not what data it can touch.
- A high-value missing tool is never built because it is not measured.
- Cost, latency, failure rate, and blast radius are not part of routing.
- A tool manifest is signed but its binary or container is swapped.
- Dataset freshness is mistaken for truth.
- Cross-repo knowledge leaks secrets, licenses, customer data, or unreleased strategy.

### User attention failure
- The system hides an irreversible decision to save attention.
- The user receives too many low-value alerts and stops reading.
- A voice command is misheard, replayed, spoofed, or socially engineered.
- The approval UI lacks diff, blast radius, rollback, and alternatives.
- JMCP asks the user to make decisions it could prove itself.
- A high uncertainty state is misclassified as low risk.

### Operational failure
- Telemetry costs exceed compute costs.
- A single broker outage freezes all work.
- Clock skew breaks expiry and ordering assumptions.
- The event ledger grows without retention tiers.
- Debugging the control plane requires the control plane itself.
- The React cockpit becomes a wall of traces instead of a decision surface.

### Legal/compliance failure
- Durable memory stores regulated data without retention rules.
- Autonomous agents change license-sensitive code.
- Evidence bundles contain secrets.
- Voice recordings are retained longer than expected.
- Audit trails are incomplete for an external review.
- A generated tool introduces export-control, privacy, or accessibility obligations.

### Future-vector failure
- Ambition outruns certification.
- Self-improvement optimizes for benchmark wins instead of user value.
- The tool-building loop creates unmaintained tools faster than they can be audited.
- Federation spreads poisoned standards between organizations.
- The system becomes so complex that only itself can explain itself.
- The product stops being a user-time saver and becomes a new operating burden.

## 7. Non-negotiable system laws

1. **No side effect without a lease.** A command that can write files, mutate repos, spend money, access secrets, call production, modify memory, modify policy, build a tool, or change the authority kernel requires a valid scoped lease at execution time.
2. **No claim without evidence.** Status, completion, promotion, memory, and policy claims must link to evidence. Evidence has quality levels and can be challenged.
3. **No hidden durable learning.** Durable memory is a change-controlled artifact with scope, TTL, confidence, counterexamples, and rollback.
4. **No tool by reputation.** Every tool has a manifest, audit result, sandbox profile, cost model, data-touch model, owner, and conformance level.
5. **No user interruption without an attention packet.** The system must state why silence is riskier than interruption, what decision is needed, what the options are, and what it recommends.
6. **No voice shortcut.** Voice can express intent. Risky voice commands require transcript review/readback or an equivalent explicit confirmation path.
7. **No self-improvement of the judge by the judged.** Self-improvement that touches policy, schema, scheduler, router, sandbox, or authority requires isolation, canary, evidence, and quorum.
8. **No bypass.** All mutable side effects go through JPCM command gateways. Direct credentials inside tools are treated as violations.
9. **No silent protocol drift.** Versioning, conformance tests, golden fixtures, and compatibility rules are part of the standard.
10. **No cockpit burden.** Drilldown exists, but the default experience is exception-first, decision-first, and summary-first.

## 8. Reference architecture

JMCP has seven planes:

1. **Authority Plane:** root policy, leases, approvals, emergency pause, risk tiers, signing keys, quorum.
2. **Communication Plane:** JPCM envelopes, message backbone, ledger, replay, transport bindings.
3. **Workflow Plane:** work orders, tasks, DAGs, scheduling, WIP, dedupe, budgets, cancellation, rollback.
4. **Evidence Plane:** evidence bundles, receipts, provenance, replay, verification, challenge, promotion.
5. **Tool/Data/Model Awareness Plane:** registries, manifests, cost/risk/performance telemetry, technology radar, build-vs-buy.
6. **Cognition and Execution Plane:** Jekko/ZYAL, Codex, Claude, OpenHands-like agents, shell tools, CI, browsers, data stores, and generated tools as leased workers.
7. **User Plane:** text, voice, React cockpit, attention packets, approvals, digests, drilldown.

## 9. Communication backbone

The backbone is a **bus + ledger + object store + policy gate**. The recommended local-first binding is NATS with JetStream because it combines subject-based messaging, request/reply, pub/sub, persistence, replay, key-value, and object storage in a lightweight fabric. Kafka or Redpanda can be used for larger organizational deployments; HTTP+JSON, gRPC, and WebSocket are binding profiles for external clients and UI surfaces. The protocol is independent of the broker.

Backbone streams:

- `jpcm.authority.*`: lease issuance, revocation, policy decisions, approvals, emergency pause.
- `jpcm.intent.*`: text, voice, webhook, scheduled, and system-detected intents.
- `jpcm.task.*`: work orders, task states, progress, locks, cancellation, rollback.
- `jpcm.tool.*`: tool manifests, audit results, calls, tool-building proposals, deprecation.
- `jpcm.data.*`: data assets, data access requests, observations, lineage.
- `jpcm.evidence.*`: bundles, items, verification, challenge, promotion.
- `jpcm.memory.*`: memory candidates, proposals, accepted/rejected/revoked memories.
- `jpcm.attention.*`: candidates, packets, escalations, receipts.
- `jpcm.telemetry.*`: traces, metrics, logs, costs, budget events.
- `jpcm.self.*`: self-improvement proposals, experiments, canaries, rollbacks.

Backbone rules:

- D0 ephemeral telemetry may be sampled and dropped.
- D1 at-most-once events are advisory.
- D2 at-least-once events require idempotency keys.
- D3 effectively-once events require idempotency, dedupe windows, and replay-safe handlers.
- D4 authority-serialized events require a single authority ordering key and ledger commit before side effect.

## 10. JPCM-1.0.0 protocol standard

JPCM messages are canonical JSON objects validated by `jpcm_1_0_0_protocol.schema.json`. Every envelope has a version, message id, message type, subject, timestamps, producer, trace context, delivery semantics, risk context, policy context, schema reference, payload hash, payload, optional evidence refs, optional lease refs, redaction policy, optional signature, and extensions.

### 10.1 Envelope requirements

- `jpcm_version` MUST equal `1.0.0` for this version.
- `message_id` MUST be globally unique.
- `subject` MUST use the `jpcm.*` namespace.
- `payload_hash` MUST be computed over canonical JSON using RFC 8785 JSON Canonicalization Scheme unless a binding profile states otherwise.
- Side-effecting messages MUST include `lease_refs` and MUST fail closed if the lease is missing, expired, revoked, out-of-scope, over budget, or issued under a stale policy epoch.
- R4, R5, and R6 risk messages SHOULD be D4 authority serialized. R6 MUST be D4.
- Signatures MUST cover all required envelope fields and payload hash for authority, lease, evidence, memory, tool-build, self-improvement, and policy messages.
- Extensions MAY add metadata but MUST NOT change the meaning of required fields.

### 10.2 Conformance levels

- **C0 Observer:** emits valid heartbeats and telemetry; no side effects.
- **C1 Reporter:** reports state and evidence; no commands.
- **C2 Leased Actor:** accepts leases and executes bounded tasks.
- **C3 Evidence Actor:** produces replayable evidence bundles and supports challenge.
- **C4 Authority Participant:** participates in policy, approval, ledger, or signing workflows.
- **C5 Self-Improving Participant:** proposes tool, memory, policy, scheduler, or authority improvements under quarantine, canary, rollback, and quorum rules.

## 11. Task and workflow management

A user, voice surface, service, scheduler, incident detector, or technology radar may create an intent. JMCP converts valid intents into work orders. Work orders contain typed tasks. Tasks form a DAG and have explicit states:

`proposed -> triaged -> planned -> leased -> dispatched -> running -> waiting_for_evidence -> reviewing -> verifying -> ready_to_promote -> promoting -> completed`.

Alternate states include `waiting_for_user`, `blocked`, `paused`, `failed`, `canceled`, `rolled_back`, and `quarantined`.

Task kinds include investigation, bug fix, feature, refactor, security review, incident response, repo creation, research, knowledge compression, memory promotion, tool build, tool audit, self improvement, data ingestion, telemetry analysis, test generation, dependency update, performance optimization, cost reduction, user follow-up, policy change, migration, release, rollback, documentation, prototype, sandbox experiment, deprecation, scheduled maintenance, audit, red team, and model evaluation.

Scheduling policy combines priority, risk, user value, cost, dependency readiness, WIP limits, dedupe keys, lock conflicts, confidence, and deadline. The scheduler must prefer finishing high-confidence high-value tasks over spawning unbounded parallel agents.

## 12. User attention firewall

JMCP minimizes user interruptions through a risk-of-silence score. It escalates only when one or more conditions are true:

- The action is irreversible or hard to roll back.
- The blast radius includes production, secrets, regulated data, cross-repo standards, durable memory, policy, or authority.
- The system cannot distinguish between valid alternatives using available evidence.
- The user has explicitly reserved a decision class.
- The system detects a safety, security, legal, budget, or trust boundary violation.
- Silence would hide a material change in project scope, deadline, cost, or risk.

Attention packets include decision, options, recommendation, evidence, blast radius, cost, rollback, deadline, and drilldown links. The UI default is an attention inbox, not a raw log stream.

## 13. Text and voice chat

Text and voice are peer input surfaces, but voice has a larger threat model. Voice messages are processed as:

1. `voice.segment.created`: raw segment metadata, source, timing, confidence, ambient risk.
2. `voice.transcript.candidate`: transcript and confidence.
3. `voice.intent.candidate`: proposed intent with ambiguity analysis.
4. `attention.packet.created` if confirmation is required.
5. `work_order.created` only after the attention or policy gate is satisfied.

R0 and R1 voice intents may proceed after transcript confidence thresholds. R2+ voice intents require readback or equivalent explicit confirmation. R4+ voice intents require text-visible confirmation unless a local policy defines a stronger biometric and physical-presence check.

## 14. Tool and data awareness

JMCP maintains registries for services, tools, agents, data assets, models, prompts, workflows, policies, schemas, sandboxes, and generated tools. Each entry records owner, version, hash, SBOM, provenance, conformance level, capabilities, side effects, touched data classes, leases required, evidence required, cost model, latency SLO, reliability, audit status, deprecation status, and alternatives.

Tool awareness is not passive inventory. JMCP continuously computes a technology radar:

- Which tools are missing but repeatedly needed?
- Which tools create too much user interruption?
- Which tools are slow, costly, unreliable, or risky?
- Which repo-specific scripts should become global Jankurai tools?
- Which global tools should be deprecated because they produce weak evidence?
- Which model/router/sandbox upgrades would increase task yield?

A tool-building task is allowed only through `tool.build.proposed`, a tool manifest, an evaluation plan, sandbox profile, Jankurai/Jeryu evidence, and maintenance owner.

## 15. Evidence and promotion

Evidence quality levels:

- **E0 claim-only:** not sufficient for promotion.
- **E1 single-source:** one log, model summary, screenshot, or unchecked result.
- **E2 reproducible:** commands and environment can rerun the result.
- **E3 independent:** a separate verifier reproduces or checks the claim.
- **E4 adversarial:** red-team, fuzzing, mutation, negative tests, or challenge passed.
- **E5 production-correlated:** validated against production telemetry, incidents, or real user outcomes.

Promotion of code, memory, standards, policy, or tools requires the evidence level specified by policy. No-proof-no-promotion is stronger than no-proof-no-merge: it applies to durable memory, tool adoption, policy updates, and self-improvement.

## 16. Self-improvement and automated tool building

JMCP should always improve, but improvement is treated as a hazardous work class. The self-improvement loop is:

1. Detect bottleneck, repeated failure, expensive pattern, missing tool, weak evidence, or user-interruption waste.
2. Create `self_improvement.proposed` or `tool.build.proposed` with hypothesis, target, risk, evaluation plan, rollback, and approval gate.
3. Run in shadow mode or sandbox experiment.
4. Produce evidence and compare against baseline.
5. Canary with bounded traffic if safe.
6. Promote only after policy, evidence, and attention gates.
7. Monitor harm metrics and rollback automatically if violated.

The authority kernel, schema, policy engine, scheduler, sandbox, and model router are protected targets. Improvements to these targets require quorum, independent evidence, and explicit rollback.

## 17. Security architecture

The threat model assumes compromised inputs, compromised agents, malicious tool manifests, malicious repo content, compromised CI runners, stale or forged evidence, prompt injection, memory poisoning, voice spoofing, confused deputies, credential leaks, and partial control-plane compromise.

Required controls:

- Minimal authority kernel and isolated signing keys.
- Capability leases instead of ambient credentials.
- Sandboxed adapters with no direct credentials beyond their lease.
- Signed manifests and signed evidence bundles.
- Secret scanning and redaction before evidence storage.
- Policy simulation for high-risk actions.
- Prompt-injection containment by treating model outputs as proposals, never authority.
- Quarantine for new tools, new agents, and new memories.
- Emergency pause and revocation propagation.
- Replayable decisions and immutable ledger.

## 18. React cockpit

The React frontend is an exception-first cockpit:

- **Attention Inbox:** decisions that need the user.
- **System Map:** services, health, trust tier, leases, current incidents.
- **Work Graph:** active work orders, task DAG, blockers, WIP, budgets.
- **Evidence Drilldown:** claims, evidence quality, replay, challenge, provenance.
- **Tool/Data Radar:** inventory, gaps, degradation, build proposals, cost/risk.
- **Voice Console:** transcripts, confirmations, ambiguity, receipts.
- **Incident Room:** fault trees, timelines, blast radius, rollback.

It must support rapid drilldown, but the product goal is that JMCP drills down first.

## 19. Storage model

- **SQL:** work orders, tasks, leases, service registry, policy decisions, approvals, scorecards.
- **Event ledger:** immutable JPCM envelope stream with replay and snapshots.
- **Object store:** evidence artifacts, logs, audio segments, screenshots, diffs, build outputs.
- **Graph store:** code graph, dependency graph, task graph, causal graph, memory graph.
- **Vector/search index:** retrieval over evidence, lessons, docs, incidents, transcripts, and repo knowledge.
- **Secret store:** external KMS/vault; never store secrets in JPCM payloads.

## 20. Implementation phases

1. **Phase 0:** implement JPCM types, schema validation, NATS/HTTP bindings, ledger, and conformance fixtures.
2. **Phase 1:** authority kernel, lease issuance/revocation, policy decisions, emergency pause.
3. **Phase 2:** work orders, task DAG, scheduler, budgets, locks, cancellation, rollback.
4. **Phase 3:** adapters for Jankurai, Jeryu, Jekko, MCP bridge, A2A bridge, SQL/noSQL, CI.
5. **Phase 4:** evidence bundles, replay, challenge, proof-quality gates.
6. **Phase 5:** attention firewall, text/voice, React cockpit.
7. **Phase 6:** tool/data/model registry, technology radar, automated tool-building workflow.
8. **Phase 7:** memory governance, knowledge compression, self-improvement canary system.
9. **Phase 8:** federation, cross-organization evidence exchange, proof-carrying tool marketplace.

## 21. Final file tree


```text
jmcp/
  Cargo.toml
  crates/
    jmcp-authority/             # minimal trusted kernel: leases, policies, approvals, emergency pause
    jmcp-jpcm/                  # Rust types generated from jpcm_1_0_0_protocol.schema.json
    jmcp-bus/                   # NATS/JetStream, HTTP+JSON, gRPC, WebSocket bindings
    jmcp-ledger/                # append-only event ledger, replay, snapshots, evidence index
    jmcp-policy/                # OPA/Rego or embedded policy engine, policy epoch manager
    jmcp-scheduler/             # work-order DAGs, WIP limits, budgets, locks, priority, cancellation
    jmcp-attention/             # attention firewall, text/voice escalation, approval packets
    jmcp-registry/              # service/tool/data/model registry and technology radar
    jmcp-evidence/              # evidence bundles, replay commands, provenance verification
    jmcp-memory/                # scoped memory, knowledge compression, TTL, contradiction handling
    jmcp-self-improve/          # self-improvement experiments, canary, rollback, quarantine
    jmcp-sandbox/               # process/container/VM/WebAssembly execution isolation
    adapters/
      jankurai-adapter/         # proof lanes, audit receipts, standards, repo lessons
      jeryu-adapter/            # git, CI, code graph, repository reality, production promotion
      jekko-adapter/            # ZYAL workflows, agent scripting, model/router integration
      mcp-bridge/               # imports MCP tools as quarantined JPCM tools
      a2a-bridge/               # imports A2A agents as leased external agents
      openclaw-bridge/          # optional text/voice/channel surface, never authority
  services/
    authorityd/
    schedulerd/
    registryd/
    evidenced/
    attentiond/
    voiced/
    replayd/
  ui/
    package.json
    vite.config.ts
    src/
      app/
      components/AttentionInbox.tsx
      components/SystemMap.tsx
      components/TaskGraph.tsx
      components/EvidenceDrilldown.tsx
      components/ToolRadar.tsx
      components/VoiceConsole.tsx
      workers/trace-index.worker.ts
  schemas/
    jpcm_1_0_0_protocol.schema.json
  conformance/
    fixtures/
    golden/
    fuzz/
    red-team/
  docs/
    JMCP_FINAL_ENGINEERING_SPEC.md
    JMCP_FINAL_IEEE_PAPER.tex
```


## 22. Conformance test suite

The conformance suite must include:

- Schema validation for every message family.
- Golden envelope fixtures for each delivery class and risk tier.
- Invalid examples for missing leases, stale policy epochs, bad signatures, payload hash mismatch, and forbidden extensions.
- Retry, duplicate, reordering, and replay tests.
- Lease revocation propagation tests.
- Voice ambiguity and readback tests.
- Prompt injection and tool poisoning fixtures.
- Evidence spoofing and false-green CI fixtures.
- Memory poisoning and contradiction fixtures.
- Self-improvement rollback and canary fixtures.
- Performance and backpressure tests at target event rates.

## 23. Extreme future vector

JMCP starts as a personal engineering control plane. Its extreme vector is a **sovereign process-intelligence fabric**:

1. Personal command surface for text/voice engineering tasks.
2. Autonomous software fab for repo maintenance, bug reduction, and redundant-code elimination.
3. Cross-repo learning system that turns local lessons into global standards through Jankurai.
4. Autonomous R&D foundry that researches, prototypes, benchmarks, and creates new repos.
5. Proof-carrying tool marketplace where generated tools ship with manifests, evidence, and kill switches.
6. Engineering digital twin that models bottlenecks, risks, dependencies, and yield across all work.
7. Self-hardening organization OS that reduces meetings, tickets, logs, and dashboards into decision packets.
8. Federated evidence economy where organizations exchange proofs without exposing sensitive internals.

The ambition is extreme, but the invariant is simple: **more autonomy is permitted only when evidence, containment, and user-attention quality improve faster than agency.**

## 24. Deliverables in this final pass

- `JMCP_FINAL_ENGINEERING_SPEC.md` - this specification.
- `JMCP_FINAL_IEEE_PAPER.tex` - 20-30 page IEEE-style paper with diagrams and references.
- `JMCP_FINAL_IEEE_PAPER.pdf` - compiled PDF.
- `jpcm_1_0_0_protocol.schema.json` - complete versioned JSON schema.


## 25. JPCM normative rule appendix

The following semantic rules are part of JPCM-1.0.0 even where JSON Schema cannot enforce them:

1. Every controlled participant publishes a service manifest before doing controlled work.
2. Every side-effecting adapter rechecks leases immediately before effect execution.
3. R6 self-modifying-authority messages are D4 authority-serialized.
4. Payload hashes use canonical JSON (JCS) unless a stronger binding profile is approved.
5. Voice cannot create risky work directly; it creates transcript and intent candidates first.
6. Tool descriptions are untrusted data, even when signed.
7. External protocols such as MCP and A2A are bridge inputs, not authority peers.
8. Memory promotion requires evidence, scope, confidence, TTL, counterexamples, and rollback.
9. Self-improvement cannot approve itself or alter its own evaluation criteria.
10. Generated tools start quarantined and earn trust through evidence.
11. Policy decisions record epoch, ruleset refs, reason, and obligations.
12. Evidence challenges are first-class messages and can downgrade or revoke promotion.
13. Emergency pause revokes high-risk active leases and blocks new D4 side effects.
14. Backpressure may drop or sample telemetry but never leases, revocations, approvals, evidence bundles, or policy decisions.
15. The cockpit must always distinguish claim, evidence, recommendation, and decision.

## 26. Protocol object quick reference

- **ServiceManifest:** identity, interface bindings, capabilities, touched data, side effects, sandbox, SBOM, conformance.
- **WorkOrder:** objective, non-goals, acceptance criteria, budget, attention policy, task graph.
- **Task:** kind, state, owner, dependencies, locks, leases, evidence requirements, rollback, progress.
- **WorkflowGraph:** nodes, edges, WIP limits, scheduler.
- **CapabilityLease:** principal, capabilities, scope, constraints, expiry, use count, policy epoch.
- **ToolManifest:** inputs, outputs, side-effect class, required leases, evidence requirements, cost, latency, owner.
- **DataAsset:** classification, owner, location, schema, freshness, retention, allowed uses, lineage.
- **EvidenceBundle:** claim, subject refs, artifacts, hashes, quality, verdict, replay command.
- **AttentionPacket:** reason, decision, options, recommendation, deadline, minimum context, drilldown.
- **VoiceSegment:** source, transcript, confidence, speaker verification, ambient risk, confirmation requirement.
- **MemoryProposal:** scope, claim, evidence, confidence, TTL, counterexamples, rollback, promotion gate.
- **ToolBuildProposal:** problem, candidate manifest, expected value, risk, evaluation plan, owner, sunset condition.
- **SelfImprovementProposal:** target, hypothesis, change class, evaluation, rollback, approval, success and harm metrics.

## 27. Required scenarios for acceptance testing

A V1 implementation is not accepted until it passes these scenarios:

1. User asks for a bug fix by text; JMCP creates work order, leases agent, collects evidence, opens PR draft.
2. User asks for a risky deployment by voice; JMCP generates transcript, intent candidate, readback, attention packet, and waits for confirmation.
3. MCP tool includes hidden malicious instructions; bridge quarantines it and records tool poisoning evidence.
4. Agent tries to write outside lease scope; adapter denies and emits policy violation.
5. CI passes with weak tests; evidence challenge prevents promotion.
6. Memory candidate lacks TTL and counterexamples; memory gate rejects it.
7. Technology radar proposes a generated migration checker; tool-building workflow runs in sandbox and canary.
8. Scheduler self-improvement proposes a new priority model; shadow evaluation runs, harm metrics are checked, and deployment requires quorum.
9. Broker backpressure occurs; low-value telemetry is sampled but lease revocation still arrives.
10. Emergency pause is triggered; active high-risk leases are revoked and D4 side effects stop.

## 28. Final protocol deliverable

The final schema file is `jpcm_1_0_0_protocol.schema.json`. It validates the JPCM envelope and the required payload families for service manifests, work orders, workflow graphs, tasks, capability leases, tool manifests, tool-build proposals, data assets, evidence bundles, attention packets, voice segments, memory proposals, self-improvement proposals, policy decisions, conformance claims, and errors.
