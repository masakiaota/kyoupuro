# https://atcoder.jp/contests/abc147/tasks/abc147_c
# 制約が少ない→全探索可能
# 全探索によって矛盾がない場合を探す

# 問題はどうやって矛盾しているかどうかをどうやって判別するか
# katei[i]で1のあるiが成り立っているか確かめれば良い(矛盾しなければ良い)
# つまり、ある人の発言において、正直者ものの{i} in {katei[i]==1}となる
# かつ、不親切ものの{i} not in {j|katei[i]==1}となればよい。


N = int(input())
syoujiki = []
fusin = []
for _ in range(N):
    a = int(input())
    syou = set()
    fusi = set()
    for _ in range(a):
        x, y = list(map(int, input().split()))
        if y == 1:
            syou.add(x - 1)  # 0 based inexにする
        else:
            fusi.add(x - 1)
    syoujiki.append(syou)
    fusin.append(fusi)


def is_ok(n):  # katei[i]はbinaryなのでnで表現しn>>iでアクセスすることにする。
    # 過程集合の作成
    ret = 0
    katei = set()
    for i in range(N):
        if (n >> i) % 2:  # その桁が1なら
            katei.add(i)
            ret += 1
    for i in katei:  # 仮定でおいたi番目の人の発言を考慮する
        cond1 = syoujiki[i].issubset(katei)  # ここがおかしい。いま言及しているものしか考えられない
        cond2 = fusin[i].isdisjoint(katei)
        # cond1とcond2を最後までみたいすときにtrueにしたい
        # →満たさないときにfalseにする
        if (not cond1) or (not cond2):
            return 0
    return ret


ans = 0
for katei in range(1 << N):
    ans = max(is_ok(katei), ans)

print(ans)
