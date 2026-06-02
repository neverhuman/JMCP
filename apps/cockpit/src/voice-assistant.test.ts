import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { stripWakeWord } from "./hooks/useVoiceAssistant";
import {
  VOICE_MODEL,
  micSupported,
  reason,
  synthesize,
  transcribe,
} from "./lib/speechClient";

// A minimal stand-in for the Fetch API Response surface that speechClient reads:
// `.ok`, `.json()`, and `.blob()`. Each test builds one of these via the helpers
// below so we never touch the real network.
interface ResponseLike {
  ok: boolean;
  status: number;
  json: () => Promise<unknown>;
  blob: () => Promise<Blob>;
}

function jsonResponse(value: unknown, ok = true, status = 200): ResponseLike {
  return {
    ok,
    status,
    json: () => Promise.resolve(value),
    blob: () => Promise.resolve(new Blob()),
  };
}

function blobResponse(payload: Blob, ok = true, status = 200): ResponseLike {
  return {
    ok,
    status,
    json: () => Promise.resolve(null),
    blob: () => Promise.resolve(payload),
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

  it("throws when the sidecar response is not ok", async () => {
    installFetch(() => Promise.resolve(jsonResponse({}, false, 503)));
    await expect(transcribe(new Blob())).rejects.toThrow("ASR 503");
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

  it("throws when the reasoning endpoint response is not ok", async () => {
    installFetch(() => Promise.resolve(jsonResponse({}, false, 500)));
    await expect(reason([{ role: "user", content: "x" }])).rejects.toThrow("LLM 500");
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

  it("throws when the synthesis endpoint response is not ok", async () => {
    installFetch(() => Promise.resolve(blobResponse(new Blob(), false, 404)));
    await expect(synthesize("hello")).rejects.toThrow("TTS 404");
  });
});

describe("micSupported", () => {
  let savedMediaDevices: MediaDevices | undefined;

  beforeEach(() => {
    savedMediaDevices = navigator.mediaDevices;
  });

  afterEach(() => {
    Object.defineProperty(navigator, "mediaDevices", {
      configurable: true,
      value: savedMediaDevices,
    });
  });

  it("is false in a jsdom environment without microphone capture", () => {
    Object.defineProperty(navigator, "mediaDevices", {
      configurable: true,
      value: undefined,
    });
    expect(micSupported()).toBe(false);
  });
});
