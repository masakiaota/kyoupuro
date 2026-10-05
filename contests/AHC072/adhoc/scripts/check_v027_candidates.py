#!/usr/bin/env python3
"""Reproduce candidate sampling using independent identity replay and RNG math."""

from collections import Counter, defaultdict
import json
from pathlib import Path

from analyze_v026_late_starts import MASK, replay, saved_moves, set_hash

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "adhoc/v027_audit"


class Rng:
    def __init__(self, state):
        self.state = state

    def next(self):
        self.state ^= (self.state << 7) & MASK
        self.state ^= self.state >> 9
        return self.state

    def choose(self, n):
        return self.next() * n >> 64


def main():
    total = Counter()
    selected_times = defaultdict(set)
    case_summaries = []
    current_case, completed_cases = None, set()
    for line in (OUT / "generated_cuts.jsonl").open():
        row = json.loads(line)
        if row["case"] != current_case:
            if current_case is not None:
                assert expected_pass == 16
                case_summaries.append({"case": current_case, **case_counts})
            current_case = row["case"]
            assert current_case not in completed_cases
            completed_cases.add(current_case)
            groups, flights = replay(current_case, saved_moves(current_case), collect=True)
            ordered = sorted(((r["time"], r["keep"], ids, r) for ids, times in groups.items() for r in times.values()))
            templates, parent_times = {}, {}
            for ids, times in groups.items():
                records = sorted(times.values(), key=lambda r: r["time"])
                parent_times[ids] = max(records[1:], key=lambda r: (r["split"], r["time"]))["time"] if len(records) > 1 else -1
                for item in records:
                    saved = sum(flight.issubset(ids) for flight in flights[item["time"]:])
                    templates[(ids, item["time"])] = (item, saved)
            expected_pass = 0
            case_counts = Counter(groups=len(groups), eligible_groups=sum(len(v) > 2 for v in groups.values()))
        assert row["pass"] == expected_pass
        if expected_pass:
            assert row["rng_before"] == previous_rng
        rng = Rng(row["rng_before"])
        seen, chosen = Counter(), {}
        expected = {}
        for time, keep, ids, item in ordered:
            seen[ids] += 1
            if seen[ids] == 1:
                expected[(ids, 0)] = time
            elif seen[ids] == 2 or rng.choose(seen[ids] - 1) == 0:
                chosen[ids] = time
        expected.update({(ids, 1): time for ids, time in chosen.items()})
        actual = {}
        for ids_list, time, keep, size, saved, code, later, split, alternative, middle in row["cuts"]:
            ids = tuple(ids_list)
            key = (ids, later)
            assert key not in actual
            item, expected_saved = templates[(ids, time)]
            assert (keep, size, saved, split) == (item["keep"], item["size"], expected_saved, item["split"])
            base_hash = int(set_hash(ids)) ^ 0xd1b54a32d192ed03
            expected_hash = base_hash ^ (((time + 1) * 0x94d049bb133111eb) & MASK) if later else base_hash
            assert code == expected_hash
            assert alternative == int(bool(later) and time != parent_times[ids])
            assert middle == int(bool(later) and time < max(groups[ids]))
            actual[key] = time
            if later:
                assert time > min(groups[ids]) and code != base_hash
                selected_times[(current_case, ids)].add(time)
                case_counts["alternative_selected"] += alternative
                case_counts["intermediate_selected"] += middle
        assert expected == actual, (current_case, expected_pass)
        assert len(groups) <= len(actual) <= 2 * len(groups)
        for _ in range(row["regular"] + len(row["cuts"])):
            rng.next()
        assert rng.state == row["rng_after"], (current_case, expected_pass, "RNG")
        previous_rng = rng.state
        case_counts["builds"] += 1
        case_counts["cuts"] += len(actual)
        expected_pass += 1
    assert expected_pass == 16
    case_summaries.append({"case": current_case, **case_counts})
    assert len(completed_cases) == 100
    for row in case_summaries:
        total.update({k: v for k, v in row.items() if k != "case"})
    changed_groups = sum(len(times) > 1 for times in selected_times.values())
    witness = [sorted(times) for (case, ids), times in selected_times.items()
               if case == "0005.txt" and set_hash(ids) == "17426116910026456433"]
    assert len(witness) == 1 and 105 in witness[0]
    assert changed_groups > 0 and total["alternative_selected"] > 0 and total["intermediate_selected"] > 0
    result = {"verified_cases": 100, "total": dict(total), "groups_with_changed_representative": changed_groups,
              "case0005_witness_times": witness[0], "cases": case_summaries}
    (OUT / "generation_check.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({k: v for k, v in result.items() if k != "cases"}, indent=2))


if __name__ == "__main__":
    main()
