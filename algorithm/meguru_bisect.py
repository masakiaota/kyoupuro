# めぐる式 二分探索
# https://qiita.com/drken/items/97e37dd6143e33a64c8c
# メリット
# 配列化できない関数を探索可能
# バグりにくい (終了状態がきちんとしている)
# ライブラリとして扱うことが可能で実装が高速
# 思考リソースの消耗を防げる。(leftが条件を満たすので最大のargが欲しい場合でも、rightが条件を満たすので最小のargが欲しい場合でも同じコードで動く)


def is_ok(arg):
    # 条件を満たすかどうか？問題ごとに定義
    pass


def meguru_bisect(ng, ok):
    '''
    define is_okと
    初期値のng,okを受け取り,is_okを満たす最小(最大)のokを返す
    ng ok は  とり得る最小の値-1 とり得る最大の値+1
    最大最小が逆の場合はよしなにひっくり返す
    '''
    while (abs(ok - ng) > 1):
        mid = (ok + ng) // 2
        if is_ok(mid):
            ok = mid
        else:
            ng = mid
    return ok
