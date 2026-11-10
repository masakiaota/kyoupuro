"""データセット別の評価記録の保存先と読み取り。"""

from __future__ import annotations

from hashlib import sha256
import json
from pathlib import Path, PurePosixPath

RECORDS_DIR = Path(__file__).resolve().parents[1] / "results" / "eval_records"


def records_path(input_dir: str, records_dir: Path = RECORDS_DIR) -> Path:
    if not isinstance(input_dir, str) or not input_dir or "\0" in input_dir:
        raise ValueError("input_dir must be a nonempty path")
    dataset = PurePosixPath(input_dir)
    # 外部入力や予約名はハッシュ化し、保存先の逸脱と名前の衝突を防ぐ。
    if (dataset.is_absolute() or not dataset.parts or ".." in dataset.parts
            or dataset.parts[0] == "_external" or dataset.as_posix() != input_dir):
        return records_dir / "_external" / (sha256(input_dir.encode()).hexdigest() + ".jsonl")
    return records_dir.joinpath(*dataset.parts[:-1], dataset.name + ".jsonl")


def record_paths(input_dir: str | None = None, records_dir: Path = RECORDS_DIR):
    if input_dir is not None:
        path = records_path(input_dir, records_dir)
        return [path] if path.is_file() else []
    return sorted(records_dir.rglob("*.jsonl"))


def iter_records(input_dir: str | None = None, records_dir: Path = RECORDS_DIR):
    for path in record_paths(input_dir, records_dir):
        with path.open() as handle:
            for line in handle:
                if line.strip():
                    yield json.loads(line)
