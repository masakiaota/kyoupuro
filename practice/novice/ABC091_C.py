# 問題 https://atcoder.jp/contests/abc091/tasks/arc092_a
# 素晴らしくわかりやすい解説 https://speakerdeck.com/ichijyo/atcoder-beginner-contest-091-c-2d-plane-2n-points-fei-gong-shi-jie-shuo
# 別の解説　https://pitsbuffersolution.com/compro/atcoder/arc092c.php

# 解説が豊富な実装 https://atcoder.jp/contests/abc091/submissions/5250954

N = int(input())
AB = [list(map(int, input().split())) for _ in range(N)]
CD = [list(map(int, input().split())) for _ in range(N)]

# 青い点を基準に組み合わせの赤い点を探したい

# 赤い点も青い点もx座標をもとに並び替えたい
AB.sort()
CD.sort()

from operator import itemgetter


# 青い点について小さい順にマッチングさせていく
cnt = 0
for c, d in CD:
    # c,dよりも小さい中で一番大きいy座標が大きいものを選ぶ
    del_idx = None
    tmpmax = -1
    for i, (a, b) in enumerate(AB):
        if a < c and b < d:
            if b > tmpmax:
                # print('cd=({},{})'.format(c, d))
                # print('red=({},{})'.format(a, b))
                tmpmax = b
                del_idx = i

    if del_idx is not None:
        # print(del_idx)
        AB[del_idx] = [float('inf'), float('inf')]
        cnt += 1


print(cnt)
