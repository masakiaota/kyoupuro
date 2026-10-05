from pathlib import Path
from v089_data import sha
ROOT=Path(__file__).resolve().parents[2]
RUN=ROOT/'results/nn_rank/v133/20261005_neutral_best_studio'
BASE=ROOT/'src/bin/v113_integrated_nn_lns.cpp'
SOURCE=ROOT/'src/bin/v133_neutral_best.cpp'
BASE_SHA='037f9fcca137552c097bc396428699850cf591cf58e513e874c9058309bfd258'
def build():
    assert sha(BASE)==BASE_SHA
    assert SOURCE.is_file()
    return SOURCE
