# めんどくさいだけやん...


from collections import defaultdict
replacedict = defaultdict(lambda: '')
replacedict['b'] = '1'
replacedict['c'] = '1'
replacedict['d'] = '2'
replacedict['w'] = '2'
replacedict['t'] = '3'
replacedict['j'] = '3'
replacedict['f'] = '4'
replacedict['q'] = '4'
replacedict['l'] = '5'
replacedict['v'] = '5'
replacedict['s'] = '6'
replacedict['x'] = '6'
replacedict['p'] = '7'
replacedict['m'] = '7'
replacedict['h'] = '8'
replacedict['k'] = '8'
replacedict['n'] = '9'
replacedict['g'] = '9'
replacedict['z'] = '0'
replacedict['r'] = '0'


def replace(S: str):
    '''1単語を入力して 数字を返す'''
    ret = []
    for s in S.lower():
        ret.append(replacedict[s])
    return ''.join(ret)


N = int(input())
W = input().split()
ans = []
for w in W:
    tmp = replace(w)
    if tmp != '':
        ans.append(tmp)

print(*ans)
