# 座標圧縮
# xとyは独立に扱えるのがポイント


from itertools import product


class Compress:  # すべての情報を残すのが好きなのでクラス化した
    def __init__(self, ls):
        self.i_to_orig = sorted(set(ls))
        self.orig_to_i = {}
        for i, zahyou in enumerate(self.i_to_orig):
            self.orig_to_i[zahyou] = i
        self.len = len(self.i_to_orig)

    def __len__(self):
        return len(self.i_to_orig)


w, h, n = 10, 10, 5
x1 = [1, 1, 4, 9, 10]
x2 = [6, 10, 4, 9, 10]
y1 = [4, 8, 1, 1, 6]
y2 = [4, 8, 10, 5, 10]
x1.extend([0, 0, w + 1, w + 1])  # 周りを黒線で囲っておく
y1.extend([0, h + 1, h + 1, 0])
x2.extend([0, w + 1, w + 1, 0])
y2.extend([h + 1, h + 1, 0, 0])

# 必要な座標の確保
# 端点の座標とその周囲-1,+1は確保する
X_comp = Compress([a + d for a, d in product(x1 + x2, (-1, 0, 1))])
Y_comp = Compress([a + d for a, d in product(y1 + y2, (-1, 0, 1))])


# 圧縮済みgridの用意
grid = [[0] * len(Y_comp) for _ in range(len(X_comp))]
# 実際にgridを塗る
for xs, ys, xt, yt in zip(x1, y1, x2, y2):
    # 圧縮後の座標に変換
    xs = X_comp.orig_to_i[xs]
    ys = Y_comp.orig_to_i[ys]
    xt = X_comp.orig_to_i[xt]
    yt = Y_comp.orig_to_i[yt]
    if xs > xt:
        xs, xt = xt, xs
    if ys > yt:
        ys, yt = yt, ys
    # 塗る
    for x in range(xs, xt + 1):
        for y in range(ys, yt + 1):
            grid[x][y] = 1

print(*grid, sep='\n')  # 確認用

# このgridの領域をあとはカウントすればok!
# 領域の個数は最大250*250の62500個
# 圧縮後のgridの要素数は最大3000*3000(9e6)で再帰関数でも多分できるけど、ちょっと怪しい
# がbfsを書くのも面倒なのでdfsで


def dfs(x, y):  # 周囲を探索しながら0に置き換える
    # 終了条件はなくても勝手に止まる
    for dx, dy in [(0, 1), (0, -1), (1, 0), (-1, 0)]:
        nx, ny = x + dx, y + dy
        if not (0 <= nx < len(X_comp) and 0 <= ny < len(Y_comp)):
            continue
        if grid[nx][ny] == 0:
            grid[nx][ny] = 1
            dfs(nx, ny)


ans = 0
for x, y in product(range(len(X_comp)), range(len(Y_comp))):
    if grid[x][y] == 1:
        continue
    else:
        ans += 1
        dfs(x, y)

print(ans - 1)  # 周囲をグルっと囲む0の分、1を引く
