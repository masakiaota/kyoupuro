# 有効数字関係

# 有効数字をゴリ押しで増やす #動作未確認
from decimal import Decimal
Decimal.getcontext().prec = 500  # Floatの桁数を500まで増やす
Decimal(3).sqrt()  # それで平方根を求める
Decimal(1) / Decimal(7)  # 1 / 7
