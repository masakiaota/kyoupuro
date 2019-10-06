# https: // onlinejudge.u-aizu.ac.jp/courses/lesson/1/ALDS1/5/ALDS1_5_C
# 実装が面倒だけどやるだけ
# 一筆書きが面倒

n = int(input())

p1, p2 = (0, 0), (100, 0)
ans_ls = [p1, p2]


def koch(d, p1, p2):
    '''
    d...現在の深さ 0スタートにする
    p1,p2...端の座標。ここから途中の座標を計算しlistにappendする
    '''
    # 終了条件
    if d == n:
        return

    # 内分点の計算
    s = (2/3 * p1[0] + 1/3 * p2[0], 2/3 * p1[1] + 1/3 * p2[1])
    t = (1/3 * p1[0] + 2/3 * p2[0], 1/3 * p1[1] + 2/3 * p2[1])

    # 正三角形の頂点uの計算
    # (u-s)=R(1/3 pi) (t-s)が成り立つのでsを移行してuが計算できる。R()は回転行列で、加法定理から導出可能。
    R = [[1/2, -0.86602540378], [0.86602540378, 1/2]]
    u = (R[0][0] * (t[0] - s[0]) + R[0][1] * (t[1] - s[1]) + s[0],
         R[1][0] * (t[0] - s[0]) + R[1][1] * (t[1] - s[1]) + s[1])
    ans_ls.extend([s, u, t])  # 確定した点を追加

    # 再帰的に探索
    koch(d+1, p1, s)
    print(*s)
    koch(d+1, s, u)
    print(*u)
    koch(d+1, u, t)
    print(*t)
    koch(d+1, t, p2)


print(*p1)
koch(0, p1, p2)
print(*p2)

# AOJでもnumpyが使えると思っていた時期が私にもありました。
# しかも一筆書きになってないから不正解

# import numpy as np

# n = int(input())

# p1, p2 = np.array([0, 0]), np.array([100, 0])

# ans_ls = [p1, p2]


# def koch(d, p1, p2):
#     '''
#     d...現在の深さ 0スタートにする
#     p1,p2...端の座標。ここから途中の座標を計算しlistにappendする
#     '''
#     # 終了条件
#     if d == n:
#         return

#     # 内分点の計算
#     s = 2/3 * p1 + 1/3 * p2
#     t = 1/3 * p1 + 2/3 * p2

#     # 正三角形の頂点uの計算
#     # (u-s)=R(1/3 pi) (t-s)が成り立つのでsを移行してuが計算できる。R()は回転行列で、加法定理から導出可能。
#     r_rad = 1/3 * np.pi
#     R = np.array([
#         [np.cos(r_rad), -np.sin(r_rad)],
#         [np.sin(r_rad), np.cos(r_rad)]
#     ])
#     u = np.dot(R, (t-s)) + s
#     ans_ls.extend([s, u, t])  # 確定した点を追加

#     # 再帰的に探索
#     koch(d+1, p1, s)
#     koch(d+1, s, u)
#     koch(d+1, u, t)
#     koch(d+1, t, p2)


# koch(0, p1, p2)
# ans_ls = np.array(ans_ls).tolist()
# ans_ls.sort()
# for ans in ans_ls:
#     print(*ans)
