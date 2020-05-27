class BIT:
    def __init__(self, n):
        self.n = n
        # self.num = 2 ** (self.n_origin - 1).bit_length()  # n以上の最小の2のべき乗
        self.bit = [0] * (self.n + 1)  # bitは(1based indexっぽい感じなので)

    def init(self, ls):
        assert len(ls) <= self.n
        # lsをbitの配列に入れる
        for i, x in enumerate(ls):  # O(n log n 掛かりそう)
            self.add(i, x)

    def add(self, i, x):
        '''i番目の要素にxを足し込む'''
        i += 1  # 1 based idxに直す
        while i <= self.n:
            self.bit[i] += x
            i += (i & -i)

    def sum(self, i, j):
        '''[i,j)の区間の合計を取得'''
        return self._sum(j) - self._sum(i)

    def _sum(self, i):
        '''[,i)の合計を取得'''
        # 半開区間なので i+=1しなくていい
        ret = 0
        while i > 0:
            ret += self.bit[i]
            i -= (i & -i)
        return ret


n = 4
A = [3, 1, 4, 2]
bit = BIT(max(A) + 1)  # 0~Aの最大までの座標を用意しておく
ans = 0
# i<jにおいて,ai>ajとなる要素の個数をカウント
# jを固定すれば、jより前に出現したajよりも大きい要素の数になる
for a in A:
    ans += bit.sum(a + 1, bit.n)  # aより大きい要素の個数が
    bit.add(a, 1)

    # for a in range(0, max(A) + 1):
    #     # ちゃんと各要素がなってるかの検証
    #     print(bit.sum(a, a + 1), end=' ')
    # print()


print(ans)
