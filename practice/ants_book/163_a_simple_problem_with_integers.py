# http://poj.org/problem?id=3468
# 蟻本と同じだけどここに同じことが書いてある https://algo-logic.info/binary-indexed-tree/#toc_id_1


class BIT:
    def __init__(self, n):
        self.n = n
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


class RangeAddBIT:  # range add query
    # [l,r)にxを加算する
    # [0,[l,r), i)のとき→bit.sum(i)+(rx-lx) (iによらない)
    # [0,[l,i),r)のとき→bit.sum(i)+(ix-lx)
    # [0,i),[l,r)のとき→bit.sum(i) (iによらない)
    # を加算できれば良い。bit.sum(i)が0だと見立てるとわかりやすいかも？
    # 場合分け2つ目における1項目をbit1,2項目をbit2とする
    def __init__(self, n: int):
        self.n = n
        self.bit1 = BIT(n)  # bit1.sum(i)*iで xiを達成したい部分 imos方的に使う
        self.bit2 = BIT(n)  # bit2.sum(i)で -xlを達成したい部分が手に入る。 r<iで区間加算後の和に相当する

    def init(self, ls):
        self.bit2.init(ls)

    def add(self, l: int, r: int, x):
        '''[l,r)の要素にxを足し込む'''
        self.bit1.add(l, x)
        self.bit1.add(r, -x)
        self.bit2.add(l, -x * l)
        self.bit2.add(r, x * r)

    def sum(self, l, r):
        '''[l,r)の区間の合計を取得'''
        return self._sum(r) - self._sum(l)

    def _sum(self, i: int):
        '''[,i)の合計を取得'''
        return self.bit1._sum(i) * i + self.bit2._sum(i)


N = 10
Q = 5
A = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
queries = [(0, 4, 4),
           (0, 1, 10),
           (0, 2, 4),
           (1, 3, 6, 3),
           (0, 2, 4)]

bit = RangeAddBIT(N)
bit.init(A)
for i in range(Q):
    cmd, *tmp = queries[i]
    if cmd == 0:
        l, r = tmp
        print(bit.sum(l - 1, r))
    else:
        l, r, x = tmp
        bit.add(l - 1, r, x)
