#!/usr/bin/env python3
"""Validate the deterministic Wake Word V1 corpus contract without reading audio."""
from __future__ import annotations
import json, posixpath
from pathlib import Path
from typing import Any
ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "docs" / "wake-word-corpus.json"
ARTIFACT_MANIFEST = ROOT / "wake-word-artifacts.json"
REQUIRED_LABELS = {"positive_wake_phrase","positive_wake_phrase_with_command","negative_ordinary_speech","negative_near_miss"}
REQUIRED_FIXTURE_FIELDS = {"id","label","path","speaker_or_source","provenance","license","bytes","sha256","sample_rate_hz","channels","sample_format","expected_detection"}
def _require(condition: bool, message: str) -> None:
    if not condition: raise AssertionError(message)
def _require_object(value: Any, name: str) -> dict[str, Any]:
    _require(isinstance(value, dict), f"{name} must be an object"); return value
def _require_sha256(value: Any, name: str) -> None:
    _require(isinstance(value,str) and len(value)==64 and value==value.lower() and all(c in "0123456789abcdef" for c in value), f"{name} must be lowercase hex sha256")
def _runtime_c_api_sha256(a: dict[str,Any], platform_key: str) -> str:
    for file in a["runtime"]["platforms"][platform_key]["files"]:
        if "sherpa-onnx-c-api" in file["path"]: return file["sha256"]
    raise AssertionError(f"artifact manifest missing {platform_key} C API file")
def validate(data: dict[str,Any]) -> None:
    artifacts=json.loads(ARTIFACT_MANIFEST.read_text(encoding="utf-8")); model=next(a for a in artifacts["artifacts"] if a["kind"]=="kws-model")
    _require(data.get("schema_version")==1,"schema_version must be 1"); _require(data.get("corpus_id")=="wake-word-v1-deterministic-corpus","unexpected corpus_id"); _require(data.get("wake_phrase")=="Hey, Moose","wake_phrase must be Hey, Moose")
    p=_require_object(data.get("policy"),"policy"); _require(p.get("sample_rate_hz")==16000,"policy.sample_rate_hz must be 16000"); _require(p.get("channels")==1,"policy.channels must be 1"); _require(p.get("sample_format")=="pcm_s16le","policy.sample_format must be pcm_s16le"); _require(p.get("score")==1.0,"policy.score must be 1.0"); _require(p.get("threshold")==0.25,"policy.threshold must be 0.25"); _require(p.get("fixture_root")=="docs/fixtures/wake-word-v1","policy.fixture_root must be docs/fixtures/wake-word-v1"); _require(p.get("fixture_schema_version")==1,"fixture_schema_version must be 1"); _require("Do not commit private room audio" in str(p.get("privacy","")),"privacy policy must forbid private room audio")
    i=_require_object(data.get("model_runtime_identity"),"model_runtime_identity"); _require(i.get("source_manifest")=="wake-word-artifacts.json","identity source drift"); _require(i.get("model_id")==model["id"],"identity model_id must match artifact manifest"); _require(i.get("model_archive_sha256")==model["archive"]["sha256"],"identity model_archive_sha256 must match artifact manifest"); _require(i.get("keyword_sha256")==model["keyword"]["sha256"],"identity keyword_sha256 must match artifact manifest"); _require(i.get("runtime_id")==artifacts["runtime"]["id"],"runtime_id drift"); _require(i.get("runtime_version")==artifacts["runtime"]["version"],"runtime_version drift"); _require(i.get("linux_x86_64_c_api_sha256")==_runtime_c_api_sha256(artifacts,"linux-x86_64"),"linux C API identity drift"); _require(i.get("macos_arm64_c_api_sha256")==_runtime_c_api_sha256(artifacts,"macos-arm64"),"macOS C API identity drift");
    for k,v in i.items():
        if k.endswith("sha256"): _require_sha256(v,f"identity.{k}")
    labels=data.get("required_labels"); _require(isinstance(labels,list),"required_labels must be a list"); _require(REQUIRED_LABELS <= set(labels),"required_labels is incomplete")
    c=_require_object(data.get("acceptance_criteria"),"acceptance_criteria"); _require(c.get("criteria_version")==1,"criteria_version must be 1"); _require(c.get("criteria_status")=="pending_real_fixture_calibration","criteria_status must remain pending until real fixture calibration"); _require(c.get("positive_recall_minimum") is None,"positive recall must remain uncalibrated"); _require(c.get("negative_false_accepts_maximum") is None,"negative false-accept threshold must remain uncalibrated")
    fixtures=data.get("fixtures"); _require(isinstance(fixtures,list),"fixtures must be a list"); ids=set(); seen=set()
    for f in fixtures:
        f=_require_object(f,"fixture"); missing=REQUIRED_FIXTURE_FIELDS-set(f); _require(not missing,f"fixture is missing fields: {sorted(missing)}"); fid=f["id"]; _require(isinstance(fid,str) and fid,"fixture id must be non-empty"); _require(fid not in ids,f"duplicate fixture id {fid}"); ids.add(fid); label=f["label"]; _require(label in REQUIRED_LABELS,f"unknown fixture label {label}"); seen.add(label); _require(f["sample_rate_hz"]==p["sample_rate_hz"],f"{fid}: sample rate drift"); _require(f["channels"]==p["channels"],f"{fid}: channel drift"); _require(f["sample_format"]==p["sample_format"],f"{fid}: sample format drift"); _require(isinstance(f["bytes"],int) and f["bytes"]>0,f"{fid}: bytes must be positive"); _require_sha256(f["sha256"],f"{fid}: sha256"); _require(isinstance(f["expected_detection"],bool),f"{fid}: expected_detection must be boolean"); _require_object(f["provenance"],f"{fid}.provenance"); lic=_require_object(f["license"],f"{fid}.license"); _require(lic.get("spdx"),f"{fid}: license.spdx is required"); _require(lic.get("redistributable") is True,f"{fid}: license must be redistributable"); fp=f["path"]; _require(isinstance(fp,str),f"{fid}: path must be a string"); norm=posixpath.normpath(fp); _require(norm==fp,f"{fid}: path must be normalized"); _require(not norm.startswith("../") and not posixpath.isabs(norm),f"{fid}: path must be relative"); _require(norm.startswith(f"{p['fixture_root']}/"),f"{fid}: path escapes fixture root")
    if fixtures: _require(REQUIRED_LABELS <= seen,"non-empty corpus must cover all required labels")
def main() -> None:
    validate(json.loads(MANIFEST.read_text(encoding="utf-8"))); print(f"validated {MANIFEST.relative_to(ROOT)}")
if __name__ == "__main__": main()
