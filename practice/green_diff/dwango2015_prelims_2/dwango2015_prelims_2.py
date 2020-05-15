# https://atcoder.jp/contests/dwango2015-prelims/tasks/dwango2015_prelims_2
# 25の部分を1文字に置換して連長圧縮
# 連長部分について(n+1)C2が答えかな


def run_length_encoding(s):
    '''
    連長圧縮を行う
    s ... iterable object e.g. list, str 
    return
    ----------
    s_composed,s_num,s_idx
    それぞれ、圧縮後の文字列、その文字数、その文字が始まるidx
    '''
    s_composed = []
    s_sum = []
    s_idx = [0]
    pre = s[0]
    cnt = 1
    for i, ss in enumerate(s[1:], start=1):
        if pre == ss:
            cnt += 1
        else:
            s_sum.append(cnt)
            s_composed.append(pre)
            s_idx.append(i)
            cnt = 1
            pre = ss
    s_sum.append(cnt)
    s_composed.append(pre)
    # assert len(s_sum) == len(s_composed)
    return s_composed, s_sum, s_idx


S = input()
S = S.replace('25', 'x')
S_comp, S_num, S_idx = run_length_encoding(S)
ans = 0
for s, n in zip(S_comp, S_num):
    if s == 'x':
        ans += (n + 1) * (n) // 2

print(ans)
# print(S_comp, S_num)
