export type MicrophonePermissionState = PermissionState | "unsupported" | "unknown";

export interface MicrophoneInspection {
  secureContext: boolean;
  supported: boolean;
  permissionState: MicrophonePermissionState;
  audioInputCount: number;
  labeledAudioInputCount: number;
  devicesError: string | null;
}

export class MicrophoneAccessError extends Error {
  readonly inspection: MicrophoneInspection;
  readonly cause: unknown;

  constructor(message: string, inspection: MicrophoneInspection, cause?: unknown) {
    super(message);
    this.name = "MicrophoneAccessError";
    this.inspection = inspection;
    this.cause = cause;
  }
}

const PREFERRED_AUDIO_CONSTRAINTS: MediaStreamConstraints = {
  audio: {
    echoCancellation: { ideal: true },
    noiseSuppression: { ideal: true },
    autoGainControl: { ideal: true },
  },
};

const RELAXED_AUDIO_CONSTRAINTS: MediaStreamConstraints = {
  audio: true,
};

function hasWindow(): boolean {
  return typeof window !== "undefined";
}

function hasNavigator(): boolean {
  return typeof navigator !== "undefined";
}

export function micSupported(): boolean {
  return (
    hasWindow() &&
    window.isSecureContext === true &&
    hasNavigator() &&
    typeof navigator.mediaDevices?.getUserMedia === "function" &&
    typeof window.MediaRecorder === "function"
  );
}

function emptyInspection(devicesError: string | null = null): MicrophoneInspection {
  return {
    secureContext: hasWindow() ? window.isSecureContext === true : false,
    supported: micSupported(),
    permissionState: "unknown",
    audioInputCount: 0,
    labeledAudioInputCount: 0,
    devicesError,
  };
}

async function microphonePermissionState(): Promise<MicrophonePermissionState> {
  if (!hasNavigator() || typeof navigator.permissions?.query !== "function") {
    return "unsupported";
  }
  try {
    const status = await navigator.permissions.query({ name: "microphone" });
    return status.state;
  } catch {
    return "unknown";
  }
}

export async function inspectMicrophone(): Promise<MicrophoneInspection> {
  const base = emptyInspection();
  const permissionState = await microphonePermissionState();
  if (!hasNavigator() || typeof navigator.mediaDevices?.enumerateDevices !== "function") {
    return { ...base, permissionState, devicesError: "enumerateDevices unavailable" };
  }

  try {
    const devices = await navigator.mediaDevices.enumerateDevices();
    const audioInputs = devices.filter((device) => device.kind === "audioinput");
    return {
      ...base,
      permissionState,
      audioInputCount: audioInputs.length,
      labeledAudioInputCount: audioInputs.filter((device) => device.label.trim().length > 0).length,
      devicesError: null,
    };
  } catch (error) {
    return {
      ...base,
      permissionState,
      devicesError: error instanceof Error ? error.message : String(error),
    };
  }
}

function browserErrorName(error: unknown): string {
  return typeof error === "object" &&
    error !== null &&
    "name" in error &&
    typeof error.name === "string"
    ? error.name
    : "";
}

export function describeMicrophoneError(
  error: unknown,
  inspection: MicrophoneInspection = emptyInspection(),
): string {
  if (!inspection.secureContext) {
    return "Microphone requires localhost or HTTPS. Open the cockpit through http://localhost:15873/.";
  }
  if (!inspection.supported) {
    return "This browser cannot record microphone audio. Use current Chrome or Edge.";
  }
  if (inspection.permissionState === "denied") {
    return "Browser denied microphone access. Allow this site in Chrome microphone settings, then retry.";
  }

  const name = browserErrorName(error);
  if (name === "NotAllowedError" || name === "SecurityError" || name === "PermissionDeniedError") {
    return "Browser denied microphone access. Allow this site in Chrome microphone settings, then retry.";
  }
  if (name === "NotFoundError" || name === "DevicesNotFoundError") {
    if (inspection.audioInputCount === 0) {
      return "Chrome cannot see a microphone input. Check macOS input device and Chrome microphone access, then retry.";
    }
    return `Chrome found ${inspection.audioInputCount} microphone input(s), but none matched the request. Try the default input, then retry.`;
  }
  if (name === "NotReadableError" || name === "TrackStartError") {
    return "Microphone is busy or blocked by another app. Close other audio apps, then retry.";
  }
  if (name === "OverconstrainedError" || name === "ConstraintNotSatisfiedError") {
    return "Microphone constraints failed. Retrying with the default input should fix this.";
  }

  return error instanceof Error ? error.message : "microphone permission denied";
}

export async function requestMicrophoneStream(): Promise<MediaStream> {
  const before = await inspectMicrophone();
  if (!before.secureContext || !before.supported) {
    throw new MicrophoneAccessError(describeMicrophoneError(null, before), before);
  }

  try {
    return await navigator.mediaDevices.getUserMedia(PREFERRED_AUDIO_CONSTRAINTS);
  } catch (preferredError) {
    try {
      return await navigator.mediaDevices.getUserMedia(RELAXED_AUDIO_CONSTRAINTS);
    } catch (relaxedError) {
      const after = await inspectMicrophone();
      throw new MicrophoneAccessError(
        describeMicrophoneError(relaxedError, after),
        after,
        preferredError,
      );
    }
  }
}
