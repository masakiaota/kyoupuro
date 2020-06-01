# 文字列関連アルゴリズム
def run_length_encoding(s):
    '''連長圧縮を行う
    s ... iterable object e.g. list, str 
    return
    ----------
    s_composed,s_num,s_idx
    それぞれ、圧縮後の文字列、その文字数、その文字が始まるidx'''
    s_composed, s_sum = [], []
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


# z algorithm 参考
# https://www.slideshare.net/hcpc_hokudai/ss-121539419
# これのほうがいいかな？ https://sen-comp.hatenablog.com/entry/2020/01/16/174230
# 結局これが一番わかりやすい https://qiita.com/Pro_ktmr/items/16904c9570aa0953bf05
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


# TODO ローリングハッシュ
