# https://atcoder.jp/contests/abc055/tasks/arc069_b

# 先頭の2つの動物を仮定すれば後は決定的に決まる
# 決定的に決めた後に成り立つかどうか判定すれば良い 具体的にはs[0]と生成した文字列SのS[-1],S[0],S[1]を調べればよい
N = int(input())
s = input()


def simu(ret):
    for i, ss in enumerate(s[1:] + s[:1], start=1):
        if ret[i] == 'S' and ss == 'o':
            ret += ret[i - 1]
        elif ret[i] == 'S' and ss == 'x':
            ret += 'W' if ret[i - 1] == 'S' else 'S'
        elif ret[i] == 'W' and ss == 'o':
            ret += 'W' if ret[i - 1] == 'S' else 'S'
        elif ret[i] == 'W' and ss == 'x':
            ret += ret[i - 1]
    return ret


def check(ret):
    if ret[-2] != ret[0]:  # 回ってきたときに一致しないということはだめということ
        return False
    if ret[-1] != ret[1]:
        return False
    return True
    # ここの大量の条件分岐はらくできそう
    # if ret[0] == 'S' and ss == 'o':
    #     return ret[-2] == ret[1]
    # elif ret[0] == 'S' and ss == 'x':
    #     return ret[-2] != ret[1]
    # elif ret[0] == 'W' and ss == 'o':
    #     return ret[-2] != ret[1]
    # elif ret[0] == 'W' and ss == 'x':
    #     return ret[-2] == ret[1]


# パターン1
for ret in ['SS', 'SW', 'WS', 'WW']:
    ret1 = simu(ret)
    # print(ret1)
    if check(ret1):
        print(ret1[:-2])
        exit()
print(-1)
