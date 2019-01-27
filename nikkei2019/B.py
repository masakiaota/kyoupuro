N = input()
A = input()
B = input()
C = input()


def get_n_trial(a_str, b_str, c_str):
    ret = 0
    for a, b, c in zip(a_str, b_str, c_str):
        if a == b == c:
            continue
        elif (a != b) and (b != c) and (c != a):
            ret += 2
        else:
            ret += 1
        # print(ret)
    return ret


print(get_n_trial(A, B, C))
