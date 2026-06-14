#!/usr/bin/env python3
"""Paraformer 常驻识别进程：stdin 接收 WAV 路径，stdout 返回 JSON 结果。
模型只在启动时加载一次，后续请求直接推理。"""
import sys
import json
import numpy as np
import sherpa_onnx
import wave

def main():
    if len(sys.argv) < 3:
        print(json.dumps({"error": "Usage: paraformer-offline.py <model.onnx> <tokens.txt>"}))
        sys.exit(1)

    model_path = sys.argv[1]
    tokens_path = sys.argv[2]

    try:
        recognizer = sherpa_onnx.OfflineRecognizer.from_paraformer(
            paraformer=model_path,
            tokens=tokens_path,
            num_threads=4,
        )
        print(json.dumps({"status": "ready"}), flush=True)

        for line in sys.stdin:
            line = line.strip()
            if not line:
                continue
            if line == "quit":
                break

            wav_file = line
            try:
                with wave.open(wav_file, 'rb') as wf:
                    sr = wf.getframerate()
                    n = wf.getnframes()
                    raw = wf.readframes(n)

                samples = np.frombuffer(raw, dtype=np.int16).astype(np.float32) / 32768.0

                stream = recognizer.create_stream()
                stream.accept_waveform(sr, samples)
                recognizer.decode_stream(stream)

                text = stream.result.text.strip()
                print(json.dumps({"text": text}), flush=True)
            except Exception as e:
                print(json.dumps({"error": str(e)}), flush=True)

    except Exception as e:
        print(json.dumps({"error": str(e)}))
        sys.exit(1)

if __name__ == "__main__":
    main()
