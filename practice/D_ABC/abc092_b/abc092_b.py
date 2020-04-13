# https://atcoder.jp/contests/abc092/tasks/arc093_b

A, B = map(int, input().split())


# 仮に100*100にサイズを固定したとする
# 上半分を全部白に、下半分を全部黒にすれば必ず白領域黒領域1個ずつ作れる
# 50*100の領域に、違う色のマスは最大25*50敷き詰められるから500という制約ならば余裕で敷き詰められる
# 上半分に関してはB個-1個黒を敷き詰めれば良くて、下半分に関してはA-1個白を敷き詰めれば良い。(デフォは上がすべて白)

# 上半分の用意
n_B = B - 1
n_A = A - 1
print(100, 100)
for _ in range(25):
    out = []
    for i in range(100):
        if i % 2 == 0 and n_B > 0:
            out.append('#')
            n_B -= 1
        else:
            out.append('.')
    print(''.join(out))
    print('.' * 100)

# 下半分
for _ in range(25):
    out = []
    print('#' * 100)
    for i in range(100):
        if i % 2 == 0 and n_A > 0:
            out.append('.')
            n_A -= 1
        else:
            out.append('#')
    print(''.join(out))
