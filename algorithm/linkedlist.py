class linkedlist:
    def __init__(self, N):
        '''要素がN個あるlinkedlistをつくる'''
        self.ls = [[-1, -1]
                   for _ in range(N)]  # 0はleft,1はrightへのリンク #-1は接続されていない

    def link(self, i, j):
        '''i番目の要素の右にj番目の要素をつける
        iの右、jの左へのリンクは破棄する'''
        if self.ls[i][1] != -1:  # もし接続されていればカットしておく
            self.cut(i, 1)
        self.ls[i][1] = j
        if self.ls[j][0] != -1:
            self.cut(j, 0)
        self.ls[j][0] = i

    def cut(self, i, is_right):
        '''i番目のis_right側のリンクを切る'''
        adj = self.ls[i][is_right]
        if adj == -1:  # 無いほうがいいかも？
            ValueError('unlinked element')
        self.ls[adj][1 - is_right] = -1
        self.ls[i][is_right] = -1
