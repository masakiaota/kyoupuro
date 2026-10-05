#!/usr/bin/env python3
"""Judge the frozen capacity experiment and independently verify saved outputs."""
import csv
import hashlib
import json
from pathlib import Path

from summarize_v021 import latest_run, read_case
from summarize_v028 import group
from summarize_v010 import COUNT

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "adhoc/v033_capacity"
BIN = "v033_capacity_lns"
PARENT = "v028_two_order_lns"
LABEL = "capacity_all"


def main():
    audit = json.loads((OUT/"static_verification.json").read_text())
    for name, expected in audit["sources"].items():
        # 判定後のノートには結果を追記するため、事前登録部分は保存した原文で照合する。
        path=OUT/"preregistration.md" if name=="notes/experiments/v033.md" else ROOT/name
        assert hashlib.sha256(path.read_bytes()).hexdigest()==expected, name
    fixed = json.loads((OUT/"fixed_clock_summary.json").read_text())
    work = json.loads((OUT/"fixed_work_summary.json").read_text())
    assert fixed["verified_cases"]==100 and fixed["all_shared_counts_equal"] and fixed["rng_equal"]
    records = [json.loads(line) for line in (ROOT/"results/eval_records.jsonl").read_text().splitlines()]
    current = latest_run(records,BIN,LABEL)
    baseline = latest_run(records,PARENT,"two_order_lns")
    assert len({r["run_id"] for r in records if r["bin"]==BIN})==1
    assert baseline[0]["run_id"]=="20260927T184531+0900_v028_two_order_lns_c6311b"
    assert len(current)==100 and all(r["status"]=="ok" and r["local"] for r in current)
    cases = [read_case(r) for r in current]
    old = {r["case_name"]:read_case(r) for r in baseline}
    baseline_map = {PARENT:{r["case_name"]:r for r in baseline}}
    sizes = {int(r["floors"]):{k:int(v) for k,v in r.items()} for r in csv.DictReader((OUT/"capacity_sizes.csv").open())}
    for case in cases:
        raw=(ROOT/"tools/in"/case["case_name"]).read_text().splitlines()
        N=int(raw[0].split()[0]);floors=sum(ch!='#' for row in raw[1:N+1] for ch in row)
        c=case["counts"];expected=sizes[floors]
        assert c["floor_cells"]==floors and c["cell_capacity"]==expected["capacity"]
        assert c["board_bytes"]==expected["board_bytes"]==4*c["cell_capacity"]
        assert c["identity_board_bytes"]==expected["identity_bytes"]==18*c["cell_capacity"]
        assert c["floor_dist_bytes"]==expected["dist_bytes"]==2*c["cell_capacity"]**2
        assert c["geometry_bytes"]==expected["geometry_bytes"]+N*N*4
        for key in ("group_bytes","router_bytes","constructor_bytes","weight_bytes"):
            assert c[key]==expected[key]
        assert c["portion_router_bytes"]==expected["portion_bytes"]
        # LOCALには計測用boolがあり、非LOCALの表とは構造体の末尾位置が異なる。
        local_log=(OUT/"fixed_clock/child"/(case["case_name"]+".err")).read_text()
        local_sizes={key:int(value) for key,value in COUNT.findall(local_log)}
        assert c["temporal_lns_bytes"]==local_sizes["temporal_lns_bytes"]
    result = {"bin":BIN,"parent":PARENT,"run_id":current[0]["run_id"],"parent_run_id":baseline[0]["run_id"],
              "sources_unchanged":True,"capacity_verified_cases":len(cases),"fixed_clock_verified_cases":fixed["verified_cases"],
              "fixed_work":{k:v for k,v in work.items() if k!="cases"},"cases":cases}
    for name,predicate in (("all_100",lambda name:True),("generated_99",lambda name:name!="0000.txt"),
                           ("case0000",lambda name:name=="0000.txt")):
        selected=[c for c in cases if predicate(c["case_name"])]
        prior=[c for name,c in old.items() if predicate(name)]
        data=group(selected,baseline_map);before=group(prior,{})
        data["pre_lns_delta"]=sum(c["counts"]["pre_lns_ops"]-old[c["case_name"]]["counts"]["pre_lns_ops"] for c in selected)
        data["pre_lns_different_cases"]=sum(c["counts"]["pre_lns_ops"]!=old[c["case_name"]]["counts"]["pre_lns_ops"] for c in selected)
        data["change_percent"]={k:100*(data["counts_sum"][k]/before["counts_sum"][k]-1)
                                 for k in ("lns_attempts","single_insert_calls","packet_insert_calls","event_layers")}
        data["change_percent"]["loops_including_dependency_skips"]=100*(data["loops_including_dependency_skips"]/before["loops_including_dependency_skips"]-1)
        result[name]=data
    all_cases=result["all_100"]
    errors=("baseline_recovery","final_recovery","construction_errors","lns_errors","lns_invalid_candidates")
    result["required_passed"]=(all_cases["verified_cases"]==all_cases["cases_under_2000_ms"]==100
        and all(all_cases["counts_sum"].get(k,0)==0 for k in errors))
    result["speed_improved"]=work["generated_99"]["total_ns"]["speed_improved"]
    result["adopt_submission_candidate"]=(result["required_passed"] and all_cases["comparisons"][PARENT]["delta_T"]<0
                                          and result["case0000"]["total_T"]<=43)
    (OUT/"evaluation_summary.json").write_text(json.dumps(result,ensure_ascii=False,indent=2)+"\n")
    with (OUT/"case_comparison.csv").open("w",newline="") as f:
        writer=csv.writer(f);writer.writerow(("case","floors","capacity","v028_T","v033_T","delta_T","pre_lns_delta","attempts_delta","elapsed_ms"))
        for c in cases:
            p=old[c["case_name"]];count=c["counts"]
            writer.writerow((c["case_name"],count["floor_cells"],count["cell_capacity"],p["score"],c["score"],c["score"]-p["score"],
                count["pre_lns_ops"]-p["counts"]["pre_lns_ops"],count["lns_attempts"]-p["counts"]["lns_attempts"],c["elapsed"]))
    concise={k:result[k] for k in ("run_id","required_passed","speed_improved","adopt_submission_candidate")}
    concise.update(comparison=all_cases["comparisons"][PARENT],max_elapsed_ms=all_cases["max_elapsed_ms"],
                   case0000=result["case0000"]["total_T"],pre_lns_delta=all_cases["pre_lns_delta"],
                   work_change=work["generated_99"]["total_ns"]["change_percent"],
                   loops_change=result["generated_99"]["change_percent"])
    print(json.dumps(concise,ensure_ascii=False,indent=2))


if __name__=="__main__":
    main()
