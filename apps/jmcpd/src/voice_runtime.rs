//! Runtime glue for two-way Telegram voice approvals.
//!
//! Inbound: a voice note → download (`getFile`) → ASR sidecar → the pure
//! [`evaluate_voice_approval`] decision → apply via the same
//! `decide_approval_by_token` path as typed `/approve` → reply.
//! Outbound: every reply is sent as text and (optionally) spoken back as a
//! Telegram voice note via the TTS sidecar (`sendVoice`).
//!
//! All speech I/O is isolated here; the decision logic lives in
//! [`crate::voice_approval`] and is unit-tested without any network.

use jmcp_adapter_speech::{AsrClient, AudioFormat, TtsClient};
use jmcp_app::AppState;
use jmcp_approval_telegram::{TelegramBotClient, TelegramVoice};
use serde_json::json;

use crate::telegram_helpers::{decide_from_telegram, emit_structured_event};
use crate::voice_approval::{evaluate_voice_approval, VoiceChallengeBook};

/// Speech clients + the in-memory challenge book for the voice approval loop.
pub struct VoiceContext {
    asr: AsrClient,
    tts: TtsClient,
    pub book: VoiceChallengeBook,
    speak_replies: bool,
}

impl VoiceContext {
    /// Build from `JMCP_ASR_URL` / `JMCP_TTS_URL` (defaulting to the local
    /// sidecar ports). `speak_replies` also sends each reply as a voice note.
    pub fn from_env(book: VoiceChallengeBook, speak_replies: bool) -> Self {
        Self {
            asr: AsrClient::from_env(),
            tts: TtsClient::from_env(),
            book,
            speak_replies,
        }
    }

    /// Speak `text` back to the chat as a Telegram voice note (best-effort).
    async fn speak(&self, client: &TelegramBotClient, chat_id: i64, text: &str) {
        if !self.speak_replies {
            return;
        }
        match self
            .tts
            .synthesize_as(text, None, None, AudioFormat::OggOpus)
            .await
        {
            Ok(ogg) => {
                let _ = client.send_voice(chat_id, ogg, None).await;
            }
            Err(err) => emit_structured_event(
                "warn",
                "telegram.voice.tts_failed",
                json!({ "chatId": chat_id, "error": err.to_string() }),
            ),
        }
    }

    /// Send a text reply and (optionally) its spoken voice note.
    async fn reply(&self, client: &TelegramBotClient, chat_id: i64, text: &str) {
        let _ = client.send_message(chat_id, text).await;
        self.speak(client, chat_id, text).await;
    }
}

/// Handle one inbound voice note end to end. Never returns an error for an
/// approver-visible failure — it replies with a friendly message instead.
pub async fn handle_voice_message(
    client: &TelegramBotClient,
    state: &AppState,
    ctx: &VoiceContext,
    chat_id: i64,
    user_id: i64,
    voice: &TelegramVoice,
) {
    let audio = match client.download_voice(&voice.file_id).await {
        Ok(audio) => audio,
        Err(err) => {
            emit_structured_event(
                "warn",
                "telegram.voice.download_failed",
                json!({ "userId": user_id, "error": err.to_string() }),
            );
            ctx.reply(client, chat_id, "I could not download your voice note.")
                .await;
            return;
        }
    };

    let transcription = match ctx.asr.transcribe(audio, Some("en")).await {
        Ok(transcription) => transcription,
        Err(err) => {
            emit_structured_event(
                "warn",
                "telegram.voice.asr_failed",
                json!({ "userId": user_id, "error": err.to_string() }),
            );
            ctx.reply(
                client,
                chat_id,
                "Voice transcription is unavailable right now. Please type /approve or /deny.",
            )
            .await;
            return;
        }
    };

    let plan = evaluate_voice_approval(
        &ctx.book,
        user_id,
        &transcription.text,
        transcription.confidence,
        chrono::Utc::now(),
    );

    let mut reply = plan.reply;
    let mut decided = "none";
    if let Some(apply) = plan.apply {
        let outcome = decide_from_telegram(state, &apply.token, user_id, chat_id, apply.decision);
        ctx.book.forget(user_id);
        decided = match apply.decision {
            jmcp_domain::ApprovalDecision::Approved => "approved",
            jmcp_domain::ApprovalDecision::Rejected => "rejected",
        };
        reply = format!("{reply} {outcome}");
    }

    emit_structured_event(
        "info",
        "telegram.voice.handled",
        json!({
            "userId": user_id,
            "chatId": chat_id,
            "transcript": transcription.text,
            "confidence": transcription.confidence,
            "decision": decided,
        }),
    );
    ctx.reply(client, chat_id, &reply).await;
}
