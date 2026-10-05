#!/usr/bin/env python3
"""Freeze predefined saved-input diagnostics; this does not run a solver."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "adhoc/v044_joint_temporal"
OUT.mkdir(parents=True, exist_ok=True)

specifications = [
    ("0060", [0, *range(24, 34), 35],
     [(4, 11), (9, 15), (13, 11), (16, 12), (16, 13), (13, 15)], None),
    ("0005", [0, 223], [(12, 11), (16, 1), (16, 2), (16, 7), (12, 6), (12, 14)], "v026_late_start_lns"),
    ("0068", [0, 264], [(5, 19), (3, 19), (2, 18), (4, 19)], "v041_joint_window"),
    ("0097", [0, 568], [(12, 19), (13, 8), (17, 12), (15, 8), (19, 2), (8, 12)], "v041_joint_window"),
    ("0019", [0, 269], [(18, 4), (19, 3), (18, 2), (18, 3), (19, 5)], "v025_fast_math"),
]
control_origins = [(4, 7), (6, 7), (11, 2), (12, 11), (13, 8), (19, 2), (18, 3),
                   (17, 12), (10, 0), (6, 6), (14, 2), (6, 5), (10, 2), (7, 5)]
trials = [dict(tag="0097_control_0570", case="0097", boundary=570, control=1,
               origins=control_origins, reference="adhoc/v043_audit/plans/0097_570.txt")]
sources = {"notes/experiments/v044.md", "src/bin/v000_template.cpp",
           "adhoc/bin/v044_joint_temporal_probe.cpp", "adhoc/scripts/prepare_v044_joint_temporal.py",
           "adhoc/scripts/check_v044_joint_temporal.py"}
for case, boundaries, origins, reference in specifications:
    report = json.loads((ROOT / "adhoc/cooperative_plan_review_20260928" / f"{case}_v039.json").read_text())
    ids = {tuple(value["start"]): key for key, value in report["tokens"].items()}
    selected = {ids[point] for point in origins}
    sources.update([f"tools/in/{case}.txt", f"results/out/v039_exact_board_lns/{case}.txt"])
    for boundary in boundaries:
        alive = {token for tower in report["snapshots"][boundary].values() for token in tower}
        assert selected <= alive
        reference_path = f"results/out/{reference}/{case}.txt" if reference and boundary == 0 else "-"
        trials.append(dict(tag=f"{case}_{boundary:04d}", case=case, boundary=boundary,
                           control=0, origins=origins, reference=reference_path))
assert len(trials) == 21
for trial in trials:
    if trial["reference"] != "-":
        sources.add(trial["reference"])
lines = ["# tag case boundary control selected_count [initial_i initial_j] reference"]
for trial in trials:
    coordinates = " ".join(f"{i} {j}" for i, j in trial["origins"])
    lines.append(f"{trial['tag']} {trial['case']} {trial['boundary']} {trial['control']} "
                 f"{len(trial['origins'])} {coordinates} {trial['reference']}")
(OUT / "trials.txt").write_text("\n".join(lines) + "\n")
(OUT / "settings.json").write_text(json.dumps(dict(label_limit=1_000_000, trials=trials), indent=2) + "\n")
(OUT / "preregistration.md").write_bytes((ROOT / "notes/experiments/v044.md").read_bytes())
sources.update(["adhoc/v044_joint_temporal/trials.txt", "adhoc/v044_joint_temporal/settings.json",
                "adhoc/v044_joint_temporal/preregistration.md"])
hashes = {path: hashlib.sha256((ROOT / path).read_bytes()).hexdigest() for path in sorted(sources)}
(OUT / "sources_before.json").write_text(json.dumps(hashes, indent=2) + "\n")
print(f"Frozen {len(trials)} trials and {len(hashes)} source hashes; no solver invoked.")
