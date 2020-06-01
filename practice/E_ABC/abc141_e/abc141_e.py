# https://atcoder.jp/contests/abc141/tasks/abc141_e
# https://www.slideshare.net/hcpc_hokudai/ss-121539419
# これのほうがいいかな？ https://sen-comp.hatenablog.com/entry/2020/01/16/174230

# 結局これが一番わかりやすい https://qiita.com/Pro_ktmr/items/16904c9570aa0953bf05

# zアルゴリズムをやるらしい


def z_algorithm(S: str):
    '''SとS[i]の最長共通接頭辞長 Z[i] の配列を返す O(len(S))'''
    Z = [-1] * len(S)  # -1で初期化 (-1が出てきたらおかしい！)
    Z[0] = len(S)
    i = 1  # 現在求めたいZ[i]の位置
    j = 0  # S[i:i+j]の文字列は先頭と一致している
    while i < len(S):
        while (i + j < len(S) and S[j] == S[i + j]):
            j += 1
        Z[i] = j

        if j == 0:
            i += 1
            continue
        k = 1
        while (k < j and k + Z[k] < j):  # ならば再利用可能
            # ∵kからZ[k]後がjより小さいということは同一文字列の終了がjを突き抜けないことが保証されている
            # →Z[k]のほうが必ず早く終了するので値はZ[i+k]の値はZ[k]になる
            Z[i + k] = Z[k]
            k += 1
        i += k  # 次のiはまだ埋まっていないところまで移動
        j -= k  # jも無駄な探索を避けるために移動分だけの幅減少に留める
        # jの働き後半がはみ出すときは確定させないで次の探索で確定できるようにするというだけ
    return Z


N = int(input())
S = input()
# まずzアルゴリズムを任意の開始地点に用いる
# ほしいデータ構造
# S[i:]とS[j:]の最長共通接頭辞長行列 M[i][j]
# これのうち重複していないなかでの最大値が答え。
# つまりmax{M[i][j] | i+M[i][j]<=j}

# i,jはあくまで相対位置なのでMを実際に構成しなくてもiを一つずつずらしてjを調べるということでおk
ans = 0
for i in range(N):  # 開始位置
    Z = z_algorithm(S[i:])
    for j in range(N - i):
        if Z[j] <= j:
            ans = max(ans, Z[j])
print(ans)
