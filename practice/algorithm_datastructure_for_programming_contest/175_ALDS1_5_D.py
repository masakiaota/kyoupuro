# ぶっちゃけアイデアを理解するのが難しかった。
# 詳細な解説は本を読もう
# 長さ2の配列ならば反転数をもとめるのは一瞬
# 2つの配列をソートしながらマージするときについでに反転数が求まるのが鍵で、これを実装に組み込む

INF = 10**9 + 1


def merge(A: list, left: int, mid: int, right: int) -> int:
    '''
    2つの配列をmergeする関数

    Aはこのプログラムで処理する配列。
    left, mid, rightはAのindex。
    '''
    cnt = 0  # mergeするときに何回反転するか
    n1 = mid-left  # 右の方の配列のlen (反転数を計算するときにこいつを使う)
    # n2 = right-mid  # 左の方の配列のlen ぶっちゃけいらない

    L = A[left:mid]  # python sliceはshallow copyらしい
    L.append(INF)  # 番兵
    R = A[mid:right]
    R.append(INF)  # 番兵

    i_l, i_r = 0, 0  # LとRのコントロール用idx
    for k in range(left, right):
        # L,Rの先頭を比較していって小さい方をAに打ち込む
        if L[i_l] <= R[i_r]:
            A[k] = L[i_l]
            i_l += 1
        else:
            A[k] = R[i_r]
            i_r += 1
            cnt += n1-i_l  # ここが重要 反転したときだけ反点数を足し込んでいく
    return cnt


def merge_sort(A: list, left: int, right: int):
    '''
    マージソートをしつつ、反転数を再帰的に計算する。

    ここではマージを再帰的に適応し反転数をどんどん上に伝播させていく仕組み
    '''
    if left + 1 >= right:  # 終了条件
        return 0

    mid = (left+right)//2
    v1 = merge_sort(A, left, mid)  # 右左にも再帰的にmerge_sortを適応
    v2 = merge_sort(A, mid, right)
    v3 = merge(A, left, mid, right)  # 右左整ったらmerge

    return v1+v2+v3


n = int(input())
A = list(map(int, input().split()))
print(merge_sort(A, 0, n))
