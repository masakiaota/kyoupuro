def manhattan(x_1, x_2):
    """
    x_1は(x,y)のタプル
    マンハッタン距離を返す
    """
    return abs(x_1[0] - x_2[0]) + abs(x_1[1] - x_2[1])


def can_or_not(dist, dt):
    if dist > dt:
        return False
    elif dist % 2 != dt % 2:
        return False
    return True


flgs = set()
t_p, x_p, y_p = 0, 0, 0

N = int(input())
for n in range(N):
    t, x, y = list(map(int, input().split()))
    dist = manhattan((x_p, y_p), (x, y))
    dt = t - t_p
    flgs.add(can_or_not(dist, dt))
    t_p, x_p, y_p = t, x, y

if False in flgs:
    print("No")
else:
    print("Yes")
