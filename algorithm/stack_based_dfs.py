def dfs(*args):
    # 終端処理 is_end(*args)
    # 入るときの処理 into(*args)
    # 探索候補について再帰する dfs(*next_args) for all next_args← こいつをスタックでうまくやる
    # 抜けるときの処理 outof(*args)
    pass

# これらをstackを用いて書き換える


def dfs(*args):  # 戻り値なしver
    S_args = [args]  # 引数管理のstack
    S_cmd = [0]  # 0:into, 1:outofの処理をすべきと記録するstack

    def is_end(args):
        '''終了条件書く
        u, = args
        return u == 4
        '''

    def into(args):
        '''入るときの処理
        u, = args
        tour.append(u)
        '''

    def nxt(args):
        S_args.append(args)  # 抜けるときに戻ってくることを予約
        S_cmd.append(1)
        '''今の引数からみて次の引数を列挙
        u, = args
        for nx in tree[u]:  # 次の探索
            _stack(nx)
        '''

    def outof(args):
        '''抜けるときの処理
        u, = args
        tour.append(u)
        '''

    def _stack(*args):  # お好きな引数で
        S_args.append(args)
        S_cmd.append(0)

    while S_cmd:
        now_args = S_args.pop()
        cmd = S_cmd.pop()
        if cmd == 0:
            if is_end(now_args):
                continue
            into(now_args)
            nxt(now_args)  # 次の再帰する(次のintoを予約)
        else:
            outof(now_args)



# TODO 以下書きかけ
tour = []


def dfs(*args):
    S_args = [args]  # 引数管理のstack
    S_cmd = [0]  # 0:into, 1:outofの処理をすべきと記録するstack
    S_rets = []  # 戻り値を管理するstack
    S_is_reduce = []  # 対応する位置の戻り値は次回でreduceされるか

    def is_end(args):
        #####終了条件書く#####
        return False
        u, = args
        cond = (u == 2)
        return cond
        ret = 1000
        return cond, ret  # u==2で終了し十分大きい値を返す
        ##########

    def into(args):
        #####入るときの処理#####
        u, = args
        tour.append(u)
        S_rets.append(V[u])  # 初期値を代入
        S_is_reduce.append(0)
        ##########

    def nxt(args):
        #####今の引数からみて次の引数を列挙しろ#####
        u, = args
        for nx in tree[u]:
            _stack(nx)
        ##########

    def outof(args):
        #####抜けるときの処理#####
        u, = args
        tour.append(u)
        # ret = 10**9  # 初期値
        ret = 10**9  # 初期値
        while S_is_reduce[-1]:  # reduceが必要なもの
            S_is_reduce.pop()
            ret = min(ret, S_rets.pop())
        S_is_reduce.pop()
        ret = min(ret, S_rets.pop())
        S_is_reduce.append(1)
        S_rets.append(ret)

        print(u, ret)

    def _stack(*args):  # お好きな引数で
        S_args.append(args)
        S_cmd.append(0)

    while S_cmd:
        now_args = S_args.pop()
        cmd = S_cmd.pop()
        if cmd == 0:
            into(now_args)
            S_args.append(now_args)  # 抜ける処理を予約
            S_cmd.append(1)
            if not is_end(now_args):
                nxt(now_args)  # 次の再帰する(次のintoを予約)
        else:
            outof(now_args)

    return S_rets


from collections import defaultdict
V = [5, 4, 2, 3, 3, 1]
tree = defaultdict(lambda: [])
tree[0] = [5, 1]
tree[1] = [3, 2]
tree[2] = [4]

print(dfs(0))
print(tour)


from collections import defaultdict
V = [5, 4, 2, 3, 3, 1]
tree = defaultdict(lambda: [])
tree[0] = [5, 1]
tree[1] = [3, 2]
tree[2] = [4]

print(dfs(0))
print(tour)


# # fibonacci
# def dfs(*args):
#     S_args = [args]  # 引数管理のstack
#     S_cmd = [0]  # 0:into, 1:outofの処理をすべきと記録するstack
#     S_rets = []  # 戻り値を管理するstack
#     S_is_reduce = []  # 対応する位置の戻り値は次回でreduceされるか

#     def is_end(args):
#         #####終了条件書く#####
#         n, = args
#         if n == 1:
#         return False
#         cond = (u == 2)
#         return cond
#         ret = 1000
#         return cond, ret  # u==2で終了し十分大きい値を返す
#         ##########

#     def into(args):
#         #####入るときの処理#####
#         u, = args
#         tour.append(u)
#         S_rets.append(V[u])  # 初期値を代入
#         S_is_reduce.append(0)
#         ##########

#     def nxt(args):
#         #####今の引数からみて次の引数を列挙しろ#####
#         u, = args
#         for nx in tree[u]:
#             _stack(nx)
#         ##########

#     def outof(args):
#         #####抜けるときの処理#####
#         u, = args
#         tour.append(u)
#         # ret = 10**9  # 初期値
#         ret = 10**9  # 初期値
#         while S_is_reduce[-1]:  # reduceが必要なもの
#             S_is_reduce.pop()
#             ret = min(ret, S_rets.pop())
#         S_is_reduce.pop()
#         ret = min(ret, S_rets.pop())
#         S_is_reduce.append(1)
#         S_rets.append(ret)

#         print(u, ret)

#     def _stack(*args):  # お好きな引数で
#         S_args.append(args)
#         S_cmd.append(0)

#     while S_cmd:
#         now_args = S_args.pop()
#         cmd = S_cmd.pop()
#         if cmd == 0:
#             into(now_args)
#             S_args.append(now_args)  # 抜ける処理を予約
#             S_cmd.append(1)
#             if not is_end(now_args):
#                 nxt(now_args)  # 次の再帰する(次のintoを予約)
#             else:
#                 outof(now_args)  # その場で抜けちゃうのは？
#         else:
#             outof(now_args)

#     return S_rets
