import concurrent.futures
import difflib
import json
import re
import subprocess
from build_v136_speed import ROOT,RUN,BASE,SOURCE,build
from check_v089_board import compile_binary,trace
from check_v113_integrated import normalize,block
from run_v105_hybrid import preprocess
from run_v089_evaluation import validated_output
from v089_data import save,sha,now

DATA=ROOT/'results/nn_rank/v090/20261003_scaling_studio'

def check():
    build();where=RUN/'mechanism';where.mkdir(parents=True,exist_ok=True)
    marker=where/'result.json'
    if marker.exists():
        result=json.loads(marker.read_text());assert result['source_sha256']==sha(SOURCE);return result
    cases=[c for c in json.loads((DATA/'input_manifest.json').read_text()) if c['role']=='train'][:8]
    save(where/'input_manifest.json',cases)
    for label,source in [('base',BASE),('speed',SOURCE),('checked',SOURCE)]:
        name=f'check_v136_{label}'
        macro='#define V136_CHECK_BACKGROUND\n' if label=='checked' else ''
        wrapper=ROOT/'adhoc/bin'/(name+'.cpp')
        wrapper.write_text('// '+wrapper.name+'\n'+macro+'#define AUDIT_BOARD board_info\n#define AUDIT_POOL state_pool\n'+
                          '#define AUDIT_SOURCE "../../'+str(source.relative_to(ROOT))+'"\n#include "check_v071_fixed_clock.cpp"\n'+
                          ('struct BackgroundAuditReporter { ~BackgroundAuditReporter(){std::cerr<<"[background.audited] "<<Board::background_audited<<"\\n";} } background_reporter;\n' if label=='checked' else ''))
    def builds(label):
        d=where/label;d.mkdir(exist_ok=True)
        for local in ([True] if label=='checked' else [True,False]):
            mode='local' if local else 'judge'
            compile_binary('check_v136_'+label,d/('clock_'+mode),local)
            if label!='checked':
                source=BASE if label=='base' else SOURCE
                preprocess(source,d/(mode+'.ii'),local)
        return label
    with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:list(pool.map(builds,['base','speed','checked']))
    # The real submission is also compiled in both modes, not just the clock wrapper.
    for local in (True,False):compile_binary(SOURCE.stem,where/('solver_'+('local' if local else 'judge')),local)
    for mode in ['local','judge']:
        a=normalize((where/'base'/(mode+'.ii')).read_text());b=normalize((where/'speed'/(mode+'.ii')).read_text())
        (where/(mode+'.patch')).write_text(''.join(difflib.unified_diff(a.splitlines(True),b.splitlines(True))))
        for name in ['struct TimeKeeper','class TemporalLNS','struct NNSharedDeadline']:
            before=block(a,name);after=block(b,name)
            if name=='class TemporalLNS':
                before=before.replace('board_info.apply(b,m);h[p]=height(b[p]);h[q]=height(b[q]);',
                                      'board_info.apply_background(b,m,q,h[p],h[q]);')
            assert before==after,(mode,name)
    assert re.search(r'static constexpr char nn_packed\[\] =.*?;',BASE.read_text(),re.S).group()==re.search(r'static constexpr char nn_packed\[\] =.*?;',SOURCE.read_text(),re.S).group()
    rows=[]
    for mode in ['local','judge']:
        for i,case in enumerate(cases):
            records={}
            for label in (['base','speed'] if i%2==0 else ['speed','base']):
                d=where/label/f'{mode}_{case["index"]:04d}'
                if d.with_suffix('.out').exists() and d.with_suffix('.err').exists():
                    proc=subprocess.CompletedProcess([],0,d.with_suffix('.out').read_text(),d.with_suffix('.err').read_text())
                else:
                    proc=subprocess.run([where/label/('clock_'+mode)],input=(DATA/case['path']).read_text(),text=True,capture_output=True,timeout=120)
                    d.with_suffix('.out').write_text(proc.stdout);d.with_suffix('.err').write_text(proc.stderr)
                assert proc.returncode==0,(mode,label,case,proc.stderr[-2000:])
                audit=re.search(r'\[refactor.audit\] (.+)',proc.stderr).group(1)
                ticks,unit=map(int,re.search(r'\[refactor.cpu\] ticks=(\d+) per_second=(\d+)',proc.stderr).groups())
                counts=trace(proc.stderr)
                records[label]=dict(output=proc.stdout,counts=counts,audit=audit,cpu_seconds=ticks/unit)
            for key in ['output','counts','audit']:
                assert records['base'][key]==records['speed'][key],(mode,case['index'],key)
            result=validated_output(DATA/case['path'],records['speed']['output']);assert result['E']==0
            row=dict(mode=mode,case=case['index'],base_cpu=records['base']['cpu_seconds'],speed_cpu=records['speed']['cpu_seconds'],audit=records['base']['audit'],T=result['T'])
            rows.append(row);print(json.dumps(row),flush=True)
    case=cases[0]
    proc=subprocess.run([where/'checked/clock_local'],input=(DATA/case['path']).read_text(),text=True,capture_output=True,timeout=120)
    (where/'checked/replay.err').write_text(proc.stderr)
    assert proc.returncode==0,proc.stderr[-2000:]
    assert proc.stdout==(where/'speed'/f'local_{case["index"]:04d}.out').read_text()
    count=int(re.search(r'\[background.audited\] (\d+)',proc.stderr).group(1));assert count>0
    speed={}
    for mode in ['local','judge']:
        selected=[r for r in rows if r['mode']==mode]
        a=sum(r['base_cpu'] for r in selected);b=sum(r['speed_cpu'] for r in selected)
        speed[mode]=dict(base_seconds=a,speed_seconds=b,ratio=b/a,reduction_percent=100*(1-b/a))
    result=dict(source_sha256=sha(SOURCE),parent_sha256=sha(BASE),matched=16,background_checked=count,rows=rows,speed=speed,
                speed_passed=all(v['ratio']<=0.99 for v in speed.values()),passed=True,completed_at=now(),x86_execution='not_measured')
    save(marker,result);print(json.dumps(result,ensure_ascii=False),flush=True);return result
if __name__=='__main__':check()
