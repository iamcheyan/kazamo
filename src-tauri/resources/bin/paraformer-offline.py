#!/usr/bin/env python3
"""Offline Paraformer transcription via sherpa-onnx Python API (no WebSocket)."""
import sys
import json
import numpy as np
import sherpa_onnx
import wave

def main():
    if len(sys.argv) < 4:
        print(json.dumps({"error": "Usage: paraformer-offline.py <model_dir> <tokens> <wav_file>"}))
        sys.exit(1)

    model_dir = sys.argv[1]
    tokens = sys.argv[2]
    wav_file = sys.argv[3]

    try:
        recognizer = sherpa_onnx.OfflineRecognizer.from_paraformer(
            paraformer=model_dir,
            tokens=tokens,
            num_threads=4,
        )

        with wave.open(wav_file, 'rb') as wf:
            sr = wf.getframerate()
            n = wf.getnframes()
            raw = wf.readframes(n)

        samples = np.frombuffer(raw, dtype=np.int16).astype(np.float32) / 32768.0

        stream = recognizer.create_stream()
        stream.accept_waveform(sr, samples)
        recognizer.decode_stream(stream)

        text = stream.result.text.strip()
        print(json.dumps({"text": text}))
    except Exception as e:
        print(json.dumps({"error": str(e)}))
        sys.exit(1)

if __name__ == "__main__":
    main()
