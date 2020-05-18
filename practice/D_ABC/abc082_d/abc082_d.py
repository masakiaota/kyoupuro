# https://atcoder.jp/contests/abc082/tasks/arc087_b


def run_length_encoding(s):
    '''
    連長圧縮を行う
    s ... iterable object e.g. list, str
    return
    ----------
    s_composed,s_num,s_idx
    それぞれ、圧縮後の文字列、その文字数、その文字が始まるidx
    '''
    s_composed = []
    s_sum = []
    s_idx = [0]
    pre = s[0]
    cnt = 1
    for i, ss in enumerate(s[1:], start=1):
        if pre == ss:
            cnt += 1
        else:
            s_sum.append(cnt)
            s_composed.append(pre)
            s_idx.append(i)
            cnt = 1
            pre = ss
    s_sum.append(cnt)
    s_composed.append(pre)
    # assert len(s_sum) == len(s_composed)
    return s_composed, s_sum, s_idx

# データに関しては事前にランレングス圧縮しておくと楽かもね


s = input()
x, y = map(int, input().split())
s_composed, s_sum, _ = run_length_encoding(s)

if s_composed[0] == 'T':  # 必ずTから始まるようにしたい
    s_composed.insert(0, 'F')
    s_sum.insert(0, 0)
# 前処理 xの移動履歴とyの移動履歴を作る
is_x = True
hist_x = []
hist_y = []
for s, n in zip(s_composed, s_sum):
    if s == 'T':
        if n & 1:  # xyの状態を反転させる
            is_x = not is_x
    else:
        if is_x:
            hist_x.append(n)
        else:
            hist_y.append(n)

'''
dpx[i][j]...x座標がjであることが可能か？ x方向の移動についてi回目の移動のあとに。
dpy[i][j]...y座標がjであることが可能か？ y方向の移動についてi回目の移動のあとに。

更新則
xについての移動,距離d移動するとすると
dpx[i+1][j+d]=dpx[i+1][j+d] or dpx[i][j]
dpx[i+1][j-d]=dpx[i+1][j-d] or dpx[i][j]
(配るDP)で更新すると楽かな？
yも同様

初期条件
dpx[:][:]=False
dpx[0][d_x0]=True #xの0回目の移動。

dpy[:][:] = False
dpy[0][d_y0]=True 簡単のために
dpy[0][-d_y0]=True
'''

# どこかにバグが有ってREになる...
try:
    if hist_x:
        x_range = 2 * (sum(hist_x) + 1500)  # ここが0になるときがある
        shift = x_range // 2
        dpx = [[False] * x_range for _ in range(len(hist_x))]
        dpx[0][hist_x[0] + shift] = True
        for i in range(1, len(hist_x)):
            d = hist_x[i]
            for j in range(x_range):
                if j + d < x_range:
                    dpx[i][j] = dpx[i][j] or dpx[i - 1][j + d]
                if -1 < j - d:
                    dpx[i][j] = dpx[i][j] or dpx[i - 1][j - d]
        ansx = dpx[-1][x + shift]
    # else:
    #     ansx = x == 0

    if hist_y:
        y_range = 2 * (sum(hist_y) + 1500)  # ここが0になるときがある
        shift = y_range // 2
        dpy = [[False] * y_range for _ in range(len(hist_y))]
        dpy[0][hist_y[0] + shift] = True
        dpy[0][-hist_y[0] + shift] = True
        for i in range(1, len(hist_y)):
            d = hist_y[i]
            for j in range(y_range):
                if j + d < y_range:
                    dpy[i][j] = dpy[i][j] or dpy[i - 1][j + d]
                if -1 < j - d:
                    dpy[i][j] = dpy[i][j] or dpy[i - 1][j - d]
        ansy = dpy[-1][y + shift]
    else:
        ansy = y == 0

    print('Yes' if ansx and ansy else 'No')
except:
    print('No')

# print(ansx, ansy)
# print(*dpy, sep='\n')
