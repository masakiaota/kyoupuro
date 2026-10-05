#!/usr/bin/env python3
"""Run frozen v039 checks and measurements once; never edit solver sources."""
import fcntl
import json
import re
import subprocess
import sys
from prepare_v039 import ROOT,OUT,NAMES,digest

COUNTS=re.compile(r'\[summary.count\] ([^=]+)=(-?\d+)')
ADDED={'floor_cells','board_payload_bytes','board_handle_bytes','board_pool_slots','board_pool_bytes','board_pool_free_at_end'}


def unchanged():
    for name,want in json.loads((OUT/'frozen.json').read_text())['hashes'].items():
        if digest(ROOT/name)!=want:raise RuntimeError('Changed frozen file: '+name)


def inspect():
    results={}
    for name in ('parent_production','child_production','bench'):
        binary=OUT/name
        raw=subprocess.check_output(['xcrun','llvm-nm','--numeric-sort','--demangle',str(binary)],text=True)
        (OUT/(name+'_symbols.txt')).write_text(raw)
        symbols={}
        for line in raw.splitlines():
            match=re.match(r'([\da-fA-F]+)\s+[Tt]\s+(.*)',line)
            if match:symbols[int(match[1],16)]=match[2]
        selected={}
        for address,title in symbols.items():
            if 'TemporalLNS::insertAt(' not in title or 'lambda' in title:continue
            stop=min(a for a in symbols if a>address)
            assembly=subprocess.check_output(['xcrun','llvm-objdump','--disassemble','--no-show-raw-insn',f'--start-address={address}',f'--stop-address={stop}',str(binary)],text=True)
            label=('parent' if title.startswith('parent::') else 'child') if name=='bench' else name
            (OUT/(label+'_insert_assembly.txt' if name!='bench' else label+'_bench_insert_assembly.txt')).write_text(assembly)
            targets=[]
            for line in assembly.splitlines():
                match=re.search(r'\b(?:bl|b)\s+0x([0-9a-fA-F]+)',line)
                if match:targets.append(symbols.get(int(match[1],16),''))
            selected[title]={'bytes_to_next_symbol':stop-address,
                'clock_check_transfers':sum('Clock::check()' in x for x in targets),
                'mono_color_transfers':sum('monoColor(' in x for x in targets),
                'calls':sum(bool(re.search(r'\bbl\s',x)) for x in assembly.splitlines())}
        assert len(selected)==(2 if name=='bench' else 1),(name,selected)
        results[name]=selected
    (OUT/'assembly_summary.json').write_text(json.dumps(results,indent=2)+'\n')
    print(json.dumps(results,indent=2))


def parse(stdout,stderr):
    text=stderr.decode()
    counts={k:int(v) for k,v in COUNTS.findall(text) if k not in ADDED}
    return (stdout,counts,int(re.search(r'\[fixed.rng\] (\d+)',text)[1]),int(re.search(r'\[fixed.ticks\] (\d+)',text)[1]))


def fixed():
    destination=OUT/'fixed_clock';destination.mkdir()
    results=[]
    for i,path in enumerate(sorted((OUT/'fixture/tools/in').glob('*.txt'))):
        records=[]
        for label in NAMES:
            run=subprocess.run([str(OUT/(label+'_fixed'))],input=path.read_bytes(),capture_output=True,timeout=60)
            target=destination/label;target.mkdir(exist_ok=True)
            (target/path.name).write_bytes(run.stdout);(target/(path.name+'.err')).write_bytes(run.stderr)
            if run.returncode:raise RuntimeError(f'{label} failed {path.name}: {run.stderr[-1800:]!r}')
            records.append(parse(run.stdout,run.stderr))
            if label=='child':
                peak=int(re.search(rb'\[pool.peak\] (\d+)',run.stderr)[1])
                takes=int(re.search(rb'\[pool.takes\] (\d+)',run.stderr)[1])
        if records[0]!=records[1]:
            keys=('output','counts','rng','ticks');wrong=[k for k,a,b in zip(keys,records[0],records[1]) if a!=b]
            (OUT/'fixed_failure.json').write_text(json.dumps({'case':path.name,'different':wrong,'parent_counts':records[0][1],'child_counts':records[1][1]},indent=2)+'\n')
            raise RuntimeError('Fixed-clock mismatch '+path.name+' '+str(wrong))
        results.append({'case':path.name,'T':records[1][1]['T'],'ticks':records[1][3],'pool_peak':peak,'pool_takes':takes,'common_keys':len(records[1][1])})
        if (i+1)%10==0:print('fixed clock matched',i+1,flush=True)
    (OUT/'fixed_clock_summary.json').write_text(json.dumps({'matched':len(results),'pool_peak':max(r['pool_peak'] for r in results),'cases':results},indent=2)+'\n')


def stream(args,target):
    assert not target.exists(),'Already executed: '+str(target)
    with target.open('w') as f:
        p=subprocess.Popen(args,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,cwd=ROOT)
        for line in p.stdout:print(line,end='',flush=True);f.write(line);f.flush()
        if p.wait():raise RuntimeError('Failed; see '+str(target))


if __name__=='__main__':
    if sys.argv[1]=='inspect':inspect();sys.exit(0)
    unchanged()
    print('Waiting for evaluation lock',flush=True)
    with (ROOT/'results/.eval.lock').open('a+') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX)
        if sys.argv[1]=='probe':stream([str(OUT/'probe'),str(OUT/'fixture')],OUT/'probe.log')
        elif sys.argv[1]=='fixed':fixed()
        elif sys.argv[1]=='bench':stream([str(OUT/'bench'),str(OUT/'fixture'),str(OUT)],OUT/'bench.log')
        else:raise ValueError(sys.argv[1])
    unchanged();print('Frozen sources and inputs unchanged',flush=True)
