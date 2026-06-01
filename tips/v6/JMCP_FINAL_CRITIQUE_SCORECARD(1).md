# JMCP/JPCM Final Hostile Critique and Scorecard

This scorecard treats every uploaded V4/V5 artifact as attackable. Scores are not a reward for ambition; they measure readiness to become a control-plane standard that can safely govern services, agents, tools, data, voice, self-improvement, and automated tool construction.

## Rubric

- **Problem definition and non-goals**: 10 points
- **Protocol finality and machine-testability**: 16 points
- **Task/workflow semantics**: 12 points
- **Security, safety, and authority control**: 16 points
- **Evidence/provenance and observability**: 12 points
- **User-attention and voice/text interaction**: 8 points
- **Tool/data awareness and autonomous tool building**: 8 points
- **Failure handling and disaster modes**: 8 points
- **Implementation specificity and conformance tests**: 10 points

## Artifact Scores

| Artifact | Type | Score | Main criticism | Required correction in final |
|---|---:|---:|---|---|
| `JMCP_V4_ENGINEERING_SPEC.md` | Spec/scorecard | 86/100 | Strong architecture and evidence posture, but protocol obligations are still partially narrative and not enough negative conformance tests exist. | Make every critical behavior normative, testable, signed, versioned, and failure-aware. |
| `JMCP_V4_engineering_spec(1).md` | Spec/scorecard | 88/100 | Better operational detail and task flow, but still needs a fully normative wire schema and service-card negotiation rules. | Make every critical behavior normative, testable, signed, versioned, and failure-aware. |
| `JMCP_V4_ieee_whitepaper.tex` | Paper | 85/100 | Clear paper structure, but insufficient page-level depth on protocol compatibility and formal task semantics. | Expand into a 20-30 page IEEE-style standard paper with diagrams, stronger related work, falsifiable evaluation, and 50-60 references. |
| `jmcp_cp_v1_envelope.schema.json` | Schema | 63/100 | Useful seed envelope, but too small for final use: it lacks full payload taxonomy, service cards, user-attention contract, leases, and task semantics. | Promote from envelope fragment to full versioned protocol schema with payload taxonomy, leases, service cards, task lifecycle, evidence, attention, and conformance. |
| `jmcp_v4_ieee_white_paper.tex` | Paper | 88/100 | Mature paper with good related work; needs stronger diagrams, more references, and clearer falsifiable claims. | Expand into a 20-30 page IEEE-style standard paper with diagrams, stronger related work, falsifiable evaluation, and 50-60 references. |
| `jmcp_v4_jpcp_v1_envelope_schema.json` | Schema | 64/100 | Slightly broader seed schema, but still not a complete service protocol or validation surface. | Promote from envelope fragment to full versioned protocol schema with payload taxonomy, leases, service cards, task lifecycle, evidence, attention, and conformance. |
| `jmcp_v4_protocol_first_engineering_spec(1).md` | Spec/scorecard | 87/100 | Good control-plane framing, but user-attention, voice, and offline/degraded modes remain under-specified. | Make every critical behavior normative, testable, signed, versioned, and failure-aware. |
| `jmcp_v4_protocol_first_engineering_spec.md` | Spec/scorecard | 90/100 | Best V4 spec: protocol-first framing is right. Remaining gap: task leases, context budgets, and automated tool building need explicit schemas. | Make every critical behavior normative, testable, signed, versioned, and failure-aware. |
| `jmcp_v4_protocol_first_ieee_paper.tex` | Paper | 90/100 | Strongest paper input; protocol-first argument is persuasive, but still short of a complete standard and full conformance suite. | Expand into a 20-30 page IEEE-style standard paper with diagrams, stronger related work, falsifiable evaluation, and 50-60 references. |
| `jmcp_v4_scorecard.md` | Spec/scorecard | 82/100 | Useful critique, but the scorecard is not adversarial enough about false evidence, governance capture, and recursive self-improvement failures. | Make every critical behavior normative, testable, signed, versioned, and failure-aware. |
| `jmcp_v4_whitepaper.tex` | Paper | 87/100 | Ambitious and readable, but it blends system vision and implementation details without enough normative separation. | Expand into a 20-30 page IEEE-style standard paper with diagrams, stronger related work, falsifiable evaluation, and 50-60 references. |

## Cross-Artifact Critique

The V4 artifacts converge on the right thesis but still leave five risks under-controlled: protocol drift, false evidence, recursive self-improvement, user-attention collapse, and automated tool-building supply-chain risk. The final version treats these as first-class protocol surfaces rather than appendix concerns.

## Failure-Mode Register

| # | Failure mode | What can go wrong | Final hardening rule |
|---:|---|---|---|
| 1 | Protocol drift | Services quietly add non-standard fields or bypass leases | Schema validation, signed conformance receipts, reject unknown required semantics |
| 2 | Authority confusion | Agents treat advice as approval or confuse advisory and execution channels | Effect classes, explicit approval state, enforcement at runner and bus |
| 3 | False evidence | Logs, test results, or screenshots are fabricated or stale | Content-addressed evidence, verifier identity, replayable commands, quorum checks |
| 4 | Context poisoning | Untrusted repo docs or web pages inject instructions into task memory | Source taint labels, role separation, context firewall, quoted untrusted material |
| 5 | Lease overbreadth | A worker receives more permission than task needs | Least-privilege lease compiler and expiring scoped tokens |
| 6 | Recursive self-modification | JMCP improves itself into a broken or unsafe state | Shadow mode, canary rollout, quarantine, rollback, human approval for authority changes |
| 7 | User-attention flooding | Everything becomes an escalation and the user stops trusting it | Attention budgets, severity thresholds, digesting, decision bundling |
| 8 | User under-involvement | The system hides a critical decision or irreversible effect | Mandatory human gates for high-effect classes and unknown risk |
| 9 | Voice ambiguity | Speech recognition turns a casual statement into an action | Confirm high-effect actions, voice intent confidence, readback, session binding |
| 10 | Backbone overload | Telemetry volume swamps the control plane | Tiered streams, sampling, priority lanes, backpressure, local buffering |
| 11 | Split brain | Multiple JMCP instances issue conflicting decisions | Consensus lease holder, epoch fencing, monotonic sequence numbers |
| 12 | Data exfiltration | Agents leak secrets through prompts, logs, traces, or external tools | Secret scanners, egress policy, redaction, data-classified leases |
| 13 | Supply-chain compromise | Tool dependency or generated tool is malicious | SBOM, SLSA provenance, signatures, sandboxed build, reproducible checks |
| 14 | Policy capture | A bad lesson becomes global law and blocks good work | Jankurai lesson quarantine, challenge/appeal process, expiry and confidence |
| 15 | Observability theater | Dashboards look complete but omit causal proof | Evidence graph, trace-task correlation, missing-evidence alerts |
| 16 | Planner hallucination | JMCP scopes impossible work with confident estimates | Capability registry, cost model, dry-run, feasibility probes |
| 17 | Adversarial repo | Malicious code exploits analysis or build tools | Hermetic sandbox, read-only mounts, network denial, resource caps |
| 18 | Tool sprawl | The system builds many overlapping tools | Tool inventory, reuse scoring, deprecation, capability map |
| 19 | Semantic mismatch | MCP/A2A/OpenAPI tools report success using incompatible meanings | Adapters translate into JPCM effects and evidence taxonomy |
| 20 | Stale knowledge graph | Jeryu graph lags behind actual code | Freshness stamps, invalidation on commit, stale-source penalties |
| 21 | Runaway optimization | JMCP optimizes metrics while harming user goals | Goal charter, user-value KPIs, periodic alignment review |
| 22 | Silent degraded mode | Components fail but UI still presents confident status | Health leases, degraded-state badges, fail-closed for high effects |
| 23 | Temporal inconsistency | Tasks rely on outdated policy, schema, or repo state | Version pinning, schema epochs, rebasing checkpoints |
| 24 | Cross-tenant bleed | Lessons/data from one tenant contaminate another | Tenant isolation, anonymized promotion pipeline, access proofs |
| 25 | Unsafe automation | Automation outruns review and deploys broken code | Promotion ladder, staged effects, CI veto, Jankurai proof gate |
| 26 | Ambiguous ownership | No service owns a decision, artifact, or incident | Every event has responsible authority and owner service |
| 27 | Inadequate incident reconstruction | After failure, causality cannot be rebuilt | Append-only event log, causal IDs, evidence bundles, retention policy |
| 28 | Prompt/tool identity spoofing | A malicious service pretends to be a trusted one | mTLS, JWS signatures, service-card registry, key rotation |
| 29 | Schema ossification | Protocol cannot evolve without breaking services | Semver, capability negotiation, extension namespaces, deprecation windows |
| 30 | Over-centralization | JMCP becomes a bottleneck or single point of failure | Control/read-plane separation, local autonomous execution, replicated read models |

## Final Scoring Standard

A future artifact should not score above 95 unless it includes: executable schemas, golden conformance fixtures, red-team prompts and malicious services, disaster-mode drills, voice confirmation tests, tool-building quarantine tests, evidence replay tests, and measurable user-attention reduction targets. The final V5 package includes these as normative requirements, but implementation still must prove them empirically.
