# メインアイデア
# T分割と||分割で差が小さい方を探す
# 90度回転させた場合も考える
# 総合的に小さい方を採用する
# 図的な解説は別のPDFをアップロードしてある
H, W = list(map(int, input().split()))

# 3の倍数の場合は即座に0とわかる
if (H * W) % 3 == 0:
    print(0)
    exit()


def try_divi(W, H):
    # パターン1
    err = W
    # パターン2
    w2, w3 = W // 2, W // 2
    if W % 2:
        w2 += 1
    for h1 in range(1, H // 2 + 1):
        h2 = H-h1
        S1, S2, S3 = h1*W, w2*h2, w3*h2
        new_err = max(S1, S2, S3) - min(S1, S2, S3)
        if new_err < err:
            err = new_err
    return err


print(min(try_divi(W, H), try_divi(H, W)))
