H, W = list(map(int, input().split()))

# 3の倍数の場合
if (H * W) % 3 == 0:
    print(0)
else:

    # 縦長にする
    H, W = max(H, W), min(H, W)

    # |-パターン
    BCh = 0
    Bw, Cw = W // 2, W // 2
    if W % 2 == 1:
        Bw += 1

    def get_abs(BCh):
        """
        最大最小の面積の差
        """
        S_B = BCh * Bw
        S_C = BCh * Cw
        S_A = W * (H - BCh)  # max
        # print(S_A)
        return abs(max([S_A, S_B, S_C]) - min([S_A, S_B, S_C]))

    # print(Bw)
    err = get_abs(BCh)
    # print(err)
    for h in range(1, H):
        new_err = get_abs(h)
        if new_err < err:
            err = new_err
            # print(h, err)
        else:
            break

        # else:
        #     break

    # ||パターン
    if H > 2:
        Hmin = H // 3
        Hmax = H // 3 + 1

        err2 = (Hmax - Hmin) * W
    else:
        err2 = 1

    if err2 < err:
        err = err2
    print(err)

# 多分回転らへんのことを考慮し忘れている
