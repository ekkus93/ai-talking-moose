#!/usr/bin/env python3
"""Validate the deterministic generated Wake Word V1 corpus recipe contract."""
from __future__ import annotations
import json, posixpath
from pathlib import Path
from typing import Any
ROOT=Path(__file__).resolve().parents[1]
MANIFEST=ROOT/"docs"/"wake-word-corpus.json"
ARTIFACT_MANIFEST=ROOT/"wake-word-artifacts.json"
REQUIRED_LABELS={"positive_wake_phrase","positive_wake_phrase_with_command","negative_ordinary_speech","negative_near_miss"}
REQUIRED_FIXTURE_FIELDS={"id","label","output","speaker_or_source","text","voice","speed_wpm","gain_db","distance_scale","noise_amplitude","expected_detection","provenance","license"}
def _require(condition: bool,message: str)->None:
    if not condition: raise AssertionError(message)
def _obj(value: Any,name: str)->dict[str,Any]:
    _require(isinstance(value,dict),f"{name} must be an object"); return value
def _sha(value: Any,name: str)->None:
    _require(isinstance(value,str) and len(value)==64 and value==value.lower() and all(c in "0123456789abcdef" for c in value),f"{name} must be lowercase hex sha256")
def _runtime_sha(a: dict[str,Any],platform: str)->str:
    for f in a["runtime"]["platforms"][platform]["files"]:
        if "sherpa-onnx-c-api" in f["path"]: return f["sha256"]
    raise AssertionError(f"artifact manifest missing {platform} C API file")
def validate(data: dict[str,Any])->None:
    artifacts=json.loads(ARTIFACT_MANIFEST.read_text(encoding="utf-8")); model=next(a for a in artifacts["artifacts"] if a["kind"]=="kws-model")
    _require(data.get("schema_version")==2,"schema_version must be 2"); _require(data.get("corpus_id")=="wake-word-v1-deterministic-corpus","unexpected corpus_id"); _require(data.get("wake_phrase")=="Hey, Moose","wake_phrase must be Hey, Moose")
    p=_obj(data.get("policy"),"policy"); _require(p.get("sample_rate_hz")==16000,"policy.sample_rate_hz must be 16000"); _require(p.get("channels")==1,"policy.channels must be 1"); _require(p.get("sample_format")=="pcm_s16le","policy.sample_format must be pcm_s16le"); _require(p.get("score")==1.0,"policy.score must be 1.0"); _require(p.get("threshold")==0.25,"policy.threshold must be 0.25"); _require(p.get("generator")=="scripts/generate_wake_word_corpus.py","generator path drift"); _require(p.get("generation_tool")=="espeak-ng","generation tool drift"); _require(p.get("generated_audio_is_ephemeral") is True,"generated audio must remain ephemeral"); _require("Do not commit private room audio" in str(p.get("privacy","")),"privacy policy must forbid private room audio")
    i=_obj(data.get("model_runtime_identity"),"model_runtime_identity"); _require(i.get("source_manifest")=="wake-word-artifacts.json","identity source drift"); _require(i.get("model_id")==model["id"],"identity model_id drift"); _require(i.get("model_archive_sha256")==model["archive"]["sha256"],"model archive identity drift"); _require(i.get("keyword_sha256")==model["keyword"]["sha256"],"keyword identity drift"); _require(i.get("runtime_id")==artifacts["runtime"]["id"],"runtime_id drift"); _require(i.get("runtime_version")==artifacts["runtime"]["version"],"runtime_version drift"); _require(i.get("linux_x86_64_c_api_sha256")==_runtime_sha(artifacts,"linux-x86_64"),"linux C API identity drift"); _require(i.get("macos_arm64_c_api_sha256")==_runtime_sha(artifacts,"macos-arm64"),"macOS C API identity drift")
    for k,v in i.items():
        if k.endswith("sha256"): _sha(v,f"identity.{k}")
    labels=data.get("required_labels"); _require(isinstance(labels,list) and REQUIRED_LABELS<=set(labels),"required_labels is incomplete")
    c=_obj(data.get("acceptance_criteria"),"acceptance_criteria"); _require(c.get("criteria_version")==2,"criteria_version must be 2"); _require(c.get("criteria_status")=="active_predeclared","criteria_status must be active_predeclared"); recall=c.get("positive_recall_minimum"); _require(isinstance(recall,(int,float)) and 0<recall<=1,"positive recall criterion must be active"); false_accepts=c.get("negative_false_accepts_maximum"); _require(isinstance(false_accepts,int) and false_accepts>=0,"negative false-accept criterion must be active")
    fixtures=data.get("fixtures"); _require(isinstance(fixtures,list) and fixtures,"fixtures must be a non-empty list"); ids=set(); outputs=set(); seen=set(); positives=0; negatives=0
    for f in fixtures:
        f=_obj(f,"fixture"); missing=REQUIRED_FIXTURE_FIELDS-set(f); _require(not missing,f"fixture is missing fields: {sorted(missing)}"); fid=f["id"]; _require(isinstance(fid,str) and fid and fid not in ids,f"invalid or duplicate fixture id {fid}"); ids.add(fid); label=f["label"]; _require(label in REQUIRED_LABELS,f"unknown fixture label {label}"); seen.add(label); output=f["output"]; _require(isinstance(output,str) and output and posixpath.basename(output)==output and output.endswith(".pcm"),f"{fid}: output must be a simple .pcm filename"); _require(output not in outputs,f"duplicate fixture output {output}"); outputs.add(output); _require(isinstance(f["text"],str) and f["text"].strip(),f"{fid}: text is required"); _require(isinstance(f["voice"],str) and f["voice"].strip(),f"{fid}: voice is required"); _require(isinstance(f["speed_wpm"],int) and 80<=f["speed_wpm"]<=300,f"{fid}: speed_wpm out of range"); _require(isinstance(f["gain_db"],(int,float)) and -30<=f["gain_db"]<=12,f"{fid}: gain_db out of range"); _require(isinstance(f["distance_scale"],(int,float)) and 0<f["distance_scale"]<=2,f"{fid}: distance_scale out of range"); _require(isinstance(f["noise_amplitude"],(int,float)) and 0<=f["noise_amplitude"]<=0.1,f"{fid}: noise_amplitude out of range"); _require(isinstance(f["expected_detection"],bool),f"{fid}: expected_detection must be boolean"); positives+=int(f["expected_detection"]); negatives+=int(not f["expected_detection"]); _obj(f["provenance"],f"{fid}.provenance"); lic=_obj(f["license"],f"{fid}.license"); _require(lic.get("spdx") and lic.get("redistributable") is True,f"{fid}: fixture recipe license must be redistributable")
    _require(REQUIRED_LABELS<=seen,"fixtures must cover all required labels"); _require(positives>0 and negatives>0,"fixtures must contain positive and negative cases")
def main()->None:
    validate(json.loads(MANIFEST.read_text(encoding="utf-8"))); print(f"validated {MANIFEST.relative_to(ROOT)}")
if __name__=="__main__": main()
