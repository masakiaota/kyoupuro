# max速度で移動したと仮定したとき、どの女の家を経由してもbに時間内にbにたどり着けないことを示せれば'NO'
sx, sy, tx, ty, T, V = map(int, input().split())
n = int(input())
s = complex(sx, sy)
t = complex(tx, ty)

for _ in range(n):
    x, y = map(int, input().split())
    p = complex(x, y)
    d = abs(p - s) + abs(t - p)
    if d / V <= T:
        print('YES')
        exit()
print('NO')
