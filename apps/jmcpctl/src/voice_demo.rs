use anyhow::{Context, Result};
use jmcp_adapter_speech::{AsrClient, AudioFormat, TtsClient};
use jmcp_approval_telegram::{TelegramBotClient, TelegramConfig};
use std::collections::BTreeSet;
use std::path::Path;
use std::time::{Duration, Instant};

/// Rust replacement for the old Python Telegram voice demo.
///
/// The demo keeps the same three behaviors:
/// - discover Telegram updates and report chat ids;
/// - synthesize text to OGG/Opus and upload it via `sendVoice`;
/// - wait for a voice note, download it, transcribe it, and reply.
pub(crate) struct VoiceDemo {
    telegram: TelegramBotClient,
    asr: AsrClient,
    tts: TtsClient,
}

/// Result of the `discover` subcommand.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct DiscoverResult {
    pub update_count: usize,
    pub chat_ids: BTreeSet<i64>,
}

/// Result of the `send` subcommand.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct VoiceSendResult {
    pub message_id: i64,
    pub bytes: usize,
    pub chars: usize,
}

/// Result of the `listen` subcommand when a voice note is received.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ListenResult {
    pub chat_id: i64,
    pub voice_duration: Option<i64>,
    pub transcript: String,
    pub reply_voice_sent: bool,
}

impl VoiceDemo {
    pub(crate) fn new(telegram: TelegramBotClient, asr: AsrClient, tts: TtsClient) -> Self {
        Self { telegram, asr, tts }
    }

    pub(crate) fn from_env_file(path: impl AsRef<Path>) -> Result<Self> {
        let config = TelegramConfig::from_env_file_for_setup(path)?;
        Ok(Self::new(
            TelegramBotClient::new(config),
            AsrClient::from_env(),
            TtsClient::from_env(),
        ))
    }

    pub(crate) async fn discover(&self) -> Result<DiscoverResult> {
        let updates = self.telegram.get_updates(None, 0).await?;
        let mut chat_ids = BTreeSet::new();
        for update in &updates {
            if let Some(message) = &update.message {
                chat_ids.insert(message.chat.id);
            }
        }
        Ok(DiscoverResult {
            update_count: updates.len(),
            chat_ids,
        })
    }

    pub(crate) async fn send(&self, chat_id: i64, text: &str) -> Result<VoiceSendResult> {
        let ogg = self
            .tts
            .synthesize_as(text, None, None, AudioFormat::OggOpus)
            .await
            .context("synthesize Telegram voice note")?;
        let bytes = ogg.len();
        let message = self
            .telegram
            .send_voice(chat_id, ogg, Some("JMCP voice"))
            .await
            .context("send Telegram voice note")?;
        Ok(VoiceSendResult {
            message_id: message.message_id,
            bytes,
            chars: text.chars().count(),
        })
    }

    pub(crate) async fn listen(
        &self,
        reply_voice: bool,
        seconds: u64,
    ) -> Result<Option<ListenResult>> {
        let deadline = Instant::now() + Duration::from_secs(seconds);
        let mut offset = None;

        while Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }

            let poll_seconds = remaining.as_secs().clamp(1, 20);
            let updates = self.telegram.get_updates(offset, poll_seconds).await?;
            for update in updates {
                offset = Some(update.update_id + 1);
                let Some(message) = update.message else {
                    continue;
                };
                let Some(voice) = message.voice else {
                    continue;
                };

                let chat_id = message.chat.id;
                let audio = self
                    .telegram
                    .download_voice(&voice.file_id)
                    .await
                    .context("download Telegram voice note")?;
                let transcription = self
                    .asr
                    .transcribe(audio, Some("en"))
                    .await
                    .context("transcribe Telegram voice note")?;
                let transcript = transcription.text;
                self.telegram
                    .send_message(chat_id, &format!("I heard: {transcript}"))
                    .await
                    .context("send text reply")?;

                let reply_voice_sent = if reply_voice && !transcript.is_empty() {
                    let reply = self
                        .tts
                        .synthesize_as(
                            &format!("You said: {transcript}"),
                            None,
                            None,
                            AudioFormat::OggOpus,
                        )
                        .await
                        .context("synthesize voice reply")?;
                    self.telegram
                        .send_voice(chat_id, reply, Some("JMCP voice"))
                        .await
                        .context("send voice reply")?;
                    true
                } else {
                    false
                };

                return Ok(Some(ListenResult {
                    chat_id,
                    voice_duration: Some(voice.duration),
                    transcript,
                    reply_voice_sent,
                }));
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jmcp_approval_telegram::TelegramConfig;
    use serde_json::{json, Value};
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread;

    struct StubApi {
        url: String,
        seen: Arc<Mutex<Vec<String>>>,
    }

    fn start_stub<F>(handler: F) -> StubApi
    where
        F: Fn(&str, &str) -> (String, Vec<u8>) + Send + Sync + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let seen_writer = seen.clone();
        let handler = Arc::new(handler);
        thread::spawn(move || {
            for conn in listener.incoming() {
                let Ok(mut stream) = conn else {
                    break;
                };
                let mut buf = vec![0u8; 32768];
                let n = stream.read(&mut buf).unwrap_or(0);
                let request = String::from_utf8_lossy(&buf[..n]).to_string();
                let target = request.lines().next().unwrap_or_default().to_owned();
                seen_writer.lock().unwrap().push(request.clone());
                let (ctype, body) = handler(&target, &request);
                let header = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: {ctype}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(header.as_bytes());
                let _ = stream.write_all(&body);
                let _ = stream.flush();
            }
        });
        StubApi {
            url: format!("http://{addr}"),
            seen,
        }
    }

    fn telegram_config(api_base: &str) -> TelegramConfig {
        TelegramConfig::from_env_contents_for_setup(&format!(
            "TELEGRAM_BOT_TOKEN=test-token\nTELEGRAM_API_BASE={api_base}\n"
        ))
        .unwrap()
    }

    fn demo_for(api_base: &str, asr_base: &str, tts_base: &str) -> VoiceDemo {
        VoiceDemo::new(
            TelegramBotClient::new(telegram_config(api_base)),
            AsrClient::new(asr_base.to_owned()),
            TtsClient::new(tts_base.to_owned()),
        )
    }

    fn sample_voice_update() -> Value {
        json!({
            "ok": true,
            "result": [
                {
                    "update_id": 11,
                    "message": {
                        "message_id": 1,
                        "from": {
                            "id": 42,
                            "is_bot": false,
                            "first_name": "Ada",
                            "username": "ada"
                        },
                        "chat": { "id": 77, "type": "private" },
                        "voice": {
                            "file_id": "VOICE1",
                            "duration": 7,
                            "mime_type": "audio/ogg"
                        },
                        "text": null
                    }
                }
            ]
        })
    }

    #[tokio::test]
    async fn discover_reports_unique_chat_ids() {
        let telegram = start_stub(|target, _request| {
            if target.contains("getUpdates") {
                (
                    "application/json".to_owned(),
                    json!({
                        "ok": true,
                        "result": [
                            {
                                "update_id": 1,
                                "message": {
                                    "message_id": 1,
                                    "chat": { "id": 42, "type": "private" },
                                    "text": "/start"
                                }
                            },
                            {
                                "update_id": 2,
                                "message": {
                                    "message_id": 2,
                                    "chat": { "id": 42, "type": "private" },
                                    "text": "hello"
                                }
                            },
                            {
                                "update_id": 3,
                                "message": {
                                    "message_id": 3,
                                    "chat": { "id": 99, "type": "group" },
                                    "text": "hi"
                                }
                            }
                        ]
                    })
                    .to_string()
                    .into_bytes(),
                )
            } else {
                (
                    "application/json".to_owned(),
                    br#"{"ok":false,"description":"unexpected request"}"#.to_vec(),
                )
            }
        });
        let demo = demo_for(&telegram.url, "http://127.0.0.1:1", "http://127.0.0.1:2");

        let result = demo.discover().await.unwrap();
        assert_eq!(result.update_count, 3);
        assert_eq!(result.chat_ids, BTreeSet::from([42, 99]));
        assert!(telegram.seen.lock().unwrap()[0].contains("getUpdates"));
    }

    #[tokio::test]
    async fn send_synthesizes_ogg_and_uploads_voice_note() {
        let tts = start_stub(|target, request| {
            if target.contains("synthesize?format=ogg") {
                assert!(request.contains("\"text\":\"hello there\""));
                ("audio/ogg".to_owned(), b"OggS-demo-voice".to_vec())
            } else {
                (
                    "application/json".to_owned(),
                    br#"{"ok":false,"description":"unexpected request"}"#.to_vec(),
                )
            }
        });
        let telegram = start_stub(|target, request| {
            if target.contains("sendVoice") {
                assert!(request.contains("multipart/form-data"));
                assert!(request.contains("name=\"voice\""));
                assert!(request.contains("OggS-demo-voice"));
                (
                    "application/json".to_owned(),
                    br#"{"ok":true,"result":{"message_id":7,"chat":{"id":42,"type":"private"},"text":null}}"#
                        .to_vec(),
                )
            } else {
                (
                    "application/json".to_owned(),
                    br#"{"ok":false,"description":"unexpected request"}"#.to_vec(),
                )
            }
        });
        let demo = demo_for(&telegram.url, "http://127.0.0.1:1", &tts.url);

        let result = demo.send(42, "hello there").await.unwrap();
        assert_eq!(result.message_id, 7);
        assert_eq!(result.bytes, "OggS-demo-voice".len());
        assert_eq!(result.chars, "hello there".chars().count());
        assert!(tts.seen.lock().unwrap()[0].contains("synthesize?format=ogg"));
        assert!(telegram.seen.lock().unwrap()[0].contains("sendVoice"));
    }

    #[tokio::test]
    async fn listen_downloads_transcribes_and_replies() {
        let telegram = start_stub(|target, request| {
            if target.contains("getUpdates") {
                (
                    "application/json".to_owned(),
                    sample_voice_update().to_string().into_bytes(),
                )
            } else if target.contains("getFile") {
                (
                    "application/json".to_owned(),
                    json!({
                        "ok": true,
                        "result": {
                            "file_id": "VOICE1",
                            "file_path": "voice/file_42.oga"
                        }
                    })
                    .to_string()
                    .into_bytes(),
                )
            } else if target.contains("/file/") {
                assert!(request.contains("voice/file_42.oga"));
                ("audio/ogg".to_owned(), b"OggS-stub-voice-bytes".to_vec())
            } else if target.contains("sendMessage") {
                assert!(request.contains("I heard: please approve"));
                (
                    "application/json".to_owned(),
                    br#"{"ok":true,"result":{"message_id":8,"chat":{"id":77,"type":"private"},"text":"I heard: please approve"}}"#
                        .to_vec(),
                )
            } else if target.contains("sendVoice") {
                assert!(request.contains("multipart/form-data"));
                assert!(request.contains("OggS-reply-voice"));
                (
                    "application/json".to_owned(),
                    br#"{"ok":true,"result":{"message_id":9,"chat":{"id":77,"type":"private"},"text":null}}"#
                        .to_vec(),
                )
            } else {
                (
                    "application/json".to_owned(),
                    br#"{"ok":false,"description":"unexpected request"}"#.to_vec(),
                )
            }
        });
        let asr = start_stub(|target, request| {
            if target.contains("transcribe?language=en") {
                assert!(request.contains("OggS-stub-voice-bytes"));
                (
                    "application/json".to_owned(),
                    json!({
                        "text": "please approve",
                        "language": "en",
                        "language_probability": 0.99,
                        "confidence": 0.98,
                        "duration": 3.2,
                        "rtf": 0.07,
                        "segments": []
                    })
                    .to_string()
                    .into_bytes(),
                )
            } else {
                (
                    "application/json".to_owned(),
                    br#"{"ok":false,"description":"unexpected request"}"#.to_vec(),
                )
            }
        });
        let tts = start_stub(|target, request| {
            if target.contains("synthesize?format=ogg") {
                assert!(request.contains("\"text\":\"You said: please approve\""));
                ("audio/ogg".to_owned(), b"OggS-reply-voice".to_vec())
            } else {
                (
                    "application/json".to_owned(),
                    br#"{"ok":false,"description":"unexpected request"}"#.to_vec(),
                )
            }
        });
        let demo = demo_for(&telegram.url, &asr.url, &tts.url);

        let result = demo.listen(true, 1).await.unwrap().unwrap();
        assert_eq!(result.chat_id, 77);
        assert_eq!(result.voice_duration, Some(7));
        assert_eq!(result.transcript, "please approve");
        assert!(result.reply_voice_sent);
        let seen = telegram.seen.lock().unwrap();
        assert!(seen.iter().any(|r| r.contains("getUpdates")));
        assert!(seen.iter().any(|r| r.contains("getFile")));
        assert!(seen.iter().any(|r| r.contains("/file/")));
        assert!(seen.iter().any(|r| r.contains("sendMessage")));
        assert!(seen.iter().any(|r| r.contains("sendVoice")));
    }
}
