#!/usr/bin/env python3
from __future__ import annotations

import json
import sys
from pathlib import Path


def edit_distance(left: list[int], right: list[int]) -> int:
    previous = list(range(len(right) + 1))
    for i, lvalue in enumerate(left, start=1):
        current = [i]
        for j, rvalue in enumerate(right, start=1):
            current.append(
                min(
                    current[-1] + 1,
                    previous[j] + 1,
                    previous[j - 1] + (lvalue != rvalue),
                )
            )
        previous = current
    return previous[-1]


def main() -> int:
    if len(sys.argv) != 4:
        raise SystemExit(f"usage: {sys.argv[0]} <reference.json> <candidate.json> <comparison.json>")

    reference = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
    candidate = json.loads(Path(sys.argv[2]).read_text(encoding="utf-8"))
    output_path = Path(sys.argv[3])

    refs = {case["id"]: case for case in reference["cases"]}
    candidates = {case["id"]: case for case in candidate["cases"]}
    if refs.keys() != candidates.keys():
        missing = sorted(refs.keys() - candidates.keys())
        extra = sorted(candidates.keys() - refs.keys())
        raise SystemExit(f"corpus mismatch: missing={missing} extra={extra}")

    rows = []
    total_distance = 0
    total_reference = 0
    exact = 0
    synthesized = 0
    max_rtf = 0.0

    for case_id in refs:
        ref = refs[case_id]
        cand = candidates[case_id]
        ref_ids = ref["official_model_token_ids"]
        cand_ids = cand["candidate_model_token_ids"]
        distance = edit_distance(ref_ids, cand_ids)
        denominator = max(len(ref_ids), 1)
        normalized_distance = distance / denominator
        total_distance += distance
        total_reference += len(ref_ids)
        if distance == 0:
            exact += 1
        if cand["synthesized"]:
            synthesized += 1
            max_rtf = max(max_rtf, float(cand["rtf"]))

        rows.append(
            {
                "id": case_id,
                "category": ref["category"],
                "official_ipa": ref["official_ipa"],
                "candidate_ipa": cand["candidate_ipa"],
                "official_model_token_count": len(ref_ids),
                "candidate_model_token_count": len(cand_ids),
                "token_edit_distance": distance,
                "normalized_token_edit_distance": normalized_distance,
                "synthesized": cand["synthesized"],
                "rtf": cand["rtf"],
            }
        )

    report = {
        "schema_version": 1,
        "case_count": len(rows),
        "exact_model_input_cases": exact,
        "aggregate_token_edit_distance": total_distance,
        "aggregate_reference_tokens": total_reference,
        "aggregate_normalized_token_edit_distance": (
            total_distance / total_reference if total_reference else 0.0
        ),
        "synthesized_cases": synthesized,
        "max_synthesized_rtf": max_rtf if synthesized else None,
        "cold_load_ms": candidate["cold_load_ms"],
        "max_rss_mib_after_load": candidate["max_rss_mib_after_load"],
        "max_rss_mib_after_synthesis": candidate["max_rss_mib_after_synthesis"],
        "cases": rows,
    }
    output_path.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    print(
        "P0 compatibility summary: "
        f"exact={exact}/{len(rows)} "
        f"aggregate_edit={report['aggregate_normalized_token_edit_distance']:.4f} "
        f"max_rtf={max_rtf:.4f} "
        f"cold_load_ms={candidate['cold_load_ms']:.1f}"
    )
    for row in sorted(rows, key=lambda item: item["normalized_token_edit_distance"], reverse=True)[:10]:
        print(
            f"{row['id']}: edit={row['token_edit_distance']} "
            f"normalized={row['normalized_token_edit_distance']:.4f}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
