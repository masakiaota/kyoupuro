#!/usr/bin/env python3
from __future__ import annotations

import argparse
import http.cookiejar
import re
import subprocess
import sys
from pathlib import Path

import requests
from bs4 import BeautifulSoup


def _infer_dir_from_url(url: str) -> str:
    # Prefer AtCoder task screen name when possible.
    m = re.search(r"/tasks/([^/?#]+)", url)
    if m:
        return m.group(1)

    # Fallback: last path component.
    last = url.rstrip("/").split("/")[-1] or "problem"
    safe = re.sub(r"[^A-Za-z0-9_.-]+", "_", last)
    return safe or "problem"


def _load_oj_cookiejar() -> http.cookiejar.LWPCookieJar | None:
    # Typical locations:
    # - macOS: ~/Library/Application Support/online-judge-tools/cookie.jar
    # - Linux: ~/.local/share/online-judge-tools/cookie.jar
    candidates = [
        Path.home() / "Library/Application Support/online-judge-tools/cookie.jar",
        Path.home() / ".local/share/online-judge-tools/cookie.jar",
    ]
    for path in candidates:
        if path.exists():
            jar = http.cookiejar.LWPCookieJar(str(path))
            jar.load(ignore_discard=True, ignore_expires=True)
            return jar
    return None


def _download_samples(url: str, workdir: Path) -> None:
    test_dir = workdir / "test"
    test_dir.mkdir(parents=True, exist_ok=True)
    # Keep user-added tests; only clear sample-* files so `oj d` can re-run.
    for p in sorted(test_dir.glob("sample-*")):
        if p.is_file():
            p.unlink()

    # Require `oj` in PATH (recommended to run via `uv run ...`).
    subprocess.run(["oj", "d", url], cwd=workdir, check=True)


def _fetch_statement_text(url: str) -> str:
    session = requests.Session()
    jar = _load_oj_cookiejar()
    if jar is not None:
        session.cookies = jar

    resp = session.get(url, timeout=20)
    resp.raise_for_status()

    soup = BeautifulSoup(resp.text, "html.parser")
    node = soup.select_one("#task-statement")
    if node is None:
        # Fallback: keep the entire HTML for debugging.
        return resp.text
    return node.get_text("\n", strip=True)


def _ensure_template(workdir: Path) -> None:
    path = workdir / "main.py"
    if path.exists():
        return

    path.write_text(
        "\n".join(
            [
                "import sys",
                "",
                "",
                "def main() -> None:",
                "    # TODO: implement solution here.",
                "    #",
                "    # Common fast input pattern:",
                "    #   it = iter(sys.stdin.buffer.read().split())",
                "    #   n = int(next(it))",
                "    #   ...",
                "    pass",
                "",
                "",
                "if __name__ == \"__main__\":",
                "    main()",
                "",
            ]
        ),
        encoding="utf-8",
    )


def _infer_submit_url(url: str) -> str | None:
    m = re.search(r"^https://atcoder\\.jp/contests/([^/]+)/tasks/([^/?#]+)", url)
    if not m:
        return None
    contest_id, task = m.group(1), m.group(2)
    return f"https://atcoder.jp/contests/{contest_id}/submit?taskScreenName={task}"


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("url")
    parser.add_argument("--dir", dest="dir_", default=None, help="directory name (default: infer from URL)")
    args = parser.parse_args(argv)

    url: str = args.url
    dir_name = args.dir_ or _infer_dir_from_url(url)
    workdir = Path(dir_name)
    workdir.mkdir(parents=True, exist_ok=True)

    try:
        _download_samples(url, workdir)
    except FileNotFoundError:
        print("error: `oj` not found in PATH. Run this via `uv run python ...`.", file=sys.stderr)
        return 2
    except subprocess.CalledProcessError:
        # Some tasks (e.g. interactive/output-only) don't have downloadable samples.
        # Even in such cases, we still want to fetch the statement and prepare the directory.
        print("warn: failed to download samples via `oj d` (maybe no samples). continue.", file=sys.stderr)

    statement = _fetch_statement_text(url)
    (workdir / "statement.txt").write_text(statement + "\n", encoding="utf-8")

    _ensure_template(workdir)

    submit_url = _infer_submit_url(url)
    print(f"prepared: {workdir}")
    print("next:")
    print(f"  cd {workdir}")
    print('  uv run oj t -c "python main.py"')
    if submit_url:
        print(f"  submit (manual): {submit_url}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
