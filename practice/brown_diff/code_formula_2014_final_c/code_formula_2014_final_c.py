# https://atcoder.jp/contests/code-formula-2014-final/tasks/code_formula_2014_final_c
import re
print(*sorted(set(re.findall('@(\w+)', input()))), sep='\n')
