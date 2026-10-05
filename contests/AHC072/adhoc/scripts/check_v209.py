#!/usr/bin/env python3
"""v209の固定、診断、100件評価。analyzeは保存結果のみを照合する。"""
import argparse
from collections import Counter
import csv
from datetime import datetime
import json
import os
from pathlib import Path
import re
import subprocess
import sys

from build_v202_v203 import save, sha
from check_v207 import BANK_KEYS, inspect as bank_inspect
from prepare_v209 import AUDIT, CHILD, PARENT, PARENT_RUN, ROOT
from tune_v204 import evaluator, mechanism as race_mechanism

OUT=ROOT/"results/analysis/v209"
LABEL="v209_color_quotient_same_local_budget"
ALIASES=("alias_closures","alias_gap_relaxations","alias_top_relaxations","alias_cost_relaxations")
CONTROLS={
    PARENT:dict(run=PARENT_RUN,output="results/out/v207_lazy_candidate_bank"),
    "v204_initial_race_tuned":dict(run="20261003T031601+0900_v204_initial_race_tuned_47557d",
        output="results/analysis/v206/validation200/previous_outputs/v204_initial_race_tuned"),
    "v208_mono_collection":dict(run="20261003T105536+0900_v208_mono_collection_46832e",
        output="results/analysis/v208/normal100/outputs"),
}


def records():
    runs=[c["run"] for c in CONTROLS.values()]
    return [json.loads(line) for line in (ROOT/"results/eval_records.jsonl").open()
            if CHILD in line or any(run in line for run in runs)]


def status(state,**extra):
    save(OUT/"status.json",dict(state=state,pid=os.getpid(),updated_at=datetime.now().astimezone().isoformat(),**extra))


def aggregate(details):
    counts,times,bank,aliases=Counter(),Counter(),Counter(),Counter()
    for row in details:
        counts.update(row["counts"]);times.update(row["times_ms"])
        bank.update(row.get("bank",{}));aliases.update(row.get("aliases",{}))
    return dict(cases=len(details),counts=dict(counts),average_times_ms={k:v/len(details) for k,v in times.items()},
                bank_totals={k:bank[k] for k in BANK_KEYS} if bank else {},aliases=dict(aliases),details=details)


def freeze():
    assert not (AUDIT/"frozen.json").exists()
    pre=json.loads((AUDIT/"preflight.json").read_text())
    for name,expected in pre["solver_sha256"].items():assert sha(ROOT/f"src/bin/{name}.cpp")==expected
    for b in pre["builds"]:
        assert not b["warnings"] and sha(AUDIT/f"{CHILD}_{b['mode']}")==b["binary_sha256"]
    checks=json.loads((AUDIT/"port_preprocessed.json").read_text())
    assert checks["production"]["equal_after_diagnostic_normalization"] and checks["local"]["expected_only"]
    all_records=records();assert not any(r["bin"]==CHILD for r in all_records)
    inputs=evaluator.list_input_files(ROOT/"tools/in");assert len(inputs)==100
    old_inputs=json.loads((ROOT/"results/tuning/v204/frozen.json").read_text())
    for p in inputs:assert sha(p)==old_inputs[str(p.relative_to(ROOT))]
    OUT.mkdir(parents=True,exist_ok=True)
    fixed=[*inputs,Path(__file__),ROOT/"adhoc/scripts/prepare_v209.py",ROOT/f"src/bin/{CHILD}.cpp"]
    fixed.extend(p for p in AUDIT.iterdir() if p.is_file())
    fixed.extend(ROOT/"adhoc/scripts"/name for name in
        ("check_v207.py","tune_v204.py","tune_v073.py","check_v037_results.py","check_v028_two_orders.py"))
    fixed.extend((ROOT/"scripts/eval.py",ROOT/"scripts/build_solver.sh"))
    controls={}
    for name,spec in CONTROLS.items():
        controls[name]=sorted((r for r in all_records if r["run_id"]==spec["run"]),key=lambda r:r["case_name"])
        rows=controls[name];assert len(rows)==100 and {r["case_name"] for r in rows}=={p.name for p in inputs}
        details=[]
        for r in rows:
            assert r["local"] and r["input_dir"]=="tools/in" and r["status"]=="ok"
            answer=ROOT/spec["output"]/r["case_name"]
            if name==PARENT:
                m=bank_inspect(r["case_name"],answer)
            else:
                c,t=race_mechanism(ROOT/"tools/in"/r["case_name"],answer,(5,15))
                m=dict(case=r["case_name"],counts=c,times_ms=t)
            assert m["counts"]["T"]==r["score"]
            details.append(m);fixed.extend((answer,answer.with_suffix(".txt.err")))
        metric=OUT/f"{name}_metrics.json";save(metric,aggregate(details))
        fixed.extend((metric,ROOT/f"src/bin/{name}.cpp"))
    save(AUDIT/"frozen.json",dict(sha256={str(p.relative_to(ROOT)):sha(p) for p in fixed},
         controls=controls,jobs=2,local_time_ratio=0.80,lns_end_ms=1520,search_limit_ms=1544,solver_executions=0))
    status("frozen");print("100入力、3版の保存結果、添付版と移植版、両ビルド、検証器を固定。solver実行0回。",flush=True)


def verify():
    frozen=json.loads((AUDIT/"frozen.json").read_text())
    for path,expected in frozen["sha256"].items():assert sha(ROOT/path)==expected,path
    return frozen


def inspect(case,answer):
    m=bank_inspect(case,answer)
    line=next(line for line in answer.with_suffix(".txt.err").read_text().splitlines() if line.startswith("baseline="))
    values=dict(re.findall(r"(\w+)=([^\s]+)",line))
    m["aliases"]={k:int(values[k]) for k in ALIASES}
    assert m["aliases"]["alias_cost_relaxations"]<=m["aliases"]["alias_gap_relaxations"]+m["aliases"]["alias_top_relaxations"]
    assert all(n>=0 for n in m["aliases"].values())
    return m


def active(metrics):
    return all(metrics["aliases"][key]>0 for key in ALIASES) and all(metrics["bank_totals"][key]>0 for key in BANK_KEYS)


def diagnostic():
    verify();assert not (AUDIT/"diagnostic_started.json").exists()
    args=argparse.Namespace(bin_name=CHILD,jobs=2,wait_lock=True)
    with evaluator.acquire_eval_lock(args,"v209_diagnostic"):
        save(AUDIT/"diagnostic_started.json",dict(cases=["0000.txt","0001.txt"],local=True));status("diagnostic")
        details=[]
        for case in ("0000.txt","0001.txt"):
            answer=AUDIT/f"diagnostic_{case}"
            with (ROOT/"tools/in"/case).open() as stdin,answer.open("w") as stdout,answer.with_suffix(".txt.err").open("w") as stderr:
                subprocess.run([str(AUDIT/f"{CHILD}_local")],stdin=stdin,stdout=stdout,stderr=stderr,check=True,timeout=15)
            details.append(inspect(case,answer));print(f"{case}: 合法、全帰巣、既存エラー0",flush=True)
        metrics=aggregate(details);assert active(metrics)
        save(AUDIT/"diagnostic.json",dict(passed=True,**metrics));status("diagnostic_passed")
        print(json.dumps(dict(aliases=metrics["aliases"],bank=metrics["bank_totals"]),ensure_ascii=False),flush=True)


def analyze():
    frozen=verify();assert json.loads((AUDIT/"diagnostic.json").read_text())["passed"]
    child=sorted((r for r in records() if r["bin"]==CHILD),key=lambda r:r["case_name"])
    assert len(child)==100 and len({r["run_id"] for r in child})==1
    details=[]
    for r in child:
        assert r["status"]=="ok" and r["local"] and r["input_dir"]=="tools/in" and r["label"]==LABEL
        m=inspect(r["case_name"],ROOT/r["stdout_path"]);assert m["counts"]["T"]==r["score"]
        details.append(m)
    metrics=aggregate(details);assert active(metrics)
    total=sum(r["score"] for r in child);elapsed=max(r["elapsed"] for r in child)
    comparisons={};cases=[];current={m["case"]:m for m in details}
    for name,rows in frozen["controls"].items():
        parent={r["case_name"]:r for r in rows};assert set(parent)==set(current)
        old=json.loads((OUT/f"{name}_metrics.json").read_text());pc={m["case"]:m for m in old["details"]}
        parent_total=sum(r["score"] for r in rows)
        delta=[r["score"]-parent[r["case_name"]]["score"] for r in child]
        comparisons[name]=dict(parent_total=parent_total,parent_average=parent_total/100,
            delta=total-parent_total,delta_percent=100*(total/parent_total-1),
            wins=sum(d<0 for d in delta),draws=sum(d==0 for d in delta),losses=sum(d>0 for d in delta),
            pre_lns_delta=sum(m["counts"]["pre_lns_ops"]-pc[m["case"]]["counts"]["pre_lns_ops"] for m in details),
            pre_pair_delta=sum(m["counts"]["pre_pair_ops"]-pc[m["case"]]["counts"]["pre_pair_ops"] for m in details),
            parent_attempts=old["counts"]["lns_attempts"],parent_average_times_ms=old["average_times_ms"])
    for r in child:
        m=current[r["case_name"]]
        row=dict(case=r["case_name"],T=r["score"],elapsed_ms=r["elapsed"],**m["aliases"])
        for name,rows in frozen["controls"].items():
            before=next(p["score"] for p in rows if p["case_name"]==r["case_name"])
            row[f"{name}_T"]=before;row[f"{name}_delta"]=r["score"]-before
        cases.append(row)
    result=dict(bin=CHILD,cases=100,run_id=child[0]["run_id"],total=total,average=total/100,
        average_elapsed_ms=sum(r["elapsed"] for r in child)/100,max_elapsed_ms=elapsed,
        comparisons=comparisons,aliases=metrics["aliases"],
        alias_active_cases={key:sum(m["aliases"][key]>0 for m in details) for key in ALIASES},
        lns_attempts=metrics["counts"]["lns_attempts"],average_times_ms=metrics["average_times_ms"],
        bank_totals=metrics["bank_totals"],all_outputs_verified=True,all_errors_zero=True,
        adopted=comparisons[PARENT]["delta"]<0 and elapsed<=2000)
    save(OUT/"comparison.json",result);save(OUT/"mechanism.json",metrics)
    with (OUT/"cases.csv").open("w",newline="") as f:
        writer=csv.DictWriter(f,fieldnames=list(cases[0]));writer.writeheader();writer.writerows(cases)
    brief={k:result[k] for k in ("cases","run_id","total","average","max_elapsed_ms","aliases","lns_attempts","adopted")}
    brief["comparisons"]={name:{k:r[k] for k in ("parent_total","delta","delta_percent","wins","draws","losses")} for name,r in comparisons.items()}
    print(json.dumps(brief,ensure_ascii=False,indent=2),flush=True)
    return result


def run():
    verify();assert json.loads((AUDIT/"diagnostic.json").read_text())["passed"]
    assert not (OUT/"evaluation_started.json").exists()
    args=argparse.Namespace(bin_name=CHILD,jobs=evaluator.default_jobs(),wait_lock=True,
        dry_run=False,verbose=False,label=LABEL,no_local=False)
    with evaluator.acquire_eval_lock(args,"tools/in"):
        save(OUT/"evaluation_started.json",dict(cases=100,label=LABEL));status("evaluating")
        assert evaluator.run_eval_locked(args,evaluator.list_input_files(ROOT/"tools/in"),"tools/in",True,True)==0
    result=analyze();status("completed",adopted=result["adopted"])


if __name__=="__main__":
    try:
        {"freeze":freeze,"diagnostic":diagnostic,"run":run,"analyze":analyze}[sys.argv[1]]()
    except BaseException as error:
        if OUT.exists():status("failed",error=f"{type(error).__name__}: {error}")
        raise
