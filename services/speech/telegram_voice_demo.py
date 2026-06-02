#!/usr/bin/env python3
"""Two-way Telegram voice-note demo over the JMCP speech sidecars.

Outbound  (JMCP -> Telegram): text -> TTS sidecar (WAV) -> OGG/Opus -> sendVoice.
Inbound   (Telegram -> JMCP): voice note -> getFile/download -> ASR sidecar -> text,
                              then reply with the transcript (and optionally speak it back).

Run with the TTS venv python (needs soundfile + numpy); everything else is stdlib.

  .venv-tts/bin/python telegram_voice_demo.py discover
  .venv-tts/bin/python telegram_voice_demo.py send <chat_id> "your message"
  .venv-tts/bin/python telegram_voice_demo.py listen [--reply-voice] [--seconds 60]

Env: TELEGRAM_ENV (default ./telegram.env, a bare bot token),
     ASR_URL=http://127.0.0.1:18878  TTS_URL=http://127.0.0.1:18901
"""
import io
import json
import os
import sys
import time
import urllib.request

ASR_URL = os.environ.get("ASR_URL", "http://127.0.0.1:18878")
TTS_URL = os.environ.get("TTS_URL", "http://127.0.0.1:18901")
TOKEN = open(os.environ.get("TELEGRAM_ENV", "telegram.env")).read().strip()
API = f"https://api.telegram.org/bot{TOKEN}"
FILE_API = f"https://api.telegram.org/file/bot{TOKEN}"


def _get(url, timeout=70):
    with urllib.request.urlopen(url, timeout=timeout) as r:
        return json.load(r)


def _post_bytes(url, data, content_type, timeout=120):
    req = urllib.request.Request(url, data=data, headers={"content-type": content_type})
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return r.read()


def _multipart(fields, filename, file_bytes, file_field="voice", ctype="audio/ogg"):
    boundary = "----jmcpvoice" + str(int(time.time() * 1000))
    out = io.BytesIO()
    for key, value in fields.items():
        out.write(f"--{boundary}\r\n".encode())
        out.write(f'Content-Disposition: form-data; name="{key}"\r\n\r\n'.encode())
        out.write(f"{value}\r\n".encode())
    out.write(f"--{boundary}\r\n".encode())
    out.write(
        f'Content-Disposition: form-data; name="{file_field}"; filename="{filename}"\r\n'.encode()
    )
    out.write(f"Content-Type: {ctype}\r\n\r\n".encode())
    out.write(file_bytes)
    out.write(f"\r\n--{boundary}--\r\n".encode())
    return f"multipart/form-data; boundary={boundary}", out.getvalue()


def tts_to_ogg(text, voice=None):
    """Synthesize via the TTS sidecar, return OGG/Opus bytes Telegram can play."""
    import numpy as np
    import soundfile as sf

    payload = {"text": text}
    if voice:
        payload["voice"] = voice
    wav = _post_bytes(
        f"{TTS_URL}/synthesize", json.dumps(payload).encode(), "application/json"
    )
    samples, rate = sf.read(io.BytesIO(wav), dtype="float32")
    if getattr(samples, "ndim", 1) > 1:
        samples = np.mean(samples, axis=1)
    buf = io.BytesIO()
    sf.write(buf, samples, rate, format="OGG", subtype="OPUS")
    return buf.getvalue()


def send_voice(chat_id, text, voice=None):
    ogg = tts_to_ogg(text, voice)
    ctype, body = _multipart(
        {"chat_id": str(chat_id), "caption": "JMCP voice"}, "jmcp.ogg", ogg
    )
    resp = json.loads(_post_bytes(f"{API}/sendVoice", body, ctype))
    print(f"[send] sendVoice ok={resp.get('ok')} bytes={len(ogg)} chars={len(text)}")
    return resp


def send_message(chat_id, text):
    data = json.dumps({"chat_id": chat_id, "text": text}).encode()
    json.loads(_post_bytes(f"{API}/sendMessage", data, "application/json"))


def transcribe_ogg(audio_bytes, language="en"):
    out = _post_bytes(
        f"{ASR_URL}/transcribe?language={language}", audio_bytes, "audio/ogg"
    )
    return json.loads(out)


def discover():
    res = _get(f"{API}/getUpdates", timeout=15).get("result", [])
    print(f"updates: {len(res)}")
    for cid in {(_msg(u).get("chat") or {}).get("id") for u in res} - {None}:
        print(f"  chat_id={cid}")


def _msg(update):
    return update.get("message") or update.get("edited_message") or {}


def listen(reply_voice=False, seconds=60):
    print(f"[listen] waiting up to {seconds}s for a voice note… send one to the bot now.")
    deadline = time.time() + seconds
    offset = None
    while time.time() < deadline:
        url = f"{API}/getUpdates?timeout=20"
        if offset is not None:
            url += f"&offset={offset}"
        for update in _get(url).get("result", []):
            offset = update["update_id"] + 1
            msg = _msg(update)
            voice = msg.get("voice")
            chat_id = (msg.get("chat") or {}).get("id")
            if not voice:
                continue
            file_id = voice["file_id"]
            meta = _get(f"{API}/getFile?file_id={file_id}")
            path = meta["result"]["file_path"]
            with urllib.request.urlopen(f"{FILE_API}/{path}", timeout=60) as r:
                audio = r.read()
            result = transcribe_ogg(audio)
            text = result.get("text", "")
            print(f"[recv] {voice.get('duration')}s voice -> ASR: {text!r}")
            send_message(chat_id, f"📝 I heard: {text}")
            if reply_voice and text:
                send_voice(chat_id, f"You said: {text}")
            return text
    print("[listen] timed out with no voice note.")
    return None


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return
    cmd = sys.argv[1]
    if cmd == "discover":
        discover()
    elif cmd == "send":
        send_voice(sys.argv[2], sys.argv[3])
    elif cmd == "listen":
        listen("--reply-voice" in sys.argv, _opt_int("--seconds", 60))
    else:
        print(__doc__)


def _opt_int(flag, default):
    return int(sys.argv[sys.argv.index(flag) + 1]) if flag in sys.argv else default


if __name__ == "__main__":
    main()
