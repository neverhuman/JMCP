use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::{TelegramBotClient, TelegramConfig};

/// A multi-request stub Telegram API that routes canned responses by the request
/// target and records every full request for assertions.
struct StubApi {
    url: String,
    seen: Arc<Mutex<Vec<String>>>,
}

fn start_stub() -> StubApi {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let seen_writer = seen.clone();
    thread::spawn(move || {
        for conn in listener.incoming() {
            let Ok(mut stream) = conn else { break };
            let mut buf = vec![0u8; 16384];
            let n = stream.read(&mut buf).unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]).to_string();
            let target = request.lines().next().unwrap_or("").to_owned();
            seen_writer.lock().unwrap().push(request.clone());
            let (ctype, body): (&str, Vec<u8>) = if target.contains("getFile") {
                (
                    "application/json",
                    br#"{"ok":true,"result":{"file_id":"VOICE1","file_path":"voice/file_42.oga"}}"#
                        .to_vec(),
                )
            } else if target.contains("sendVoice") {
                (
                    "application/json",
                    br#"{"ok":true,"result":{"message_id":7,"chat":{"id":42,"type":"private"},"text":null}}"#
                        .to_vec(),
                )
            } else if target.contains("/file/") {
                ("audio/ogg", b"OggS-stub-voice-bytes".to_vec())
            } else {
                (
                    "application/json",
                    br#"{"ok":false,"description":"unexpected request"}"#.to_vec(),
                )
            };
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

fn client_for(api_base: &str) -> TelegramBotClient {
    let contents = format!(
        "TELEGRAM_BOT_TOKEN=test-token\nTELEGRAM_API_BASE={api_base}\nTELEGRAM_ALLOWED_USER_IDS=1\n"
    );
    TelegramBotClient::new(TelegramConfig::from_env_contents(&contents).unwrap())
}

#[tokio::test]
async fn get_file_resolves_download_path() {
    let stub = start_stub();
    let file = client_for(&stub.url).get_file("VOICE1").await.unwrap();
    assert_eq!(file.file_path.as_deref(), Some("voice/file_42.oga"));
    assert!(stub.seen.lock().unwrap()[0].contains("getFile"));
}

#[tokio::test]
async fn download_voice_resolves_then_fetches_bytes() {
    let stub = start_stub();
    let bytes = client_for(&stub.url)
        .download_voice("VOICE1")
        .await
        .unwrap();
    assert_eq!(bytes, b"OggS-stub-voice-bytes");
    let seen = stub.seen.lock().unwrap();
    assert!(seen.iter().any(|r| r.contains("getFile")));
    assert!(seen
        .iter()
        .any(|r| r.contains("/file/") && r.contains("file_42.oga")));
}

#[tokio::test]
async fn send_voice_uploads_multipart_audio() {
    let stub = start_stub();
    let message = client_for(&stub.url)
        .send_voice(42, b"OggS-audio-payload".to_vec(), Some("JMCP"))
        .await
        .unwrap();
    assert_eq!(message.message_id, 7);
    let seen = stub.seen.lock().unwrap();
    let request = seen.iter().find(|r| r.contains("sendVoice")).unwrap();
    assert!(request.contains("multipart/form-data"), "must be multipart");
    assert!(request.contains("name=\"voice\""), "voice part present");
    assert!(request.contains("name=\"chat_id\""), "chat_id part present");
    assert!(
        request.contains("OggS-audio-payload"),
        "the audio bytes are uploaded in the body"
    );
}

#[tokio::test]
async fn send_voice_surfaces_api_rejection() {
    // Point at a stub that always rejects (no matching route -> ok:false).
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 8192];
            let _ = stream.read(&mut buf);
            let body = br#"{"ok":false,"description":"VOICE_MESSAGES_FORBIDDEN"}"#;
            let header = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(body);
        }
    });
    let result = client_for(&format!("http://{addr}"))
        .send_voice(42, b"x".to_vec(), None)
        .await;
    assert!(matches!(
        result,
        Err(crate::TelegramApprovalError::ApiRejected(_))
    ));
}
