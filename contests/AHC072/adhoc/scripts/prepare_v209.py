#!/usr/bin/env python3
"""添付版の時間設定を既登録v207へ揃える。solverは実行しない。"""
import argparse
import difflib
import json
import os
from pathlib import Path
import re
import subprocess
import sys

import build_v202_v203 as build
from build_v202_v203 import save, sha
from tune_v204 import evaluator

ROOT = Path(__file__).resolve().parents[2]
AUDIT = ROOT / "adhoc/v209_audit"
PARENT = "v207_lazy_candidate_bank"
CHILD = "v209_color_quotient_reinsertion"
ORIGINAL = "v207_color_quotient_reinsertion"
ATTACHMENT = Path("/tmp/codex-remote-attachments/01a0fda4-0e99-7b61-b806-9b4b8af1e65b/08FF8ED9-EEDC-47D8-9B9C-3F1B6C98E1DE/1-v207_color_quotient_reinsertion.cpp")
PARENT_RUN = "20261003T103444+0900_v207_lazy_candidate_bank_2a6d10"


def prepare():
    assert (ROOT / "notes/experiments/v209.md").exists()
    target=ROOT/f"src/bin/{CHILD}.cpp"
    assert not target.exists() and not AUDIT.exists()
    AUDIT.mkdir()
    original=AUDIT/f"{ORIGINAL}.cpp"
    original.write_bytes(ATTACHMENT.read_bytes())
    assert sha(original)=="5e67c951c4ea3abb067a399007fc2c0824e12bcdd777bc462bf9d778463fa4dc"
    source=original.read_text();changes=[]

    def replace(old,new):
        nonlocal source
        assert old in source
        at=source.index(old)
        changes.append(dict(old=old,new=new,offset=at))
        source=source[:at]+new+source[at+len(old):]

    header=source[:source.index("// 探索の実数演算")]
    replace(header,'''// v209_color_quotient_reinsertion.cpp
// v209: 同じ色順になる1個体の再挿入位置を、追加操作なしで同一視する。直接の親はv207。
// 添付版v207_color_quotient_reinsertion.cppを改番した。候補保持は既に含まれる。
// LOCALの時間は既登録v207へ揃え、非LOCALの計時と処理は添付版を保持する。
// 添付元SHA-256: 5e67c951c4ea3abb067a399007fc2c0824e12bcdd777bc462bf9d778463fa4dc
''')
    inherited=json.loads((ROOT/"adhoc/v207_audit/registered_changes.json").read_text())
    for change in inherited[1:]:replace(change["old"],change["new"])
    target.write_text(source)
    save(AUDIT/"port_changes.json",changes)
    (AUDIT/"port.diff").write_text("".join(difflib.unified_diff(original.read_text().splitlines(True),source.splitlines(True),fromfile=ORIGINAL,tofile=CHILD)))
    parent=(ROOT/f"src/bin/{PARENT}.cpp").read_text()
    old_lines=parent.splitlines(True);new_lines=source.splitlines(True)
    patch=[];current=parent;offset=0
    for tag,i,j,k,l in difflib.SequenceMatcher(None,old_lines,new_lines,autojunk=False).get_opcodes():
        if tag=="equal":continue
        before="".join(old_lines[i:j]);after="".join(new_lines[k:l]);at=len("".join(old_lines[:i]))+offset
        assert current[at:at+len(before)]==before
        patch.append(dict(old=before,new=after,offset=at))
        current=current[:at]+after+current[at+len(before):];offset+=len(after)-len(before)
    assert current==source
    save(AUDIT/f"{CHILD}_changes.json",patch)
    (AUDIT/"preregistration.md").write_bytes((ROOT/"notes/experiments/v209.md").read_bytes())
    print(f"Created {CHILD}; parent {PARENT}; no solver executed.",flush=True)


def compile_and_compare():
    args=argparse.Namespace(bin_name=CHILD,jobs=2,wait_lock=True)
    with evaluator.acquire_eval_lock(args,"v209_build"):
        build.AUDIT=AUDIT;build.PAIRS=[(PARENT,CHILD)];build.PARENT_RUNS={PARENT:PARENT_RUN}
        build.main()
        env=os.environ.copy()
        if sys.platform=="darwin":
            env.setdefault("SDKROOT",subprocess.check_output(["xcrun","--show-sdk-path"],text=True).strip())
            env.setdefault("MACOSX_DEPLOYMENT_TARGET","15.0")
        compiler=env.get("CXX","g++-15")
        original=AUDIT/f"{ORIGINAL}.cpp";child=ROOT/f"src/bin/{CHILD}.cpp"
        restored=child.read_text()
        for change in reversed(json.loads((AUDIT/"port_changes.json").read_text())):
            at=change["offset"];assert restored[at:at+len(change["new"])]==change["new"]
            restored=restored[:at]+change["old"]+restored[at+len(change["new"]):]
        assert restored==original.read_text()

        def normalize(s):
            for path in (original,child,ROOT/f"src/bin/{PARENT}.cpp"):
                s=s.replace(str(path),"solver.cpp").replace(f'"{path.name}"','"solver.cpp"')
            s=s.replace('"<stdin>"','"solver.cpp"')
            return re.sub(r'("solver.cpp",\s*)\d+',r'\g<1>0',s)

        checks={}
        for mode,flags in build.MODES.items():
            command=[compiler,*build.FLAGS,*flags,"-E","-P",str(original)]
            before=normalize(subprocess.check_output(command,env=env,text=True))
            after=normalize((AUDIT/f"{CHILD}_{mode}.ii").read_text())
            (AUDIT/f"original_{mode}.ii").write_text(before)
            (AUDIT/f"port_preprocessed_{mode}.diff").write_text("".join(difflib.unified_diff(before.splitlines(True),after.splitlines(True),fromfile=ORIGINAL,tofile=CHILD)))
            if mode=="production":
                assert before==after,"production differs from original"
                checks[mode]=dict(equal_after_diagnostic_normalization=True)
            else:
                pattern=r'\(PROGRAM_TIME_LIMIT_SEC \* \(\(([0-9.]+)\) / 1\.930\) \* \(1\.880 / 1\.900\)\)'
                expected,count=re.subn(pattern,r'(PROGRAM_TIME_LIMIT_SEC * ((\1) / JUDGE_TIME_LIMIT_SEC))',before)
                old='end=(PROGRAM_TIME_LIMIT_SEC * ((1.900) / JUDGE_TIME_LIMIT_SEC));'
                assert expected.count(old)==1
                expected=expected.replace(old,'end=PROGRAM_TIME_LIMIT_SEC;')
                assert expected==after,"unexpected LOCAL port difference"
                checks[mode]=dict(time_macro_expansions=count,lns_end_changes=1,expected_only=True)
        save(AUDIT/"port_preprocessed.json",checks)
        print("Original comparison: production identical; LOCAL only registered clocks changed.",flush=True)


if __name__=="__main__":
    {"prepare":prepare,"build":compile_and_compare}[sys.argv[1]]()
