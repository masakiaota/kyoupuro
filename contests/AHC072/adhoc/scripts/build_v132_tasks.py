#!/usr/bin/env python3
"""登録した節目遷移をv044の固定診断へ組み込む。"""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
RUN = ROOT / 'results/nn_rank/v132/20261005_temporal_tasks_studio'
OUT = RUN / 'diagnostic'
SOURCE = ROOT / 'adhoc/bin/v132_temporal_tasks.cpp'


def build():
    text = (ROOT / 'adhoc/bin/v044_joint_temporal_probe.cpp').read_text()
    begin = text.index('struct SearchResult {')
    end = text.index('struct Trial {', begin)
    text = text[:begin] + (ROOT / 'adhoc/scripts/v132_temporal_tasks.cpp.txt').read_text() + '\n' + text[end:]
    text = text.replace('v044_joint_temporal_probe', 'v132_temporal_tasks')
    text = text.replace('constexpr size_t label_limit = 1000000;', 'constexpr size_t label_limit = 100000;')
    text = text.replace('adhoc/v044_joint_temporal', str(OUT.relative_to(ROOT)))
    text = text.replace('results/analysis/v044_joint_temporal.csv', str((RUN / 'trials.csv').relative_to(ROOT)))
    old_fixture = '        require(result.best==expected && result.status!="label_cap","fixture shortest result mismatch");'
    assert old_fixture in text
    text = text.replace(old_fixture, '        require(task_admission(planner,witness)>0,"fixture witness outside task graph");\n'
        '        require(result.best>=expected && result.best<=int(problem.original.size()) && result.status!="label_cap" && result.status!="expansion_cap","fixture completion mismatch");')
    text = text.replace('    fixtures(fixture_output,admissions,checkpoints);',
        '    ofstream task_admissions(out/"task_admissions.csv");\n'
        '    task_admissions<<"tag,primitive_steps,task_steps\\n";\n'
        '    fixtures(fixture_output,admissions,checkpoints);')
    needle = '            if (trial.control) require(tested.accepted && reference.size()==9,"known nine-move trace not represented");'
    assert needle in text
    text = text.replace(needle, needle + '''
            if(trial.control) {
                int count=task_admission(planner,reference);
                task_admissions<<trial.tag<<','<<reference.size()<<','<<count<<'\\n'<<flush;
                require(count>0,"known trace outside task graph");
            }''')
    text = text.replace('north_a_launches,artifact\\n',
        'north_a_launches,artifact,task_routes,multi_edges,task_legs,background_edges,maximum_task_length\\n')
    needle = "<<plan.background_moves<<','<<plan.rides<<','<<plan.supports<<','<<plan.skipped<<','<<north<<','<<artifact<<'\\n'<<flush;"
    assert needle in text
    text = text.replace(needle,
        "<<plan.background_moves<<','<<plan.rides<<','<<plan.supports<<','<<plan.skipped<<','<<north<<','<<artifact<<','"
        "<<result.routes<<','<<result.multi_edges<<','<<result.legs<<','<<result.background_edges<<','<<result.maximum_length<<'\\n'<<flush;")
    SOURCE.write_text(text)
    return SOURCE


if __name__ == '__main__':
    print(build())
