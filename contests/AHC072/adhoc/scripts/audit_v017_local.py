#!/usr/bin/env python3
"""Inspect the saved macOS LOCAL binaries and logs; never execute a solver."""

import difflib
import hashlib
import json
import math
import re
import statistics
import struct
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
NAMES = ("v015_mixed_tree", "v017_timer_origin")
EXPECTED_BINARY_HASHES = (
    "0142ff389d091d2bb07062f38e66957ba392a06efeb2a531c101e16ae1884dc0",
    "dab00b31adfeb14b70edff784c778743ea3bc01ab841cad3b935842fbc4f15f3",
)
AUDIT = ROOT / "adhoc/v017_timer_audit"


def read_sections(data):
    """Read sections of the saved little-endian 64-bit Mach-O executable."""
    header = struct.unpack_from("<8I", data)
    assert header[0] == 0xFEEDFACF
    offset = 32
    sections = {}
    for _ in range(header[4]):
        command, size = struct.unpack_from("<II", data, offset)
        if command == 0x19:  # LC_SEGMENT_64
            count = struct.unpack_from("<I", data, offset + 64)[0]
            for i in range(count):
                fields = struct.unpack_from("<16s16sQQIIIIIIII", data, offset + 72 + 80 * i)
                name, segment = (s.rstrip(b"\0").decode() for s in fields[:2])
                address, length, file_offset = fields[2:5]
                sections[(segment, name)] = {
                    "address": address, "size": length, "offset": file_offset,
                    "data": data[file_offset:file_offset + length] if file_offset else b"",
                }
        offset += size
    return sections


def disassemble(path, section):
    # otool only reads the existing binary. macOS is required for this audit.
    args = ["otool", "-tvV"] if section == "__text" else ["otool", "-vV", "-s", "__TEXT", section]
    return subprocess.check_output([*args, str(path)], text=True).splitlines()


def main():
    paths = [ROOT / "target/release" / name for name in NAMES]
    binaries = [path.read_bytes() for path in paths]
    hashes = [hashlib.sha256(data).hexdigest() for data in binaries]
    assert tuple(hashes) == EXPECTED_BINARY_HASHES, "Saved binaries have changed"
    sections = [read_sections(data) for data in binaries]
    assert sections[0].keys() == sections[1].keys()
    section_report = []
    for key, left in sections[0].items():
        right = sections[1][key]
        assert all(left[field] == right[field] for field in ("address", "size", "offset"))
        equal = left["data"] == right["data"]
        section_report.append({
            "segment": key[0], "section": key[1], "size": left["size"],
            "bytes_identical": equal,
        })
        if equal:
            continue
        assert key in {("__TEXT", name) for name in ("__text", "__text_cold", "__cstring")}
        if key[1] == "__cstring":
            masked = []
            for name, section in zip(NAMES, (left, right)):
                value = bytearray(section["data"])
                filename = (name + ".cpp").encode()
                assert value.count(filename + b"\0") == 1
                at = value.index(filename + b"\0")
                value[at:at + len(filename)] = b"\0" * len(filename)
                masked.append(value)
            assert masked[0] == masked[1], "Unexpected non-filename string difference"

    assertions = []
    differences = []
    instruction_count = 0
    assertion_targets = set()
    sources = [(ROOT / "src/bin" / (name + ".cpp")).read_text().splitlines() for name in NAMES]
    for section in ("__text", "__text_cold"):
        listings = [disassemble(path, section) for path in paths]
        differences.extend(difflib.unified_diff(
            *listings, fromfile=NAMES[0] + ":" + section, tofile=NAMES[1] + ":" + section,
            lineterm="",
        ))
        instructions = []
        for listing in listings:
            parsed = {}
            for line in listing:
                match = re.match(r"^([0-9a-f]{16})\s+(.*)$", line)
                if match:
                    parsed[int(match[1], 16)] = match[2]
            instructions.append(parsed)
        assert instructions[0].keys() == instructions[1].keys()
        instruction_count += len(instructions[0])
        for address, left in instructions[0].items():
            right = instructions[1][address]
            if left == right:
                continue
            matches = [re.fullmatch(r"mov\s+w2, #0x([0-9a-f]+)", value) for value in (left, right)]
            assert all(matches), f"Unexpected instruction change at {address:x}"
            lines = [int(match[1], 16) for match in matches]
            assert all("assert(" in source[line - 1] for source, line in zip(sources, lines))
            assert sources[0][lines[0] - 1] == sources[1][lines[1] - 1]
            call = instructions[0][address + 4]
            target = re.match(r"bl\s+(0x[0-9a-f]+)", call)
            assert target
            if "___assert_rtn" in call:
                assertion_targets.add(target[1])
            assert target[1] in assertion_targets
            assertions.append({"address": hex(address), "v015_line": lines[0], "v017_line": lines[1]})
    assert len(assertions) == 13

    summaries = [json.loads((ROOT / "adhoc" / (name[:4] + "_evaluation_summary.json")).read_text()) for name in NAMES]
    cases = [{case["case_name"]: case for case in summary["cases"]} for summary in summaries]
    assert cases[0].keys() == cases[1].keys() and len(cases[0]) == 100
    counts = {}
    for key in ("baseline_ops", "initial_best_ops", "pre_branch_ops", "pre_local_ops", "final_ops",
                "mixed_completed", "mixed_decoded", "mixed_routes", "mixed_trials", "local_nodes"):
        values = [[case[name]["counts"][key] for name in sorted(cases[0])] for case in cases]
        counts[key] = {
            "v015": sum(values[0]), "v017": sum(values[1]),
            "delta": sum(values[1]) - sum(values[0]),
            "equal_cases": sum(a == b for a, b in zip(*values)),
        }
    deltas = [cases[1][name]["score"] - cases[0][name]["score"] for name in sorted(cases[0])]
    local_center = sum(counts["final_ops"][name] for name in ("v015", "v017")) / 2
    # One observed pair per case: E[(new-old)^2] = 2*variance, if the two
    # runs have equal means/variances and independent case-level noise.
    # Common load effects across cases violate the independence assumption.
    local_single_sd = math.sqrt(sum(delta * delta for delta in deltas) / 2)
    judge_scores = [12498, 12564, 12574]
    result = {
        "scope": "Saved macOS ARM64 LOCAL executables; solver execution count is zero",
        "binary_sha256": dict(zip(NAMES, hashes)),
        "binary_size": dict(zip(NAMES, map(len, binaries))),
        "run_ids": [summary["run_id"] for summary in summaries],
        "section_layouts_identical": True,
        "sections": section_report,
        "instruction_count": instruction_count,
        "instruction_differences": assertions,
        "only_instruction_differences_are_assertion_line_numbers": True,
        "only_string_difference_is_assertion_source_filename": True,
        "counts": counts,
        "rough_score_variation": {
            "atcoder": {
                "source": "User-provided screenshot: 21:34:09, 22:06:14, 22:57:52 on 2026-09-26",
                "scores": judge_scores,
                "observed_range": max(judge_scores) - min(judge_scores),
                "sample_standard_deviation": statistics.stdev(judge_scores),
                "sample_standard_deviation_percent": statistics.stdev(judge_scores) / statistics.mean(judge_scores) * 100,
                "limitation": "Three different sources; code effects and execution variation are not separated.",
            },
            "local": {
                "observed_sum_difference": sum(deltas),
                "sum_squared_case_differences": sum(delta * delta for delta in deltas),
                "case_difference_standard_deviation": statistics.stdev(deltas),
                "single_run_sum_standard_deviation_under_assumptions": local_single_sd,
                "single_run_average_standard_deviation_under_assumptions": local_single_sd / len(deltas),
                "single_run_standard_deviation_percent_under_assumptions": local_single_sd / local_center * 100,
                "two_run_difference_standard_deviation_under_assumptions": local_single_sd * math.sqrt(2),
                "limitation": "Only one pair of runs; case independence is assumed and common environment drift is unmeasured.",
            },
        },
        "limitations": [
            "Executable metadata (UUID, code signature, symbol names) is not execution equivalence evidence.",
            "Identical successful-path instructions do not fix clock readings, scheduling or allocator addresses.",
            "The saved logs do not identify the first differing clock reading or its external cause.",
            "These LOCAL binaries cannot establish the performance of the non-LOCAL timer change.",
        ],
    }
    AUDIT.mkdir(parents=True, exist_ok=True)
    (AUDIT / "local_binary_comparison.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    (AUDIT / "local_binary_instructions.diff").write_text("\n".join(differences) + "\n")
    print(json.dumps({"instructions": instruction_count, "assertion_line_differences": len(assertions),
                      "all_checks_passed": True, "counts": counts}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
