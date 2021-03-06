# https://atcoder.jp/contests/abc019/tasks/abc019_2
def run_length_encoding(s):
    '''連長圧縮を行う
    s ... iterable object e.g. list, str 
    return
    ----------
    s_composed,s_num,s_idx
    それぞれ、圧縮後の文字列、その文字数、その文字が始まるidx'''
    s_composed, s_sum = [], []
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


s_comp, s_sum, _ = run_length_encoding(input())
print(*[s + str(n) for s, n in zip(s_comp, s_sum)], sep='')
