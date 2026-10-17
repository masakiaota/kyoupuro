#!/usr/bin/env python3
"""公式ZIPを展開し、公式採点器をビルドする。採点処理の移植は不要。"""

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parent
TOOLS = ROOT / "tools"


def unpack(zip_path):
    if TOOLS.exists():
        raise ValueError("tools/ は既にある。再ビルドは ZIP を指定せず python3 setup.py を実行する。")
    with tempfile.TemporaryDirectory(prefix="ahc-tools-") as tmp:
        with zipfile.ZipFile(zip_path) as archive:
            for item in archive.infolist():
                path = Path(item.filename)
                if path.is_absolute() or ".." in path.parts:
                    raise ValueError("ZIP 内のパスが不正だ。公式の tools.zip を指定する。")
            archive.extractall(tmp)
        manifests = [p for p in Path(tmp).rglob("Cargo.toml") if (p.parent / "src/bin").is_dir()]
        if len(manifests) != 1:
            raise ValueError("公式 Rust ツールの Cargo.toml を一つに特定できない。ZIP を確認する。")
        shutil.copytree(manifests[0].parent, TOOLS)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("zip", nargs="?", help="ダウンロードした公式ツールの ZIP")
    parser.add_argument("--mode", choices=("vis", "tester"), help="公式 README に合わせて選択")
    args = parser.parse_args()
    if args.zip:
        unpack(Path(args.zip).expanduser())
    if not (TOOLS / "Cargo.toml").is_file():
        raise ValueError("公式 ZIP を指定する: python3 setup.py ~/Downloads/tools.zip")

    has_tester = (TOOLS / "src/bin/tester.rs").is_file()
    old_config = ROOT / "eval_config.json"
    mode = args.mode
    if mode is None and old_config.is_file():
        mode = json.loads(old_config.read_text())["mode"]
    if mode is None:
        if has_tester:
            print(
                "採点方式を選択\n"
                "tools/README.md の実行方法に対応するコマンドを一つ実行する。\n"
                "  tester cmd ... の場合: python3 setup.py --mode tester\n"
                "  vis in.txt out.txt の場合: python3 setup.py --mode vis\n"
                "ZIP は tools/ に展開済みのため、再指定しない。"
            )
            return 2
        mode = "vis"
    if not (TOOLS / f"src/bin/{mode}.rs").is_file():
        raise ValueError(f"公式ツールに src/bin/{mode}.rs がない。tools/README.md を確認する。")

    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(TOOLS / "target")
    inputs = sorted((TOOLS / "in").glob("*.txt"))
    bins = [mode] + ([] if inputs else ["gen"])
    command = ["cargo", "build", "--release", "--locked"]
    for name in bins:
        command.extend(["--bin", name])
    print("公式ツールをビルドする（初回は少し時間がかかる）", flush=True)
    subprocess.run(command, cwd=TOOLS, env=env, check=True)
    if not inputs:
        seeds = TOOLS / "seeds.txt"
        if not seeds.exists():
            seeds.write_text("".join(f"{i}\n" for i in range(100)))
        subprocess.run([str(TOOLS / "target/release/gen"), str(seeds)], cwd=TOOLS, check=True)
        inputs = sorted((TOOLS / "in").glob("*.txt"))
    if not inputs:
        raise ValueError("tools/in に入力が生成されなかった。tools/README.md を確認する。")

    source = (TOOLS / f"src/bin/{mode}.rs").read_text()
    # AHC071/072 の clap 属性 long = "no-vis" と、直接の引数処理の両方に対応する。
    no_vis = mode == "vis" and ('"no-vis"' in source or '"--no-vis"' in source)
    config = {"mode": mode, "scorer_args": ["--no-vis"] if no_vis else [], "timeout": 10.0}
    old_config.write_text(json.dumps(config, ensure_ascii=False, indent=2) + "\n")
    print(f"\n準備完了: 採点方式={mode}, 入力={len(inputs)}件, 可視化省略={no_vis}")
    print("次は main.py の TODO を埋め、python3 eval.py --quick を実行する。")
    print("対話型なら snippets/interactive.py の行単位入力と flush を使う。" if mode == "tester"
          else "採点処理を Python に書き写す必要はない。")


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ValueError, OSError, zipfile.BadZipFile, subprocess.CalledProcessError) as error:
        print(f"準備失敗: {error}", file=sys.stderr)
        sys.exit(1)
