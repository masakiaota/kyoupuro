# https://onlinejudge.u-aizu.ac.jp/courses/lesson/1/ALDS1/4/ALDS1_4_D
# 工夫すればO(logn)ぐらいで解けるかもしれないが、nは最大で10**5。なのでO(nlogn)でもギリギリ間に合う(1.7*10**6回ぐらい)
# Pを与えたときにk台のトラックに詰める荷物の量を返す関数(O(n))を設計すれば二分探索でO(nlogk)で処理できる。
# Pを与えたときに何台のトラックが必要かを返す関数(O(n))でもおk

# イメージとしては単調増加関数を経由して二分探索する感じ


# %%
def n_nimotu_if_P(P, k, arr):  # Pに対する単調増加関数
    weight = 0
    n_trucks = 1  # 1台目から積んでいく
    for i, a in enumerate(arr):
        # iの荷物について積むかどうか決め、それに応じてトラックの数を増加させる
        if P < a:  # P<aの時点で即時終了 絶対その荷物は積めないので
            return i  # i個の荷物が詰める(i-1番目までの荷物が積める)

        if weight + a <= P:  # Pを超える手前までどんどん積んでいく
            weight += a
        else:
            n_trucks += 1  # Pを超えるなら次のtruckを用意
            weight = a  # 新たなトラックに積んでいる

        if n_trucks == k+1:  # k+1台目のtruckは存在しない。これまでに積んだ荷物の数を返す
            return i  # (i-1番目までのi個の荷物が積める)

    # truckに余裕があるなら全部詰める
    return len(arr)  # すべて運べるならlen(arr)を返すとする。


# %%
n, k = map(int, input().split())
luggages = []
for _ in range(n):
    luggages.append(int(input()))

# 探索する配列が未知のため、pythonのbisectは使えない状況。自分で実装
# n_nimotui_if_P()==n ならば 荷物を全部運んだ(もしくは余裕があるということ)。
# ==ならPを小さくし<ならばPを大きくしていく二分探索で解ける

left, right = 0, 10000 * n
i_break = 0
while right - left > 1:
    mid = (left+right)//2
    if n <= n_nimotu_if_P(mid, k, luggages):  # もし余裕があるならPを減らす
        right = mid  # ギリギリOKなのはrightに最終的に格納されているはず
    else:  # 余裕がなければ(n>n_nimotu_if_P())、Pを増やす
        left = mid

print(right)
