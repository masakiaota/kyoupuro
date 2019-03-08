N, M = list(map(int, input().split()))
A = list(map(int, input().split()))
B = list(map(int, input().split()))

# 早期終了

if len(set(A)) != len(A):
    print(0)
    exit()
if len(set(B)) != len(B):
    print(0)
    exit()

# 高速化のため事前ソート

A.sort(reverse=True)
B.sort(reverse=True)

n_A, n_B = 0, 0  # ある数よりも大きい要素数(単調増加)
# この問題は場合分けを思いつけるか、計算量を減らせるかにかかっている

# TLEでなくするには2つ工夫が必要
# 1. a in A等を工夫する
# 2. 与えられた集合の中にxょり大きい要素がいくつあるかという処理の高速化

# 1は簡単なので2についてだけ記録を残す
# これをn_Aとおくと、n_Aには単調増加性があることがすぐにわかる
# xをどんどん小さくしているからである。
# n_Aを逐一計算する代わりに、いままでの情報を使ってn_Aを求められると処理が高速化される
# A=[182, 181, 180, 179, 178, 176, 175, 169, 168, 167, 158, 151, 147, 129]とする(sorted)とすると、
# A[n_A]=xとなるn_Aがほしい。xが登場するまでのイテレーションでx_pre in Aとなる要素があるたび、その回数をカウントすればよい
#


def ret_n_given_x(x):
    global n_A, n_B
    flgA = False
    flgB = False
    # a in Aの代わり(こうしないとTLE)
    for a in A[n_A:]:
        if a == x:
            flgA = True
            break
    for a in B[n_B:]:
        if a == x:
            flgB = True
            break

    if flgA and flgB:
        n_A += 1  # 上記で解説した個数カウント
        n_B += 1  # xより大きな要素が何個あったかついでに記録できる
        return 1
    elif flgA:
        n_A += 1
        return n_B
    elif flgB:
        n_B += 1
        return n_A
    else:
        return (n_A * n_B)-(N*M-x)


MOD = 10**9 + 7
ans = 1
for x in range(N * M, 0, -1):
    ans *= ret_n_given_x(x)
    ans %= MOD
    # print(x, ans, ret_n_given_x(x))
    if ans == 0:
        break

print(int(ans))
