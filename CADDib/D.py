# 解き方天才過ぎやろ...
# 要はすべての色においてりんごの数が偶数になっている状態を相手に押し付けあうゲーム

N = int(input())
A = []
for _ in range(N):
    A.append(int(input()))


for a in A:
    if a % 2 == 1:
        # 奇数個のりんごの木が一つでもあれば、先行がそれをとり、後攻にEの状態を押し付けるので先行の勝ち
        print("first")
        exit()

print("second")
