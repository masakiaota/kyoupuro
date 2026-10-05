import csv,json,shutil
from pathlib import Path
from build_v136_speed import ROOT,RUN,SOURCE,BASE
from v089_data import save,sha

def main():
    full=json.loads((RUN/'result.json').read_text());mechanism=json.loads((RUN/'mechanism/result.json').read_text())
    first=json.loads((RUN/'mechanism/first_fixed_measurements.json').read_text())
    bench={}
    for mode in ['local','judge']:
        rows=[r for r in first if r['mode']==mode]
        a=sum(r['base_cpu'] for r in rows);b=sum(r['speed_cpu'] for r in rows)
        bench[mode]=dict(base_cpu_seconds=a,current_cpu_seconds=b,reduction_percent=100*(1-b/a),speedup=a/b)
    old=full['validation']['base'];new=full['validation']['speed'];a={r['case']:r for r in old['rows']};b={r['case']:r for r in new['rows']}
    def construction(d,k):return sum(r['construction'].get(k,0) for r in d['rows'])/len(d['rows'])
    def total(d,k):return sum(r['trace'].get(k,0) for r in d['rows'])
    old_nn=construction(old,'v311_nn_first_seconds')+construction(old,'v311_nn_extra_seconds')
    new_nn=construction(new,'v311_nn_first_seconds')+construction(new,'v311_nn_extra_seconds')
    phases=dict(base_nn_seconds=old_nn,current_nn_seconds=new_nn,nn_reduction_percent=100*(1-new_nn/old_nn),
                base_lns_attempts=total(old,'lns_attempts'),current_lns_attempts=total(new,'lns_attempts'),
                lns_attempts_change_percent=100*(total(new,'lns_attempts')/total(old,'lns_attempts')-1),
                first_nn_hash_equal=sum(a[k]['construction']['v311_nn_first_hash']==b[k]['construction']['v311_nn_first_hash'] for k in a))
    summary=dict(experiment='v136',parent='v113',source=str(SOURCE.relative_to(ROOT)),source_sha256=sha(SOURCE),parent_sha256=sha(BASE),
                 assessment=full['assessment'],fixed_work_first_measurement=bench,
                 repeated_during_diagnostic_failure=mechanism['speed'],fixed_clock_matches=mechanism['matched'],
                 background_replays_checked=mechanism['background_checked'],phase_observations=phases,
                 base=old['metrics'],current=new['metrics'],retained='v113',x86_execution='not_measured',tools_in='not_rerun')
    where=ROOT/'adhoc/v136';where.mkdir(exist_ok=True);save(where/'result_summary.json',summary)
    with (where/'comparison.csv').open('w') as f:
        w=csv.writer(f,lineterminator="\n");w.writerow(['case','v113_T','v136_T','difference','v113_lns_attempts','v136_lns_attempts','v113_ms','v136_ms'])
        for k in sorted(a):w.writerow([k,a[k]['T'],b[k]['T'],b[k]['T']-a[k]['T'],a[k]['trace']['lns_attempts'],b[k]['trace']['lns_attempts'],a[k]['elapsed_ms'],b[k]['elapsed_ms']])
    for name in ['result.json','first_fixed_measurements.json']:
        shutil.copy2(RUN/'mechanism'/name,where/('mechanism_'+name))
    save(RUN/'summary.json',summary);print(json.dumps(summary,ensure_ascii=False,indent=2))
if __name__=='__main__':main()
