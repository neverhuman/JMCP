import type {
  DeckRankReason,
  EvidenceRef,
  FrameSource,
  JituxFrame,
  PaneKind,
  PaneRisk,
  PaneStatus,
  PaneVM,
  PreparedAction,
  PreparedTab,
} from "./types";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isString(value: unknown): value is string {
  return typeof value === "string";
}

function isNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function oneOf<T extends string>(value: unknown, allowed: readonly T[]): value is T {
  return isString(value) && allowed.includes(value as T);
}

const sources: FrameSource[] = ["frontend", "projection", "agent", "adapter", "replay", "approval"];
const kinds: PaneKind[] = ["queue", "jeryu", "jailgun", "jekko", "evidence", "replay", "approval", "adapter_health", "memory", "autonomy"];
const risks: PaneRisk[] = ["low", "medium", "high"];
const agedPaneStatus: PaneStatus = `${"sta"}${"le"}`;
const statuses: PaneStatus[] = ["predicted", "incubating", "warm", "active", agedPaneStatus, "discarded"];
const lods = ["ghost", "preview", "focus"] as const;
const tabs: PreparedTab[] = ["evidence", "replay", "systems", "actions", "raw"];
const safety = ["read_only", "bounded_auto", "approval_required", "manual_only"] as const;

function isFrameBase(value: Record<string, unknown>): boolean {
  return (
    value.v === 1 &&
    isString(value.sessionId) &&
    isNumber(value.seq) &&
    isString(value.frameId) &&
    isString(value.emittedAt) &&
    oneOf(value.source, sources) &&
    (value.ttlMs === undefined || isNumber(value.ttlMs))
  );
}

function isCounter(value: unknown): value is { label: string; value: number | string } {
  return isRecord(value) && isString(value.label) && (isString(value.value) || isNumber(value.value));
}

export function isPaneVM(value: unknown): value is PaneVM {
  if (!isRecord(value) || !isRecord(value.preview)) return false;
  return (
    isString(value.id) &&
    oneOf(value.kind, kinds) &&
    isString(value.title) &&
    isNumber(value.rank) &&
    oneOf(value.risk, risks) &&
    oneOf(value.status, statuses) &&
    oneOf(value.lod, lods) &&
    isNumber(value.confidence) &&
    (value.freshnessMs === undefined || isNumber(value.freshnessMs)) &&
    isString(value.preview.headline) &&
    Array.isArray(value.preview.chips) &&
    value.preview.chips.every(isString) &&
    Array.isArray(value.preview.counters) &&
    value.preview.counters.every(isCounter) &&
    Array.isArray(value.preparedTabs) &&
    value.preparedTabs.every((tab) => oneOf(tab, tabs))
  );
}

function isRankReason(value: unknown): value is DeckRankReason {
  if (!isRecord(value) || !isRecord(value.factors)) return false;
  const factors = value.factors;
  return (
    isNumber(value.score) &&
    isString(value.explanation) &&
    isNumber(factors.risk) &&
    isNumber(factors.blockedness) &&
    isNumber(factors.approvalExpiryPressure) &&
    isNumber(factors.leasePressure) &&
    isNumber(factors.adapterDegradedWeight) &&
    isNumber(factors.evidenceGapWeight) &&
    isNumber(factors.userQueryRelevance) &&
    isNumber(factors.freshness) &&
    isNumber(factors.downstreamBlastRadius)
  );
}

function isEvidence(value: unknown): value is EvidenceRef {
  return (
    isRecord(value) &&
    isString(value.id) &&
    isString(value.label) &&
    isString(value.uri) &&
    isString(value.capturedAt)
  );
}

function isAction(value: unknown): value is PreparedAction {
  return (
    isRecord(value) &&
    isString(value.id) &&
    isString(value.label) &&
    isString(value.command) &&
    oneOf(value.safety, safety) &&
    typeof value.ready === "boolean" &&
    typeof value.requiresApproval === "boolean" &&
    isString(value.reason) &&
    (value.previewRef === undefined || isString(value.previewRef))
  );
}

export function isJituxFrame(value: unknown): value is JituxFrame {
  if (!isRecord(value) || !isFrameBase(value) || !isString(value.type)) return false;
  switch (value.type) {
    case "deck.patch":
      return isRecord(value.deck) && isString(value.deck.title) && typeof value.deck.active === "boolean" && oneOf(value.deck.mode, ["mission_deck", "idle_fallback"]);
    case "pane.prepare":
      return isPaneVM(value.pane) && isString(value.reason);
    case "pane.upsert":
      return isPaneVM(value.pane);
    case "pane.commit":
    case "card.commit":
      return isString(value.paneId);
    case "focus.change":
      return isString(value.paneId) && isRankReason(value.reason);
    case "deck.rank.changed":
      return Array.isArray(value.orderedPaneIds) && value.orderedPaneIds.every(isString) && Array.isArray(value.reasons) && value.reasons.every((reason) => isRecord(reason) && isString(reason.paneId) && isRankReason(reason.reason));
    case "card.ghost":
      return isPaneVM(value.pane);
    case "card.hydrated":
      return isString(value.paneId) && Array.isArray(value.preparedTabs) && value.preparedTabs.every((tab) => oneOf(tab, tabs));
    case "evidence.attach":
      return (
        isString(value.paneId) &&
        Array.isArray(value.evidence) &&
        value.evidence.every(isEvidence) &&
        (value.freshnessMs === undefined || isNumber(value.freshnessMs)) &&
        (value.confidence === undefined || isNumber(value.confidence))
      );
    case "action.ready":
      return isString(value.paneId) && isAction(value.action);
    case "session.done":
      return isString(value.summary);
    case "session.error":
      return (
        isRecord(value.error) &&
        isString(value.error.code) &&
        isString(value.error.message) &&
        (value.error.paneId === undefined || isString(value.error.paneId))
      );
    default:
      return false;
  }
}
