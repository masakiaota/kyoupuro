def run_length_encoding(s):
    '''
    連長圧縮を行う
    s ... iterable object e.g. list, str 

    return
    ----------
    s_composed,s_num
    '''
    s_composed = []
    s_sum = []
    pre = s[0]
    cnt = 1
    for ss in s[1:]:
        if pre == ss:
            cnt += 1
        else:
            s_sum.append(cnt)
            s_composed.append(pre)
            cnt = 1
            pre = ss
    s_sum.append(cnt)
    s_composed.append(pre)
    # assert len(s_sum) == len(s_composed)
    return s_composed, s_sum
