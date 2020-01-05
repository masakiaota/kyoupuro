# https://atcoder.jp/contests/diverta2019-2/tasks/diverta2019_2_c

# 重要な性質: Aの各要素には好きな符号をつけるように構成できる。ただしすべて+or-はむり
# つまり負数なら-をつけ、正数ならそのまま

# 正にすべてプラスを付ける方法→負数を一つ借りて、正を引いてひって、最後に符号を逆転
# 負数に-を付ける方法→ただ引くだけで良い


# すべて負or正のときに注意して実装
N = int(input())
A = list(map(int, input().split()))

A.sort()


if A[0] >= 0:  # もし全要素が正数なら
    now = A[0]
    ansls = []
    for a in A[1:-1]:
        ansls.append((now, a))
        now = now - a

    ansls.append((A[-1], now))
    ans = A[-1] - now
    print(ans)
    for ans in ansls:
        print(*ans)
elif A[-1] <= 0:  # もし全要素が負数なら
    A = A[::-1]
    now = A[0]
    ansls = []
    for a in A[1:]:
        ansls.append((now, a))
        now = now - a

    print(now)
    for ans in ansls:
        print(*ans)
else:  # 正負が混在するならば好きに作れる
    ans = sum([abs(a) for a in A])
    print(ans)
    from bisect import bisect_right
    idx = bisect_right(A, 0) - 1  # ギリ負数を指す

    now = A[idx]
    ansls = []
    # 正の最善戦略
    for a in A[idx + 1:-1]:
        ansls.append((now, a))
        now = now - a
    ansls.append((A[-1], now))
    now = A[-1] - now  # ためていた符号大反転

    # print('-')
    # 続いて負の最善戦略
    for a in A[:idx]:
        ansls.append((now, a))
        now = now - a
    assert now == ans

    for ans in ansls:
        print(*ans)
