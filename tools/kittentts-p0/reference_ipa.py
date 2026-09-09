#!/usr/bin/env python3
"""Generate P0-only official Kitten v0.8 eSpeak model-input references.

This tool is intentionally NOT a production dependency. It uses the same
GPL phonemizer/eSpeak boundary as the upstream Kitten Python implementation
only to measure a permissive candidate during P0.
"""

from __future__ import annotations

import json
import os
import re
import sys
from pathlib import Path

import espeakng_loader
import phonemizer
from phonemizer.backend.espeak.wrapper import EspeakWrapper

PUNCTUATION = ';:,.!?¡¿—…“«»”" '
LETTERS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"
IPA_LETTERS = (
    "ɑɐɒæɓʙβɔɕçɗɖðʤəɘɚɛɜɝɞɟʄɡɠɢʛɦɧħɥʜɨɪʝɭɬɫɮʟɱɯɰŋɳɲɴøɵɸθœɶʘ"
    "ɹɺɾɻʀʁɽʂʃʈʧʉʊʋⱱʌɣɤʍχʎʏʑʐʒʔʡʕʢǀǁǂǃˈˌːˑʼʴʰʱʲʷˠˤ˞↓↑→↗↘’̩‘ᵻ"
)
VOCAB = {char: index for index, char in enumerate(["$"] + list(PUNCTUATION) + list(LETTERS) + list(IPA_LETTERS))}
TOKEN_RE = re.compile(r"\w+|[^\w\s]")


def model_token_ids(ipa: str) -> list[int]:
    tokenized = " ".join(TOKEN_RE.findall(ipa))
    ids = [VOCAB[ch] for ch in tokenized if ch in VOCAB]
    # Exact KittenTTS v0.8 Python _prepare_inputs() framing.
    ids.insert(0, 0)
    ids.append(10)
    ids.append(0)
    return ids


def main() -> int:
    if len(sys.argv) != 3:
        raise SystemExit(f"usage: {sys.argv[0]} <corpus.json> <reference.json>")

    corpus_path = Path(sys.argv[1])
    output_path = Path(sys.argv[2])
    corpus = json.loads(corpus_path.read_text(encoding="utf-8"))

    EspeakWrapper.set_library(espeakng_loader.get_library_path())
    data_path = espeakng_loader.get_data_path()
    os.environ["ESPEAK_DATA_PATH"] = str(data_path)
    if hasattr(EspeakWrapper, "set_data_path"):
        EspeakWrapper.set_data_path(str(data_path))

    # Mirror KittenTTS v0.8's public phonemizer API exactly.  phonemizer 3.4
    # exposes EspeakBackend from phonemizer.backend rather than the espeak
    # package's __init__ module.
    backend = phonemizer.backend.EspeakBackend(
        language="en-us",
        preserve_punctuation=True,
        with_stress=True,
    )

    cases = []
    for case in corpus["cases"]:
        text = case["normalized_text"]
        ipa = backend.phonemize([text])[0]
        cases.append(
            {
                "id": case["id"],
                "category": case["category"],
                "normalized_text": text,
                "official_ipa": ipa,
                "official_model_token_ids": model_token_ids(ipa),
            }
        )

    output = {
        "schema_version": 1,
        "oracle": "KittenTTS v0.8 EspeakBackend(en-us,preserve_punctuation=true,with_stress=true)",
        "model_input_framing": "prepend 0; append 10,0",
        "espeak_library": str(espeakng_loader.get_library_path()),
        "espeak_data": str(data_path),
        "cases": cases,
    }
    output_path.write_text(json.dumps(output, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
