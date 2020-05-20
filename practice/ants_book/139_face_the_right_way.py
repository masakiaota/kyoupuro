# Kについて全探索する必要あり
# 一番左からgreedyに反転していけばいい(?)
# だけどKの探索にO(N)、区間反転がO(N)で、区間の反転にO(N)かかることからO(N^3)かかってしまう。
# 区間反転は高速化できる！(遅延セグ木で殴っても行ける)
# imos法っぽいことをしてる？
# 蟻本の解説がだるすぎるのでimosしました

N = 7
cows = 'BBFBFBB'
ans_M = 10**5
ans_K = 10**5
for k in range(1, N + 1):  # Kについて全探索
    m = 0
    is_valid = True  # 有効なkか？
    is_fliped = [0] * (N + 1)
    for i in range(N):  # 各牛について左から見ていく
        is_fliped[i] += is_fliped[i - 1]  # デルタ関数を積分してステップ関数を作るイメージ
        if is_fliped[i] & 1:  # 奇数のときは反転してる
            if cows[i] == 'B':
                continue
        else:  # ひっくり返ってない牛
            if cows[i] == 'F':
                continue
        # ひっくり返す作業が必要
        m += 1
        # K個ひっくり返す #ここではデルタ関数を立てるイメージ
        if i + k > N:
            is_valid = False  # ピッタリ牛をひっくり返すことはできない！
            break
        is_fliped[i] += 1
        is_fliped[i + k] -= 1

    if is_valid:
        print(k, m, is_fliped)  # 確認用
        if m < ans_M:
            ans_M = m
            ans_K = k
print(ans_K, ans_M)
