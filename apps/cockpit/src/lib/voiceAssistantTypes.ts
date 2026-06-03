export type VoiceState =
  | "off"
  | "listening"
  | "armed"
  | "transcribing"
  | "thinking"
  | "speaking"
  | "error";

export interface VoiceAssistantApi {
  state: VoiceState;
  supported: boolean;
  transcript: string;
  reply: string;
  error: string | null;
  wakeWords: string[];
  start: () => Promise<void>;
  stop: () => void;
  sendText: (text: string) => Promise<void>;
}
