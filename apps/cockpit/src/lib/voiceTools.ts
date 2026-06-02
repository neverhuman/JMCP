// Browser-side registry of JMCP "tools" the local voice agent can call. The local
// 30B (vLLM, :18902) is served with tool-calling enabled; these are the functions
// it may invoke. READ-ONLY actions (status / listings) run freely; STATE-CHANGING
// actions (submit / start) require an explicit confirmed=true that the model only
// sets after the operator agrees out loud. Every call goes through the same-origin
// /jmcp proxy (-> 127.0.0.1:18877) so nothing leaves the machine. Responses are
// narrowed from `unknown` with explicit guards — never `as`-cast — and summarized
// to a short spoken-friendly string the model can read back.

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function readString(value: unknown): string {
  return typeof value === "string" ? value : "";
}

function readArray(value: unknown): unknown[] {
  return Array.isArray(value) ? value : [];
}

function jmcpBase(): string {
  const value = import.meta.env.VITE_JMCP_BASE;
  return typeof value === "string" && value.length > 0 ? value : "/jmcp";
}

function isAbortError(error: unknown): boolean {
  return (
    typeof DOMException !== "undefined" &&
    error instanceof DOMException &&
    error.name === "AbortError"
  );
}

// --- OpenAI tool-spec construction (kept explicit; no `as`) -------------------

interface ToolParam {
  type: string;
  description: string;
}

interface ToolSpec {
  type: "function";
  function: {
    name: string;
    description: string;
    parameters: {
      type: "object";
      properties: Record<string, ToolParam>;
      required: string[];
    };
  };
}

function spec(
  name: string,
  description: string,
  properties: Record<string, ToolParam>,
  required: string[],
): ToolSpec {
  return {
    type: "function",
    function: { name, description, parameters: { type: "object", properties, required } },
  };
}

const strParam = (description: string): ToolParam => ({ type: "string", description });
const boolParam = (description: string): ToolParam => ({ type: "boolean", description });

export const VOICE_TOOL_SPECS: ToolSpec[] = [
  spec("jmcp_status", "Get JMCP system health and the status of connected systems.", {}, []),
  spec("list_work_orders", "List current work orders, summarized by status.", {}, []),
  spec("microtask_queue", "List the live microtask queue (queued or running microtask work orders).", {}, []),
  spec("list_autonomous_actions", "List the autonomous actions that are available to start.", {}, []),
  spec("attention_inbox", "List items in the attention inbox that may need the operator.", {}, []),
  spec(
    "submit_microtask",
    "Queue a microtask by id. This CHANGES state; only call with confirmed=true after the operator confirms out loud.",
    {
      id: strParam("The microtask id, for example 'research.concept-scan'."),
      confirmed: boolParam("True only after the operator has confirmed out loud."),
    },
    ["id", "confirmed"],
  ),
  spec(
    "start_autonomous_action",
    "Start an autonomous action by id. This CHANGES state; only call with confirmed=true after the operator confirms out loud.",
    {
      id: strParam("The autonomous action id, for example 'repo-bank-bug-scan'."),
      confirmed: boolParam("True only after the operator has confirmed out loud."),
    },
    ["id", "confirmed"],
  ),
];

// --- HTTP helpers ------------------------------------------------------------

async function getJson(path: string, signal?: AbortSignal): Promise<unknown> {
  const response = await fetch(`${jmcpBase()}${path}`, { signal });
  if (!response.ok) {
    throw new Error(`JMCP ${path} returned ${response.status}`);
  }
  return response.json();
}

async function postJmcp(path: string, signal?: AbortSignal): Promise<number> {
  const response = await fetch(`${jmcpBase()}${path}`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: "{}",
    signal,
  });
  return response.status;
}

// --- Response summarizers (short, spoken-friendly) ---------------------------

function summarizeStatus(body: unknown): string {
  if (!isRecord(body)) {
    return "JMCP status is unavailable.";
  }
  const systems = readArray(body.systems);
  const names: string[] = [];
  for (const sys of systems) {
    if (isRecord(sys)) {
      const name = readString(sys.name);
      if (name.length > 0) names.push(name);
    }
  }
  if (body.ok !== true) {
    return "JMCP reports a problem with its health check.";
  }
  const suffix = names.length > 0 ? `: ${names.join(", ")}` : "";
  const plural = systems.length === 1 ? "" : "s";
  return `JMCP is healthy, with ${systems.length} system${plural} connected${suffix}.`;
}

function countByStatus(list: unknown[]): string {
  const counts = new Map<string, number>();
  for (const item of list) {
    if (isRecord(item)) {
      const status = readString(item.status);
      const key = status.length > 0 ? status : "unknown";
      const prior = counts.get(key);
      counts.set(key, (typeof prior === "number" ? prior : 0) + 1);
    }
  }
  const parts: string[] = [];
  for (const [key, n] of counts) parts.push(`${n} ${key}`);
  return parts.join(", ");
}

function summarizeWorkOrders(body: unknown): string {
  const list = readArray(body);
  if (list.length === 0) {
    return "There are no work orders.";
  }
  const plural = list.length === 1 ? "" : "s";
  return `${list.length} work order${plural}: ${countByStatus(list)}.`;
}

function summarizeQueue(body: unknown): string {
  const list = readArray(body);
  if (list.length === 0) {
    return "The microtask queue is empty.";
  }
  const plural = list.length === 1 ? "" : "s";
  return `${list.length} microtask${plural} in the queue: ${countByStatus(list)}.`;
}

function topTitles(list: unknown[], limit: number): string[] {
  const titles: string[] = [];
  for (const item of list) {
    if (titles.length >= limit) break;
    if (isRecord(item)) {
      const title = readString(item.title);
      const id = readString(item.id);
      const label = title.length > 0 ? title : id;
      if (label.length > 0) titles.push(label);
    }
  }
  return titles;
}

function summarizeActions(body: unknown): string {
  const list = readArray(body);
  if (list.length === 0) {
    return "There are no autonomous actions available.";
  }
  const names = topTitles(list, 4);
  const plural = list.length === 1 ? "" : "s";
  return `${list.length} autonomous action${plural} available: ${names.join("; ")}.`;
}

function summarizeAttention(body: unknown): string {
  const list = readArray(body);
  if (list.length === 0) {
    return "The attention inbox is clear.";
  }
  const names = topTitles(list, 3);
  const plural = list.length === 1 ? "" : "s";
  return `${list.length} attention item${plural}: ${names.join("; ")}.`;
}

// --- State-changing actions (gated on spoken confirmation) -------------------

async function submitMicrotask(args: Record<string, unknown>, signal?: AbortSignal): Promise<string> {
  const id = readString(args.id);
  if (id.length === 0) {
    return "I need the microtask id before I can queue it.";
  }
  if (args.confirmed !== true) {
    return `Queuing the ${id} microtask changes state. Tell the operator what it does and ask them to confirm out loud first.`;
  }
  const status = await postJmcp(`/microtasks/${encodeURIComponent(id)}/submit`, signal);
  if (status < 200 || status >= 300) {
    return `I could not queue ${id}; JMCP returned ${status}.`;
  }
  return `Queued the ${id} microtask.`;
}

async function startAutonomousAction(args: Record<string, unknown>, signal?: AbortSignal): Promise<string> {
  const id = readString(args.id);
  if (id.length === 0) {
    return "I need the action id before I can start it.";
  }
  if (args.confirmed !== true) {
    return `Starting the ${id} action changes state. Tell the operator what it does and ask them to confirm out loud first.`;
  }
  const status = await postJmcp(`/autonomous-actions/${encodeURIComponent(id)}/submit`, signal);
  if (status < 200 || status >= 300) {
    return `I could not start ${id}; JMCP returned ${status}.`;
  }
  return `Started the ${id} autonomous action.`;
}

// --- Dispatch ----------------------------------------------------------------

/**
 * Execute a tool the model asked for and return a short string for it to speak.
 * Errors (except a barge-in abort, which propagates to cancel the turn) are
 * returned as spoken-friendly text rather than thrown, so one failed tool does
 * not break the conversation.
 */
export async function executeVoiceTool(
  name: string,
  argumentsJson: string,
  signal?: AbortSignal,
): Promise<string> {
  let parsed: unknown = {};
  try {
    parsed = JSON.parse(argumentsJson.length > 0 ? argumentsJson : "{}");
  } catch {
    parsed = {};
  }
  const args = isRecord(parsed) ? parsed : {};
  try {
    if (name === "jmcp_status") return summarizeStatus(await getJson("/health", signal));
    if (name === "list_work_orders") return summarizeWorkOrders(await getJson("/work-orders", signal));
    if (name === "microtask_queue") return summarizeQueue(await getJson("/microtasks/queue", signal));
    if (name === "list_autonomous_actions") return summarizeActions(await getJson("/autonomous-actions", signal));
    if (name === "attention_inbox") return summarizeAttention(await getJson("/attention-packets", signal));
    if (name === "submit_microtask") return await submitMicrotask(args, signal);
    if (name === "start_autonomous_action") return await startAutonomousAction(args, signal);
    return `I do not have a tool called ${name}.`;
  } catch (error) {
    if (isAbortError(error)) {
      throw error;
    }
    return `That action could not be completed: ${error instanceof Error ? error.message : "unknown error"}.`;
  }
}
