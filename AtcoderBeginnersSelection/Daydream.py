S = input()[::-1]
# T = []
T_dict = {
    0: "dream"[::-1],
    1: "dreamer"[::-1],
    2: "erase"[::-1],
    3: "eraser"[::-1]
}


def return_str(_S):
    """
    切れるか文字列を返す、だめならFalse
    """
    for t in T_dict.values():
        if _S[:len(t)] == t:
            return _S[len(t):]
    return False


while True:
    # print(S)
    S = return_str(S)
    if S == False:
        print("NO")
        break
    elif len(S) == 0:
        print("YES")
        break
