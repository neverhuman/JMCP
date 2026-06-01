# JMCP FINAL Hostile Review, Scorecard, and V5 Design Pressure

This review scores the uploaded V4/V5 corpus as design inputs for a final JMCP/JPCM architecture. The score measures buildability and survivability under hostile engineering review, not prose quality.

## Rubric

| Criterion | Points | Perfect answer requires |
|---|---:|---|
| Definition and negative definition | 8 | Crisp distinction between JMCP, JPCM, MCP/A2A, CI, dashboards, agents, and ordinary automation. |
| Protocol finality and schema completeness | 18 | Normative versioned schema, payload families, transports, state machines, compatibility, conformance, and examples. |
| Backbone/distributed-systems correctness | 10 | Ordering, delivery, dedupe, idempotency, backpressure, replay, DR, partitions, poison queues, and retention. |
| Task/workflow/scheduler/rollback | 10 | All work represented as WorkOrders/Tasks with priority, budgets, cancellation, rollback, evidence, and escalation. |
| Authority, leases, policy, veto | 12 | Non-ambient capability authority, revocation, policy receipts, leases checked at every side-effect boundary. |
| Evidence/provenance/promotion | 10 | Receipts, artifacts, SLSA/in-toto-style attestations, reproducibility, conflicting evidence, no-proof-no-promotion. |
| Security and adversarial design | 12 | Prompt/tool/memory poisoning, compromised agents, supply chain, sandbox escape, confused deputy, voice attack, cost abuse. |
| Tool/data awareness and self-improvement | 8 | Tool registry, data catalog, technology scout, automated tool-building, and governed self-improvement. |
| User attention, text, voice | 6 | Bare-minimum disclosure, decision packets, voice confirmation and lockout, drill-down without overload. |
| Implementation, evaluation, operations | 6 | File tree, build phases, SLOs, conformance tests, eval suites, incident response, observability. |

## Artifact scores

| Artifact | Score | Feedback |
|---|---:|---|
| `jmcp_cp_v1_envelope.schema.json` | 42 | Useful envelope skeleton, but payload semantics are underspecified, task and self-improvement objects are absent, and conformance cannot be built from it alone. |
| `jmcp_v4_jpcp_v1_envelope_schema.json` | 48 | Better message enum coverage than the first schema, but still not a complete protocol schema: no rigorous object model, no conditional payload typing, and no task/tool-building/self-improvement grammar. |
| `JMCP_V4_ENGINEERING_SPEC.md` | 81 | Strong authority/evidence framing and future vector. Main weakness is that the protocol section reads like a high-quality draft rather than a final, testable normative standard. |
| `JMCP_V4_engineering_spec(1).md` | 84 | Better operational structure and architecture planes. Still leaves adapter conformance, schema versioning, economics, and all-task taxonomy too implicit. |
| `jmcp_v4_protocol_first_engineering_spec.md` | 90 | Strongest V4 spec: protocol-first, hostile critique, and usable build plan. Remaining gaps: JSON schema incompleteness, formal delivery matrix, voice lockout details, and autonomous tool-building lifecycle. |
| `jmcp_v4_protocol_first_engineering_spec(1).md` | 87 | Excellent protocol-first naming and future vector. Slightly less complete on proof economics, data governance, and exact payload definitions. |
| `jmcp_v4_scorecard.md` | 79 | Very helpful critique document. It is not a build spec and cannot serve as the canonical protocol or implementation contract. |
| `JMCP_V4_ieee_whitepaper.tex` | 82 | Good paper-level framing and references. Needs more diagrams, deeper distributed-systems critique, and a stronger normative protocol appendix. |
| `jmcp_v4_ieee_white_paper.tex` | 86 | Broad and ambitious, with failure catalogues and conformance discussion. Needs sharper negative definition and a complete protocol schema artifact. |
| `jmcp_v4_protocol_first_ieee_paper.tex` | 90 | Best paper in the bundle. Still needs finalization into 20-30 pages, stronger diagrams, complete reference set, and a linked JSON schema as protocol law. |
| `jmcp_v4_whitepaper.tex` | 87 | Strong paper with useful protocol object sections. Needs stronger scoring-to-rewrite traceability and more explicit future-state ambition. |

## Cross-artifact verdict

The corpus is now architecturally strong, but the remaining gap is that protocol law must be machine-checkable, not only persuasive. The final package therefore makes `JPCM-1.0.0` the canonical schema, separates authority from interoperability, defines all work as task/evidence/lease state transitions, and makes self-improvement and automated tool-building first-class task families rather than informal ambitions.

## Highest-risk criticisms absorbed into the final design

1. **Authority drift**: a service gains practical ability to act outside its lease because an adapter bypasses JPCM.
2. **Protocol drift**: teams add raw webhooks, local queues, or direct MCP calls that never become auditable JPCM events.
3. **Exactly-once illusion**: duplicate events or tool calls cause repeated side effects when idempotency is missing.
4. **Evidence theater**: CI badges and screenshots are accepted as proof despite being stale, forged, or irrelevant.
5. **Memory poisoning**: a compromised repo, document, or agent plants false lessons that become global policy.
6. **User-attention failure**: the system suppresses a rare but critical decision because it optimizes for quietness.
7. **Voice command ambiguity**: speech recognition turns a low-risk instruction into a high-risk mutation.
8. **Confused deputy**: a low-trust tool routes action through a high-trust service without preserving original authority.
9. **Sandbox escape**: an agent exploits language runtime, package manager, browser, or kernel behavior.
10. **Supply-chain compromise**: a tool, dependency, model package, or MCP server changes behavior after approval.
11. **Silent cost explosion**: background agents recursively spawn work or run high-cost models without budget enforcement.
12. **Rollback fantasy**: a change is treated as reversible even though external systems, users, or data migrations make it hard to unwind.
13. **Observability overload**: telemetry exists but is sampled, uncorrelated, or too expensive for agents to query continuously.
14. **Privacy leakage**: context packets include more data than needed or are retained longer than permitted.
15. **Self-improvement corruption**: JMCP changes its own policies, prompts, scheduler, or tools without independent evaluation.
16. **Tool-building risk**: generated tools are useful but under-tested, over-permissioned, or globally registered too quickly.
17. **Human trust erosion**: too many alerts teach the user to rubber-stamp; too few alerts hide unacceptable autonomy.
18. **Protocol version lock-in**: v1 becomes impossible to evolve because compatibility and extension rules were not defined.
19. **Disaster recovery gap**: event streams restore tasks but not leases, object evidence, graph state, or UI decisions coherently.
20. **Adversarial incentives**: agents optimize visible metrics while degrading maintainability, safety, or long-term yield.

## Design corrections made in the FINAL package

- JPCM is now a versioned JSON Schema artifact, not just a prose protocol section.
- JPCM treats all services, users, voice surfaces, adapters, agents, datastores, tool calls, memory writes, and self-improvement actions as signed envelopes.
- All side effects require a valid capability lease checked at the adapter boundary.
- All promotion requires evidence bundles with verifier identity, artifact digests, and policy receipts.
- Tool building, technology scouting, and self-improvement are explicit task types with proposal, experiment, evaluation, promotion, and rollback states.
- The communication backbone assumes at-least-once delivery and requires idempotency, dedupe, replay, poison queues, and backpressure.
- The user-attention layer is treated as a safety system with false-quiet tests, not just a nicer notification UX.
- Voice is an untrusted input channel until normalized, transcribed, risk-scored, and confirmed when necessary.