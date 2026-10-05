#!/usr/bin/env python3
"""Describe saved transformations without invoking any search."""
import json
from pathlib import Path
from check_v055_results import input_board, apply, verify_pass

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'adhoc/v055_audit'


def trajectory(case, moves, first, end):
    _,_,towers,nests = input_board(case)
    for move in moves[:first]:
        apply(towers,nests,move)
    result = []
    for at,move in enumerate(moves[first:end],first):
        i,j,k,d,length = move
        di,dj = {'U':(-1,0),'D':(1,0),'L':(0,-1),'R':(0,1)}[d]
        p,q = (i,j),(i+di*length,j+dj*length)
        word = lambda cell: ''.join(chr(97+c) for c in towers[cell])
        row = {'operation_index':at, 'operation':move, 'source':p, 'destination':q,
               'source_before':word(p), 'destination_before':word(q), 'moving':word(p)[k:]}
        apply(towers,nests,move)
        row.update(source_after=word(p), destination_after=word(q))
        result.append(row)
    return result


def main():
    passes = [json.loads(line) for line in (OUT / 'passes.jsonl').read_text().splitlines()]
    examples = []
    for case,kind in [('0014.txt','pair'),('0084.txt','joint')]:
        row = next(r for r in passes if r['case']==case and r['mode']=='combined' and r['kind']==kind)
        verify_pass(row)
        before = trajectory(case,row['before'],row['first'],row['old_end'])
        after = trajectory(case,row['after'],row['first'],row['new_end'])
        if case == '0014.txt':
            related_before = [m for m in before if m['source'] == (13,13)]
            related_after = [m for m in after if m['source'] == (13,13)]
            assert len(related_before)==2 and len(related_after)==1
            assert [r['moving'] for r in related_before] == ['eeee','ee']
            assert related_after[0]['moving'] == 'eeeeee'
            assert all(not ({m['source'],m['destination']} & {(13,13),(13,14)}) for m in before[1:-1])
            description = '有限区間の置換後に、e色6匹を4匹と2匹の2便で送る部分が残った。19個の独立操作を挟む2便を1便にまとめ、さらに1手短縮した。'
        else:
            related_before,related_after = before,after
            assert len(before)==3 and len(after)==2
            assert before[0]['source_before']=='c' and before[0]['destination_before']=='aai'
            assert after[0]['source_before']=='aai' and after[0]['destination_before']=='c'
            assert before[-1]['destination_after']==after[-1]['destination_after']=='aaaic'
            description = '有限区間の置換後に、cをaaiへ合流させて右回りに運ぶ3操作が残った。aaiをcへ運び、合流位置から下へ直接運ぶ2操作にした。最後の塔はどちらもaaaicとなる。'
        sequence = [{'pass':r['pass'],'kind':r['kind'],'saved':len(r['before'])-len(r['after'])}
                    for r in passes if r['case']==case and r['mode']=='combined']
        examples.append({'case':case,'description':description,'sequence':sequence,
                         'first':row['first'],'old_end':row['old_end'],'new_end':row['new_end'],
                         'before':related_before,'after':related_after,'boundary_verified':True})
    (OUT / 'reduction_examples.json').write_text(json.dumps(examples,ensure_ascii=False,indent=2)+'\n')
    print(json.dumps(examples,ensure_ascii=False,indent=2))


if __name__ == '__main__':
    main()
