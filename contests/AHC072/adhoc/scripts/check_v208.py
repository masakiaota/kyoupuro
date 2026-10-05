#!/usr/bin/env python3
"""v208の固定、診断、登録済み300件の評価。analyzeは保存結果だけを読む。"""
import argparse
from collections import Counter
import csv
from datetime import datetime
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

from build_v202_v203 import save, sha
from check_v037_results import verify_output
from prepare_v208 import AUDIT, CHILD, PARENT, ROOT
from tune_v204 import evaluator, mechanism as race_mechanism

OUT = ROOT / "results/analysis/v208"
SETS = {
    "normal100": dict(input="tools/in", count=100,
        parent_run="20261003T031601+0900_v204_initial_race_tuned_47557d",
        parent_output="results/analysis/v206/validation200/previous_outputs/v204_initial_race_tuned"),
    "validation200": dict(input="results/analysis/v206/validation200/input", count=200,
        parent_run="20261003T100627+0900_v204_initial_race_tuned_bf911d",
        parent_output="results/analysis/v206/validation200/outputs/v204_initial_race_tuned"),
}


def records():
    runs = [s["parent_run"] for s in SETS.values()]
    return [json.loads(line) for line in (ROOT / "results/eval_records.jsonl").open()
            if CHILD in line or any(run in line for run in runs)]


def aggregate(details):
    counts, times = Counter(), Counter()
    for m in details:
        counts.update(m["counts"]); times.update(m["times_ms"])
    return dict(cases=len(details), counts=dict(counts),
                average_times_ms={k:v/len(details) for k,v in times.items()}, details=details)


def status(state, **extra):
    save(OUT / "status.json", dict(state=state, pid=os.getpid(),
         updated_at=datetime.now().astimezone().isoformat(), **extra))


def freeze():
    assert not (AUDIT / "frozen.json").exists()
    assert not (AUDIT / "diagnostic_started.json").exists()
    OUT.mkdir(parents=True, exist_ok=True)
    before=json.loads((AUDIT / "preflight.json").read_text())
    for name, expected in before["solver_sha256"].items():
        assert sha(ROOT / f"src/bin/{name}.cpp")==expected
    for build in before["builds"]:
        assert not build["warnings"]
        assert sha(AUDIT / f"{CHILD}_{build['mode']}")==build["binary_sha256"]
    assert not (AUDIT / "probe_build.log").read_text()
    old=records();assert not any(r["bin"]==CHILD for r in old)
    saved_metrics={
        "normal100":json.loads((ROOT / "results/analysis/v206/parent_metrics.json").read_text()),
        "validation200":json.loads((ROOT / "results/analysis/v206/validation200/mechanism.json").read_text())[PARENT],
    }
    (AUDIT / "preregistration.md").write_bytes((ROOT / "notes/experiments/v208.md").read_bytes())
    fixed=[ROOT/f"src/bin/{name}.cpp" for name in (PARENT,CHILD)]
    fixed.extend(p for p in AUDIT.iterdir() if p.is_file())
    fixed.extend([Path(__file__),ROOT/"adhoc/scripts/prepare_v208.py",ROOT/"adhoc/bin/check_v208_mono_collection.cpp",
                  ROOT/"scripts/eval.py",ROOT/"scripts/build_solver.sh"])
    fixed.extend(ROOT/"adhoc/scripts"/name for name in
                 ("tune_v204.py","tune_v073.py","check_v037_results.py","check_v028_two_orders.py"))
    old_input_hashes=json.loads((ROOT / "results/tuning/v204/frozen.json").read_text())
    validation_hashes=json.loads((ROOT / "results/analysis/v206/validation200/frozen.json").read_text())["sha256"]
    parents={}
    for name,spec in SETS.items():
        inputs=evaluator.list_input_files(ROOT/spec["input"])
        parents[name]=sorted((r for r in old if r["run_id"]==spec["parent_run"]),key=lambda r:r["case_name"])
        assert len(inputs)==len(parents[name])==spec["count"]
        assert {p.name for p in inputs}=={r["case_name"] for r in parents[name]}
        original=old_input_hashes if name=="normal100" else validation_hashes
        for p in inputs:assert sha(p)==original[str(p.relative_to(ROOT))]
        fixed.extend(inputs)
        details=[];metrics={m["case"]:m for m in saved_metrics[name]["details"]}
        for r in parents[name]:
            assert r["status"]=="ok" and r["local"] and r["input_dir"]==spec["input"]
            answer=ROOT/spec["parent_output"]/r["case_name"]
            c,t=race_mechanism(ROOT/spec["input"]/r["case_name"],answer,(5,15))
            assert c==metrics[r["case_name"]]["counts"] and t==metrics[r["case_name"]]["times_ms"]
            assert r["score"]==c["T"]
            fixed.extend((answer,answer.with_suffix(".txt.err")))
            details.append(dict(case=r["case_name"],counts=c,times_ms=t))
        target=OUT/f"{name}_parent_metrics.json";save(target,aggregate(details));fixed.append(target)
    save(AUDIT/"frozen.json",dict(sha256={str(p.relative_to(ROOT)):sha(p) for p in fixed},
         parent_records=parents,sets=SETS,jobs=2,local_time_ratio=0.80,solver_executions=0))
    status("frozen")
    print("両ソース・両ビルド・300入力・親の保存300出力・検証器を固定。solver実行0回。",flush=True)


def verify():
    frozen=json.loads((AUDIT/"frozen.json").read_text())
    for path,expected in frozen["sha256"].items():assert sha(ROOT/path)==expected,path
    return frozen


def mechanism(case, answer):
    c,t=race_mechanism(case,answer,(5,15))
    prefix="mono_collection_"
    assert c[prefix+"planned"]>=c[prefix+"feasible"]>=c[prefix+"positive"]>=c[prefix+"adopted"]>=c[prefix+"completed"]
    adopted=c[prefix+"adopted"]
    assert 3*adopted<=c[prefix+"adopted_sources"]<=c[prefix+"adopted_pieces"]<=8*adopted
    assert c[prefix+"completed"]<=c["construction_completed"]
    assert t["search_limit"]==1544.0
    plans=[c[f"race_seed_{i}_collection_plans"] for i in range(c["race_seed_count"])]
    assert all(p>=0 for p in plans) and sum(plans)<=adopted
    for i,p in enumerate(plans):
        if c[f"race_seed_{i}_origin"]<0:assert p==0
    return dict(case=case.name,counts=c,times_ms=t,
                retained_collection_seeds=sum(p>0 for p in plans),
                collection_winner=plans[c["race_winner_initial_rank"]]>0)


def diagnostic():
    verify();marker=AUDIT/"diagnostic_started.json"
    assert not (AUDIT/"diagnostic.json").exists()
    args=argparse.Namespace(bin_name=CHILD,jobs=2,wait_lock=True)
    with evaluator.acquire_eval_lock(args,"v208_diagnostic"):
        plan=dict(cases=["0000.txt","0001.txt"],modes=["local","production"],fixtures=8)
        if marker.exists():assert json.loads(marker.read_text())==plan
        else:save(marker,plan)
        status("diagnostic")
        if not (AUDIT/"probe.json").exists():
            with (AUDIT/"probe.json").open("w") as stdout,(AUDIT/"probe.err").open("w") as stderr:
                subprocess.run([str(AUDIT/"check_v208_mono_collection")],stdout=stdout,stderr=stderr,check=True,timeout=30)
        assert not (AUDIT/"probe.err").read_text()
        assert json.loads((AUDIT/"probe.json").read_text())["fixtures"]==8
        rows=[]
        for mode in ("local","production"):
            for case in ("0000.txt","0001.txt"):
                input_path=ROOT/"tools/in"/case;answer=AUDIT/f"{mode}_{case}"
                if not answer.exists():
                    with input_path.open() as stdin,answer.open("w") as stdout,answer.with_suffix(".txt.err").open("w") as stderr:
                        subprocess.run([str(AUDIT/f"{CHILD}_{mode}")],stdin=stdin,stdout=stdout,stderr=stderr,check=True,timeout=15)
                row=dict(mode=mode,case=case,T=verify_output(str(input_path),answer))
                if mode=="local":row.update(mechanism(input_path,answer))
                else:assert "diagnostic:" not in answer.with_suffix(".txt.err").read_text()
                rows.append(row);print(f"{mode} {case}: 合法・全帰巣",flush=True)
        local=[r for r in rows if r["mode"]=="local"]
        for key in ("proposed","feasible","adopted","completed"):
            assert sum(r["counts"]["mono_collection_"+key] for r in local)>0,key
        assert sum(r["retained_collection_seeds"] for r in local)>0
        save(AUDIT/"diagnostic.json",rows);status("diagnostic_passed")
        print("8境界条件・両モード2件の診断と新機構の発動を確認。",flush=True)


def analyze_set(name, frozen):
    spec=SETS[name];directory=OUT/name
    selected=sorted((r for r in records() if r["bin"]==CHILD and r["input_dir"]==spec["input"]),key=lambda r:r["case_name"])
    assert len(selected)==spec["count"] and len({r["run_id"] for r in selected})==1
    parent={r["case_name"]:r for r in frozen["parent_records"][name]}
    assert {r["case_name"] for r in selected}==set(parent)
    previous=json.loads((OUT/f"{name}_parent_metrics.json").read_text())
    pc={r["case"]:r["counts"] for r in previous["details"]}
    manifest=json.loads((directory/"outputs_sha256.json").read_text())
    for path,expected in manifest.items():assert sha(ROOT/path)==expected
    rows=[];metrics=[]
    for r in selected:
        assert r["status"]=="ok" and r["local"] and r["label"]==f"v208_mono_collection_{name}"
        m=mechanism(ROOT/spec["input"]/r["case_name"],directory/"outputs"/r["case_name"])
        c=m["counts"];assert c["T"]==r["score"]
        p=parent[r["case_name"]]
        rows.append(dict(case=r["case_name"],parent_T=p["score"],T=r["score"],delta_T=r["score"]-p["score"],
            elapsed_ms=r["elapsed"],pre_lns_delta_T=c["pre_lns_ops"]-pc[r["case_name"]]["pre_lns_ops"],
            pre_pair_delta_T=c["pre_pair_ops"]-pc[r["case_name"]]["pre_pair_ops"],
            retained_collection_seeds=m["retained_collection_seeds"],collection_winner=m["collection_winner"]))
        metrics.append(m)
    merged=aggregate(metrics);counts=merged["counts"];times=merged["average_times_ms"];ptimes=previous["average_times_ms"]
    for key in ("proposed","feasible","adopted","completed"):assert counts["mono_collection_"+key]>0
    assert sum(r["retained_collection_seeds"] for r in rows)>0
    total=sum(r["T"] for r in rows);parent_total=sum(r["parent_T"] for r in rows);n=len(rows)
    result=dict(cases=n,run_id=selected[0]["run_id"],parent_run_id=spec["parent_run"],
        total=total,parent_total=parent_total,average=total/n,parent_average=parent_total/n,
        delta=total-parent_total,delta_percent=100*(total/parent_total-1),
        wins=sum(r["delta_T"]<0 for r in rows),draws=sum(r["delta_T"]==0 for r in rows),losses=sum(r["delta_T"]>0 for r in rows),
        max_elapsed_ms=max(r["elapsed_ms"] for r in rows),average_elapsed_ms=sum(r["elapsed_ms"] for r in rows)/n,
        pre_lns_delta=sum(r["pre_lns_delta_T"] for r in rows),pre_pair_delta=sum(r["pre_pair_delta_T"] for r in rows),
        adopted_collection_cases=sum(m["counts"]["mono_collection_adopted"]>0 for m in metrics),
        retained_collection_cases=sum(r["retained_collection_seeds"]>0 for r in rows),
        retained_collection_seeds=sum(r["retained_collection_seeds"] for r in rows),
        collection_winner_cases=sum(r["collection_winner"] for r in rows),
        adopted_collections=counts["mono_collection_adopted"],
        average_collection_cost_ms=times["mono_collection_generation"]+times["mono_collection_planning"],
        average_construction_ms=times["construction"],parent_average_construction_ms=ptimes["construction"],
        lns_attempts=counts["lns_attempts"],parent_lns_attempts=previous["counts"]["lns_attempts"],
        average_lns_ms=times["temporal_lns"],parent_average_lns_ms=ptimes["temporal_lns"],
        all_outputs_verified=True,all_errors_zero=True,
        passed=total<parent_total and max(r["elapsed_ms"] for r in rows)<=2000)
    save(directory/"comparison.json",result);save(directory/"mechanism.json",merged)
    with (directory/"cases.csv").open("w",newline="") as f:
        writer=csv.DictWriter(f,fieldnames=list(rows[0]));writer.writeheader();writer.writerows(rows)
    print(json.dumps(result,ensure_ascii=False,indent=2),flush=True)
    return result


def analyze():
    frozen=verify();assert (AUDIT/"diagnostic.json").exists()
    results={name:analyze_set(name,frozen) for name in SETS}
    result=dict(sets=results,adopted=all(r["passed"] for r in results.values()),solver_changed_after_execution=False)
    save(OUT/"comparison.json",result)
    return result


def run():
    verify();assert (AUDIT/"diagnostic.json").exists();assert not (OUT/"evaluation_started.json").exists()
    args=argparse.Namespace(bin_name=CHILD,jobs=evaluator.default_jobs(),wait_lock=True,
        dry_run=False,verbose=False,label="",no_local=False)
    with evaluator.acquire_eval_lock(args,"v208_registered_300"):
        save(OUT/"evaluation_started.json",dict(sets=list(SETS)))
        for name,spec in SETS.items():
            verify();status("evaluating",stage=name)
            args.label=f"v208_mono_collection_{name}"
            inputs=evaluator.list_input_files(ROOT/spec["input"])
            code=evaluator.run_eval_locked(args,inputs,spec["input"],name=="normal100",True)
            assert code==0,f"{name}: eval failed ({code})"
            directory=OUT/name;directory.mkdir()
            shutil.copytree(ROOT/"results/out"/CHILD,directory/"outputs")
            save(directory/"outputs_sha256.json",{str(p.relative_to(ROOT)):sha(p) for p in (directory/"outputs").rglob("*") if p.is_file()})
            analyze_set(name,verify())
    result=analyze();status("completed",adopted=result["adopted"])


if __name__=="__main__":
    try:
        {"freeze":freeze,"diagnostic":diagnostic,"run":run,"analyze":analyze}[sys.argv[1]]()
    except BaseException as error:
        if OUT.exists():status("failed",error=f"{type(error).__name__}: {error}")
        raise
