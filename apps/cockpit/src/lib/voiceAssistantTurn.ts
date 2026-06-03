import {
  VOICE_MODEL,
  reasonStream,
  type ChatMessage,
  type ToolCallFunction,
} from "./speechClient";
import { FIRST_CHUNK_CHARS, MAX_TOOL_HOPS } from "./voiceAssistantConfig";
import {
  JITUX_FIRST_FRAME_TIMEOUT_MS,
  openVoiceJituxSession,
  waitForUsefulVoiceDeckFrame,
  type VoiceJituxDeckReadiness,
  type VoiceJituxSessionStart,
} from "./voiceJituxSession";
import { VOICE_TOOL_SPECS, executeVoiceTool } from "./voiceTools";

interface VoiceTurnOptions {
  command: string;
  history: ChatMessage[];
  signal: AbortSignal;
  enqueueSpeech: (text: string, signal?: AbortSignal) => void;
  setThinking: () => void;
  openDeckSession?: (prompt: string, signal: AbortSignal) => Promise<VoiceJituxSessionStart>;
  waitForDeckFrame?: (
    start: Promise<VoiceJituxSessionStart>,
    signal: AbortSignal,
    timeoutMs: number,
  ) => Promise<VoiceJituxDeckReadiness>;
  deckFrameTimeoutMs?: number;
}

type FastReadOnlyTool =
  | "jmcp_status"
  | "list_work_orders"
  | "microtask_queue"
  | "list_autonomous_actions"
  | "attention_inbox";

export type FastReadOnlyDecision =
  | { kind: "local_tool"; tool: FastReadOnlyTool }
  | { kind: "model"; reason: "empty_command" | "mutation_intent" | "requires_reasoning" };

export function detectFastReadOnlyTool(command: string): FastReadOnlyDecision {
  const text = command.trim().toLowerCase();
  const mutationIntent =
    /^(please\s+)?(start|submit|queue|run|launch|approve|deny|cancel)\b/.test(text) ||
    /\b(can|could|would)\s+you\s+(start|submit|queue|run|launch|approve|deny|cancel)\b/.test(text);
  if (text.length === 0) {
    return { kind: "model", reason: "empty_command" };
  }
  if (mutationIntent) {
    return { kind: "model", reason: "mutation_intent" };
  }
  if (/\b(status|health|healthy)\b/.test(text) || /\bhow\s+(is|are)\b.*\bjmcp\b/.test(text)) {
    return { kind: "local_tool", tool: "jmcp_status" };
  }
  if (/\battention\b/.test(text) || /\bneeds?\s+me\b/.test(text)) {
    return { kind: "local_tool", tool: "attention_inbox" };
  }
  if (/\bautonomous\b/.test(text) || /\bwhat\s+can\s+you\s+(safely\s+)?do\b/.test(text)) {
    return { kind: "local_tool", tool: "list_autonomous_actions" };
  }
  if (/\bwork\s+orders?\b/.test(text)) {
    return { kind: "local_tool", tool: "list_work_orders" };
  }
  if (
    /\bqueue\b/.test(text) ||
    /\b(blocked|blocking|blockers?|microtasks?)\b/.test(text)
  ) {
    return { kind: "local_tool", tool: "microtask_queue" };
  }
  return { kind: "model", reason: "requires_reasoning" };
}

export async function runVoiceTurn({
  command,
  history,
  signal,
  enqueueSpeech,
  setThinking,
  openDeckSession = openVoiceJituxSession,
  waitForDeckFrame = waitForUsefulVoiceDeckFrame,
  deckFrameTimeoutMs = JITUX_FIRST_FRAME_TIMEOUT_MS,
}: VoiceTurnOptions): Promise<string> {
  history.push({ role: "user", content: command });
  const deckSession = openDeckSession(command, signal);
  const deckReadiness = waitForDeckFrame(deckSession, signal, deckFrameTimeoutMs).catch(
    (error: unknown): VoiceJituxDeckReadiness => ({
      kind: "unavailable",
      reason: error instanceof Error ? error.message : "jitux_wait_error",
    }),
  );
  const enqueueDeckAwareSpeech = createDeckAwareSpeechQueue(
    enqueueSpeech,
    deckReadiness,
    signal,
  );
  const fastDecision = detectFastReadOnlyTool(command);
  if (fastDecision.kind === "local_tool") {
    const output = await executeVoiceTool(fastDecision.tool, "{}", signal);
    enqueueDeckAwareSpeech(output);
    history.push({ role: "assistant", content: output });
    trimHistory(history);
    return output;
  }

  let pending = "";
  let firstChunk = true;
  const flushChunks = (force: boolean) => {
    const chunks = pending.match(/[^,.!?:;]*[,.!?:;]+\s*/g);
    if (chunks !== null) {
      let consumed = 0;
      for (const chunk of chunks) {
        enqueueDeckAwareSpeech(chunk);
        consumed += chunk.length;
      }
      pending = pending.slice(consumed);
      firstChunk = false;
    }
    if (force) {
      const lastSpace = pending.lastIndexOf(" ");
      if (lastSpace > 12) {
        enqueueDeckAwareSpeech(pending.slice(0, lastSpace));
        pending = pending.slice(lastSpace + 1);
        firstChunk = false;
      }
    }
  };
  const onDelta = (delta: string) => {
    pending += delta;
    if (/[,.!?:;]/.test(pending)) {
      flushChunks(false);
    } else if (pending.length > (firstChunk ? FIRST_CHUNK_CHARS : 120)) {
      flushChunks(true);
    }
  };

  let lastText = "";
  for (let hop = 0; hop < MAX_TOOL_HOPS; hop++) {
    pending = "";
    firstChunk = true;
    const result = await reasonStream(history, onDelta, signal, VOICE_MODEL, VOICE_TOOL_SPECS);
    enqueueDeckAwareSpeech(pending);
    pending = "";
    lastText = result.text;
    if (result.toolCalls.length === 0) {
      history.push({ role: "assistant", content: result.text });
      break;
    }
    const toolCalls: ToolCallFunction[] = result.toolCalls.map((call) => ({
      id: call.id,
      type: "function",
      function: { name: call.name, arguments: call.arguments },
    }));
    history.push({ role: "assistant", content: result.text, tool_calls: toolCalls });
    setThinking();
    for (const call of result.toolCalls) {
      const output = await executeVoiceTool(call.name, call.arguments, signal);
      history.push({ role: "tool", tool_call_id: call.id, content: output });
    }
  }
  trimHistory(history);
  return lastText;
}

function trimHistory(history: ChatMessage[]): void {
  if (history.length <= 13) return;
  const trimmed = [history[0], ...history.slice(history.length - 12)];
  history.splice(0, history.length, ...trimmed);
}

function createDeckAwareSpeechQueue(
  enqueueSpeech: (text: string, signal?: AbortSignal) => void,
  deckReadiness: Promise<VoiceJituxDeckReadiness>,
  signal: AbortSignal,
): (text: string) => void {
  let ready = false;
  const pending: string[] = [];
  const release = () => {
    if (ready) {
      return;
    }
    ready = true;
    for (const text of pending) {
      enqueueSpeech(text, signal);
    }
    pending.splice(0, pending.length);
  };
  void deckReadiness.then(release, release);
  return (text: string) => {
    if (text.trim().length === 0) {
      return;
    }
    if (ready) {
      enqueueSpeech(text, signal);
      return;
    }
    pending.push(text);
  };
}
