# https://atcoder.jp/contests/apc001/tasks/apc001_c
# oは空席,mは男,fは女
# 奇数ならば必ずoは一つなければおかしい
# N=7のとき→mfmfomf oが奇数個だとoの両隣は必ず違う性別になる

# どの空席を見つけるか？奇数個のある方の空席を見つける
# mfm f omf だったら真ん中がfだったら右側にある
# mof m fmf だったら真ん中がmだったら左側にある
# mom f omo もできるからだめだ...

# 少なくとも1つは空席がある区間を二分探索する
# mfm f mof のように真ん中がfのとき左側はoが無いor 複数ある →右側には必ず一つ以上ある
# mfo m fof

# 二分探索したときに真ん中がfだと右側にoがある十分条件、 mだと左側にoがある十分条件
# ↑mから始まる場合。fから始まる場合は逆

N = int(input())

V = 'Vacant'
F = 'Female'
M = 'Male'
print(0, flush=True)
first = input()
if first == V:
    exit()
# print(N - 1, flush=True)
# right = input()
# if right == input():
#     exit()
left = 0
right = N - 1

# 二分探索
while True:
    mid = (left + right) // 2
    print(mid, flush=True)
    com = input()
    should_same = (mid - left) % 2 == 0  # oがなかったときにfirstと同じであるべきか？
    if should_same:
        cond = first == com
    else:
        cond = first != com

    if com == V:
        exit()
    if not cond:  # cond==Trueは左側は整っている
        # 左側を探索する十分条件
        right = mid - 1
    else:
        # 右側
        first = F if com != F else M
        left = mid + 1
