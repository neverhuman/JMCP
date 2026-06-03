import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { stripWakeWord } from "./hooks/useVoiceAssistant";
import {
  VOICE_MODEL,
  reason,
  reasonStream,
  synthesize,
  transcribe,
} from "./lib/speechClient";
import {
  describeMicrophoneError,
  micSupported,
  type MicrophoneInspection,
} from "./lib/microphone";
import { VOICE_TOOL_SPECS, executeVoiceTool } from "./lib/voiceTools";
import { detectFastReadOnlyTool, runVoiceTurn } from "./lib/voiceAssistantTurn";
import {
  openVoiceJituxSession,
  waitForUsefulVoiceDeckFrame,
  type VoiceJituxDeckReadiness,
  type VoiceJituxSessionStart,
} from "./lib/voiceJituxSession";

// A minimal stand-in for the Fetch API Response surface that speechClient reads:
// `.ok`, `.json()`, and `.blob()`. Each test builds one of these via the helpers
// below so we never touch the real network.
interface ResponseLike {
  ok: boolean;
  status: number;
  body: ReadableStream<Uint8Array> | null;
  json: () => Promise<unknown>;
  blob: () => Promise<Blob>;
}

function jsonResponse(value: unknown, ok = true, status = 200): ResponseLike {
  return {
    ok,
    status,
    body: null,
    json: () => Promise.resolve(value),
    blob: () => Promise.resolve(new Blob()),
  };
}

function blobResponse(payload: Blob, ok = true, status = 200): ResponseLike {
  return {
    ok,
    status,
    body: null,
    json: () => Promise.resolve(null),
    blob: () => Promise.resolve(payload),
  };
}

function streamResponse(chunks: string[], ok = true, status = 200): ResponseLike {
  const encoder = new TextEncoder();
  return {
    ok,
    status,
    body: new ReadableStream<Uint8Array>({
      start(controller) {
        for (const chunk of chunks) {
          controller.enqueue(encoder.encode(chunk));
        }
        controller.close();
      },
    }),
    json: () => Promise.resolve(null),
    blob: () => Promise.resolve(new Blob()),
  };
}

// The Fetch signature speechClient depends on, expressed through vi.fn so the
// double is typed by its call signature rather than an `as` cast.
type FetchSignature = (input: string, init?: RequestInit) => Promise<ResponseLike>;

function installFetch(impl: FetchSignature): ReturnType<typeof vi.fn<FetchSignature>> {
  const fetchDouble = vi.fn<FetchSignature>(impl);
  vi.stubGlobal("fetch", fetchDouble);
  return fetchDouble;
}

function immediateDeck() {
  return {
    openDeckSession: vi.fn(
      (): Promise<VoiceJituxSessionStart> =>
        Promise.resolve({ kind: "unavailable", reason: "test" }),
    ),
    waitForDeckFrame: vi.fn(
      (): Promise<VoiceJituxDeckReadiness> =>
        Promise.resolve({ kind: "unavailable", reason: "test" }),
    ),
  };
}

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("stripWakeWord", () => {
  it("returns the command that follows a plain wake word", () => {
    const result = stripWakeWord("hey JMCP what time is it");
    expect(result.triggered).toBe(true);
    expect(result.command).toBe("what time is it");
  });

  it("triggers with an empty command when only the wake word is spoken", () => {
    const result = stripWakeWord("jmcp");
    expect(result.triggered).toBe(true);
    expect(result.command).toBe("");
  });

  it("does not trigger when no wake word is present", () => {
    const result = stripWakeWord("hello there");
    expect(result.triggered).toBe(false);
    expect(result.command).toBe("");
  });

  it("matches wake words regardless of letter casing", () => {
    expect(stripWakeWord("HEY JIM CP open the audit").triggered).toBe(true);
    expect(stripWakeWord("CoMpUtEr").triggered).toBe(true);
    expect(stripWakeWord("Hey Jmcp status").command).toBe("status");
  });

  it("strips leading punctuation between the wake word and the command", () => {
    const result = stripWakeWord("computer, run the audit");
    expect(result.triggered).toBe(true);
    expect(result.command).toBe("run the audit");
  });

  it("does not trigger on a wake word embedded inside a larger word", () => {
    const result = stripWakeWord("please computerize the report");
    expect(result.triggered).toBe(false);
    expect(result.command).toBe("");
  });
});

describe("transcribe", () => {
  it("parses text and confidence from a JSON body", async () => {
    const fetchDouble = installFetch(() =>
      Promise.resolve(jsonResponse({ text: "  open the bay  ", confidence: 0.92 })),
    );
    const result = await transcribe(new Blob(["audio"], { type: "audio/webm" }));
    expect(result.text).toBe("open the bay");
    expect(result.confidence).toBe(0.92);
    expect(fetchDouble).toHaveBeenCalledTimes(1);
  });

  it("reports a null confidence when the body omits it", async () => {
    installFetch(() => Promise.resolve(jsonResponse({ text: "ready" })));
    const result = await transcribe(new Blob());
    expect(result.text).toBe("ready");
    expect(result.confidence).toBeNull();
  });

  it("returns an empty text when the body is not an object", async () => {
    installFetch(() => Promise.resolve(jsonResponse("not-an-object")));
    const result = await transcribe(new Blob());
    expect(result.text).toBe("");
    expect(result.confidence).toBeNull();
  });

  it("forwards the requested language to the sidecar", async () => {
    const fetchDouble = installFetch(() => Promise.resolve(jsonResponse({ text: "hola" })));
    await transcribe(new Blob(), "es");
    const firstCall = fetchDouble.mock.calls[0];
    expect(firstCall[0]).toContain("language=es");
  });

  it("uses beam size 1 by default for realtime ASR", async () => {
    const fetchDouble = installFetch(() => Promise.resolve(jsonResponse({ text: "ready" })));
    await transcribe(new Blob());
    const firstCall = fetchDouble.mock.calls[0];
    expect(firstCall[0]).toContain("beam_size=1");
  });

  it("lets accuracy runs override the ASR beam size", async () => {
    const fetchDouble = installFetch(() => Promise.resolve(jsonResponse({ text: "ready" })));
    await transcribe(new Blob(), "en", 4);
    const firstCall = fetchDouble.mock.calls[0];
    expect(firstCall[0]).toContain("beam_size=4");
  });

  it("throws when the sidecar response is not ok", async () => {
    installFetch(() => Promise.resolve(jsonResponse({}, false, 503)));
    await expect(transcribe(new Blob())).rejects.toThrow("ASR 503");
  });
});

describe("voice JITUX sessions", () => {
  it("opens a Mission Deck session through the local JMCP proxy", async () => {
    const fetchDouble = installFetch(() =>
      Promise.resolve(
        jsonResponse({
          sessionId: "jitux_1",
          streamUrl: "/jitux/sessions/jitux_1/stream",
          wsUrl: "/jitux/sessions/jitux_1/ws",
        }),
      ),
    );

    const result = await openVoiceJituxSession("what is blocking the queue?");

    expect(result).toEqual({
      kind: "ready",
      session: {
        sessionId: "jitux_1",
        streamUrl: "/jitux/sessions/jitux_1/stream",
        wsUrl: "/jitux/sessions/jitux_1/ws",
      },
    });
    expect(fetchDouble.mock.calls[0][0]).toBe("/jmcp/jitux/sessions");
    expect(fetchDouble.mock.calls[0][1]?.method).toBe("POST");
    expect(fetchDouble.mock.calls[0][1]?.body).toBe(
      JSON.stringify({ prompt: "what is blocking the queue?", source: "voice" }),
    );
  });

  it("resolves readiness when the stream emits a useful deck frame", async () => {
    const frame = {
      v: 1,
      sessionId: "jitux_1",
      seq: 1,
      frameId: "frame_1",
      emittedAt: "2026-06-03T15:00:00.000Z",
      source: "projection",
      type: "deck.patch",
      deck: { title: "Scanning queue blockers", active: true, mode: "mission_deck" },
    };
    installFetch(() =>
      Promise.resolve(
        streamResponse([`event: jitux.frame\ndata: ${JSON.stringify(frame)}\n\n`]),
      ),
    );

    const result = await waitForUsefulVoiceDeckFrame(
      Promise.resolve({
        kind: "ready",
        session: {
          sessionId: "jitux_1",
          streamUrl: "/jitux/sessions/jitux_1/stream",
          wsUrl: "/jitux/sessions/jitux_1/ws",
        },
      }),
      new AbortController().signal,
      50,
    );

    expect(result).toEqual({ kind: "frame", sessionId: "jitux_1", frameType: "deck.patch" });
  });
});

describe("reason", () => {
  it("extracts the first choice message content", async () => {
    const body = {
      choices: [{ message: { role: "assistant", content: "  the bay is open  " } }],
    };
    installFetch(() => Promise.resolve(jsonResponse(body)));
    const answer = await reason([{ role: "user", content: "status?" }]);
    expect(answer).toBe("the bay is open");
  });

  it("returns an empty string when the choices array is absent", async () => {
    installFetch(() => Promise.resolve(jsonResponse({ id: "x" })));
    const answer = await reason([{ role: "user", content: "status?" }]);
    expect(answer).toBe("");
  });

  it("sends the configured voice model in the request payload", async () => {
    const body = { choices: [{ message: { content: "ok" } }] };
    const fetchDouble = installFetch(() => Promise.resolve(jsonResponse(body)));
    await reason([{ role: "user", content: "hi" }]);
    const init = fetchDouble.mock.calls[0][1];
    const raw = init?.body;
    expect(typeof raw).toBe("string");
    if (typeof raw === "string") {
      const parsed: unknown = JSON.parse(raw);
      expect(parsed).toMatchObject({ model: VOICE_MODEL });
    }
  });

  it("forwards an abort signal to the reasoning endpoint", async () => {
    const controller = new AbortController();
    const body = { choices: [{ message: { content: "ok" } }] };
    const fetchDouble = installFetch(() => Promise.resolve(jsonResponse(body)));
    await reason([{ role: "user", content: "hi" }], controller.signal);
    expect(fetchDouble.mock.calls[0][1]?.signal).toBe(controller.signal);
  });

  it("throws when the reasoning endpoint response is not ok", async () => {
    installFetch(() => Promise.resolve(jsonResponse({}, false, 500)));
    await expect(reason([{ role: "user", content: "x" }])).rejects.toThrow("LLM 500");
  });
});

describe("reasonStream", () => {
  function dataLine(delta: string): string {
    return `data: ${JSON.stringify({ choices: [{ delta: { content: delta } }] })}\n\n`;
  }

  function toolLine(part: Record<string, unknown>): string {
    return `data: ${JSON.stringify({ choices: [{ delta: { tool_calls: [part] } }] })}\n\n`;
  }

  it("emits streaming deltas and returns the full assistant text", async () => {
    installFetch(() =>
      Promise.resolve(streamResponse([dataLine("Hel"), dataLine("lo"), "data: [DONE]\n\n"])),
    );
    const deltas: string[] = [];
    const result = await reasonStream([{ role: "user", content: "greet" }], (delta) => {
      deltas.push(delta);
    });
    expect(deltas).toEqual(["Hel", "lo"]);
    expect(result.text).toBe("Hello");
    expect(result.toolCalls).toEqual([]);
  });

  it("handles SSE data lines split across network chunks", async () => {
    const payload = dataLine("chunked");
    installFetch(() => Promise.resolve(streamResponse([payload.slice(0, 18), payload.slice(18)])));
    const deltas: string[] = [];
    const result = await reasonStream([{ role: "user", content: "greet" }], (delta) => {
      deltas.push(delta);
    });
    expect(deltas).toEqual(["chunked"]);
    expect(result.text).toBe("chunked");
  });

  it("reassembles a tool call from its streamed deltas", async () => {
    installFetch(() =>
      Promise.resolve(
        streamResponse([
          toolLine({ index: 0, id: "call_1", function: { name: "jmcp_status", arguments: "" } }),
          toolLine({ index: 0, function: { arguments: "{}" } }),
          "data: [DONE]\n\n",
        ]),
      ),
    );
    const result = await reasonStream([{ role: "user", content: "status?" }], () => undefined);
    expect(result.text).toBe("");
    expect(result.toolCalls).toEqual([{ id: "call_1", name: "jmcp_status", arguments: "{}" }]);
  });

  it("normalizes no-argument tool calls to JSON objects for the next LLM hop", async () => {
    installFetch(() =>
      Promise.resolve(
        streamResponse([
          toolLine({ index: 0, id: "call_1", function: { name: "microtask_queue" } }),
          "data: [DONE]\n\n",
        ]),
      ),
    );

    const result = await reasonStream([{ role: "user", content: "queue?" }], () => undefined);

    expect(result.toolCalls).toEqual([{ id: "call_1", name: "microtask_queue", arguments: "{}" }]);
  });

  it("includes the tools array in the request when provided", async () => {
    const fetchDouble = installFetch(() => Promise.resolve(streamResponse(["data: [DONE]\n\n"])));
    await reasonStream(
      [{ role: "user", content: "hi" }],
      () => undefined,
      undefined,
      VOICE_MODEL,
      VOICE_TOOL_SPECS,
    );
    const raw = fetchDouble.mock.calls[0][1]?.body;
    expect(typeof raw).toBe("string");
    if (typeof raw === "string") {
      const parsed: unknown = JSON.parse(raw);
      if (parsed !== null && typeof parsed === "object" && "tools" in parsed) {
        expect(Array.isArray(parsed.tools)).toBe(true);
      } else {
        throw new Error("expected a tools array in the payload");
      }
    }
  });

  it("forwards an abort signal to the streaming reasoning endpoint", async () => {
    const controller = new AbortController();
    const fetchDouble = installFetch(() => Promise.resolve(streamResponse(["data: [DONE]\n\n"])));
    await reasonStream([{ role: "user", content: "hi" }], () => undefined, controller.signal);
    expect(fetchDouble.mock.calls[0][1]?.signal).toBe(controller.signal);
  });
});

describe("voiceTools", () => {
  it("exposes the expected tool catalog, gating mutations behind confirmed", () => {
    const names = VOICE_TOOL_SPECS.map((spec) => spec.function.name);
    expect(names).toContain("jmcp_status");
    expect(names).toContain("submit_microtask");
    expect(names).toContain("start_autonomous_action");
    const submit = VOICE_TOOL_SPECS.find((spec) => spec.function.name === "submit_microtask");
    expect(submit?.function.parameters.required).toContain("confirmed");
  });

  it("summarizes a read-only status call", async () => {
    installFetch(() =>
      Promise.resolve(jsonResponse({ ok: true, systems: [{ name: "jmcpd" }, { name: "jeryu" }] })),
    );
    const spoken = await executeVoiceTool("jmcp_status", "{}");
    expect(spoken).toContain("healthy");
    expect(spoken).toContain("2 systems");
  });

  it("counts work orders by status", async () => {
    installFetch(() =>
      Promise.resolve(
        jsonResponse([{ status: "submitted" }, { status: "completed" }, { status: "completed" }]),
      ),
    );
    const spoken = await executeVoiceTool("list_work_orders", "{}");
    expect(spoken).toContain("3 work orders");
    expect(spoken).toContain("2 completed");
  });

  it("summarizes the current queue from the work-order list", async () => {
    const fetchDouble = installFetch(() =>
      Promise.resolve(jsonResponse([{ status: "Submitted" }, { status: "Running" }])),
    );

    const spoken = await executeVoiceTool("microtask_queue", "{}");

    expect(spoken).toContain("2 microtasks");
    expect(spoken).toContain("1 Submitted");
    expect(spoken).toContain("1 Running");
    expect(fetchDouble.mock.calls[0][0]).toContain("/work-orders");
  });

  it("refuses a state change without confirmation and does not POST", async () => {
    const fetchDouble = installFetch(() => Promise.resolve(jsonResponse({})));
    const spoken = await executeVoiceTool(
      "submit_microtask",
      JSON.stringify({ id: "research.concept-scan" }),
    );
    expect(spoken.toLowerCase()).toContain("confirm");
    expect(fetchDouble).toHaveBeenCalledTimes(0);
  });

  it("submits a microtask once confirmed", async () => {
    const fetchDouble = installFetch(() => Promise.resolve(jsonResponse({}, true, 200)));
    const spoken = await executeVoiceTool(
      "submit_microtask",
      JSON.stringify({ id: "research.concept-scan", confirmed: true }),
    );
    expect(spoken).toContain("Queued");
    const call = fetchDouble.mock.calls[0];
    expect(call[0]).toContain("/microtasks/research.concept-scan/submit");
    expect(call[1]?.method).toBe("POST");
  });
});

describe("runVoiceTurn fast path", () => {
  it("routes simple read-only intents without using the model", async () => {
    expect(detectFastReadOnlyTool("how is JMCP doing?")).toEqual({
      kind: "local_tool",
      tool: "jmcp_status",
    });
    expect(detectFastReadOnlyTool("what is blocking the queue right now?")).toEqual({
      kind: "local_tool",
      tool: "microtask_queue",
    });
    expect(detectFastReadOnlyTool("queue the microtask")).toEqual({
      kind: "model",
      reason: "mutation_intent",
    });
    expect(detectFastReadOnlyTool("can you start the bug scan?")).toEqual({
      kind: "model",
      reason: "mutation_intent",
    });
  });

  it("answers status through the local tool before model reasoning", async () => {
    const deck = immediateDeck();
    const fetchDouble = installFetch((input) => {
      if (input.includes("/jmcp/health")) {
        return Promise.resolve(
          jsonResponse({ ok: true, systems: [{ name: "jmcpd" }, { name: "jeryu" }] }),
        );
      }
      return Promise.reject(new Error(`unexpected fetch: ${input}`));
    });
    const spoken: string[] = [];
    const history = [{ role: "system" as const, content: "system" }];

    const answer = await runVoiceTurn({
      command: "how is JMCP doing?",
      history,
      signal: new AbortController().signal,
      enqueueSpeech: (text) => spoken.push(text),
      setThinking: vi.fn(),
      ...deck,
    });

    expect(answer).toContain("JMCP is healthy");
    expect(spoken).toEqual([answer]);
    expect(fetchDouble).toHaveBeenCalledTimes(1);
    expect(fetchDouble.mock.calls[0][0]).toContain("/jmcp/health");
    expect(history[history.length - 1]).toMatchObject({ role: "assistant", content: answer });
    expect(deck.openDeckSession.mock.invocationCallOrder[0]).toBeLessThan(
      deck.waitForDeckFrame.mock.invocationCallOrder[0],
    );
  });

  it("starts deck work before model reasoning", async () => {
    const events: string[] = [];
    const deck = {
      openDeckSession: vi.fn((): Promise<VoiceJituxSessionStart> => {
        events.push("jitux");
        return Promise.resolve({ kind: "unavailable", reason: "test" });
      }),
      waitForDeckFrame: vi.fn((): Promise<VoiceJituxDeckReadiness> => {
        events.push("deck_wait");
        return Promise.resolve({ kind: "unavailable", reason: "test" });
      }),
    };
    installFetch(() => {
      events.push("llm");
      return Promise.resolve(
        streamResponse([
          'data: {"choices":[{"delta":{"content":"Ready."}}]}\n\n',
          "data: [DONE]\n\n",
        ]),
      );
    });
    const spoken: string[] = [];

    const answer = await runVoiceTurn({
      command: "explain the current mission",
      history: [{ role: "system", content: "system" }],
      signal: new AbortController().signal,
      enqueueSpeech: (text) => {
        events.push("speech");
        spoken.push(text);
      },
      setThinking: vi.fn(),
      ...deck,
    });

    expect(answer).toBe("Ready.");
    expect(spoken).toEqual(["Ready."]);
    expect(events).toEqual(["jitux", "deck_wait", "llm", "speech"]);
  });

  it("holds first speech until a deck frame or timeout releases it", async () => {
    let releaseDeck = (_value: VoiceJituxDeckReadiness) => {};
    const deck = {
      openDeckSession: vi.fn(
        (): Promise<VoiceJituxSessionStart> =>
          Promise.resolve({ kind: "unavailable", reason: "test" }),
      ),
      waitForDeckFrame: vi.fn(
        (): Promise<VoiceJituxDeckReadiness> =>
          new Promise((resolve) => {
            releaseDeck = resolve;
          }),
      ),
    };
    installFetch((input) => {
      if (input.includes("/jmcp/health")) {
        return Promise.resolve(jsonResponse({ ok: true, systems: [{ name: "jmcpd" }] }));
      }
      return Promise.reject(new Error(`unexpected fetch: ${input}`));
    });
    const spoken: string[] = [];

    const answer = await runVoiceTurn({
      command: "status",
      history: [{ role: "system", content: "system" }],
      signal: new AbortController().signal,
      enqueueSpeech: (text) => spoken.push(text),
      setThinking: vi.fn(),
      ...deck,
    });

    expect(answer).toContain("JMCP is healthy");
    expect(spoken).toEqual([]);
    releaseDeck({ kind: "timeout" });
    await Promise.resolve();
    await Promise.resolve();
    expect(spoken).toEqual([answer]);
  });
});

describe("synthesize", () => {
  it("returns the audio blob from the synthesis endpoint", async () => {
    const audio = new Blob(["ogg-bytes"], { type: "audio/ogg" });
    installFetch(() => Promise.resolve(blobResponse(audio)));
    const result = await synthesize("hello operator");
    expect(result).toBe(audio);
    expect(result.type).toBe("audio/ogg");
  });

  it("forwards an abort signal to the synthesis endpoint", async () => {
    const controller = new AbortController();
    const fetchDouble = installFetch(() => Promise.resolve(blobResponse(new Blob())));
    await synthesize("hello operator", controller.signal);
    expect(fetchDouble.mock.calls[0][1]?.signal).toBe(controller.signal);
  });

  it("throws when the synthesis endpoint response is not ok", async () => {
    installFetch(() => Promise.resolve(blobResponse(new Blob(), false, 404)));
    await expect(synthesize("hello")).rejects.toThrow("TTS 404");
  });
});

describe("micSupported", () => {
  let savedMediaDevices: MediaDevices | undefined;
  let savedSecureContext: boolean | undefined;
  let savedMediaRecorder: typeof MediaRecorder | undefined;

  beforeEach(() => {
    savedMediaDevices = navigator.mediaDevices;
    savedSecureContext = window.isSecureContext;
    savedMediaRecorder = window.MediaRecorder;
  });

  afterEach(() => {
    Object.defineProperty(navigator, "mediaDevices", {
      configurable: true,
      value: savedMediaDevices,
    });
    Object.defineProperty(window, "isSecureContext", {
      configurable: true,
      value: savedSecureContext,
    });
    Object.defineProperty(window, "MediaRecorder", {
      configurable: true,
      value: savedMediaRecorder,
    });
  });

  it("is false in a jsdom environment without microphone capture", () => {
    Object.defineProperty(navigator, "mediaDevices", {
      configurable: true,
      value: undefined,
    });
    expect(micSupported()).toBe(false);
  });

  it("requires a secure browser context", () => {
    Object.defineProperty(window, "isSecureContext", {
      configurable: true,
      value: false,
    });
    Object.defineProperty(window, "MediaRecorder", {
      configurable: true,
      value: class MediaRecorderDouble {},
    });
    Object.defineProperty(navigator, "mediaDevices", {
      configurable: true,
      value: { getUserMedia: vi.fn() },
    });

    expect(micSupported()).toBe(false);
  });

  it("is true when secure capture and MediaRecorder are available", () => {
    Object.defineProperty(window, "isSecureContext", {
      configurable: true,
      value: true,
    });
    Object.defineProperty(window, "MediaRecorder", {
      configurable: true,
      value: class MediaRecorderDouble {},
    });
    Object.defineProperty(navigator, "mediaDevices", {
      configurable: true,
      value: { getUserMedia: vi.fn() },
    });

    expect(micSupported()).toBe(true);
  });

  it("explains when Chrome cannot see an input device", () => {
    const inspection: MicrophoneInspection = {
      secureContext: true,
      supported: true,
      permissionState: "granted",
      audioInputCount: 0,
      labeledAudioInputCount: 0,
      devicesError: null,
    };

    const message = describeMicrophoneError(
      new DOMException("Requested device not found", "NotFoundError"),
      inspection,
    );

    expect(message).toContain("Chrome cannot see a microphone input");
  });

  it("explains denied browser permission", () => {
    const inspection: MicrophoneInspection = {
      secureContext: true,
      supported: true,
      permissionState: "denied",
      audioInputCount: 1,
      labeledAudioInputCount: 1,
      devicesError: null,
    };

    const message = describeMicrophoneError(
      new DOMException("Permission denied", "NotAllowedError"),
      inspection,
    );

    expect(message).toContain("Browser denied microphone access");
  });
});
