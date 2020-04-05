# https://atcoder.jp/contests/tenka1-2012-qualC/tasks/tenka1_2012_10
# 実装がめんどくさいだけでは？
# まずは手札を全部読み込んでから
# 条件を満たすidxとパターンを見つけて
# それ以外を捨てる

rr = range
enu = enumerate


def read_cards():
    s = input()
    l = 0
    ret = []
    marks = {'S', 'H', 'D', 'C'}
    for r, ss in enu(s[1:], start=1):
        if ss in marks:
            ret.append(s[l:r])
            l = r
    ret.append(s[l:])
    return ret


cards = read_cards()

paterns = [
    {'S10', 'SJ', 'SQ', 'SK', 'SA'},
    {'H10', 'HJ', 'HQ', 'HK', 'HA'},
    {'D10', 'DJ', 'DQ', 'DK', 'DA'},
    {'C10', 'CJ', 'CQ', 'CK', 'CA'},
]

conds = [0] * 4  # 5になったら終了

flg_break = False
pat_idx = -1
for e, c in enu(cards):
    for i in rr(4):
        if c in paterns[i]:
            conds[i] += 1
        if conds[i] == 5:
            flg_break = True
            pat_idx = i
    if flg_break:
        e += 1
        break

ans = ''
for c in cards[:e]:
    if c in paterns[pat_idx]:
        continue
    ans += c
print(ans if ans != '' else 0)
