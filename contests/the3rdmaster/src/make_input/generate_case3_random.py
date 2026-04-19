#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.9"
# dependencies = [
#   "pillow>=11.3.0",
# ]
# ///

from pathlib import Path
import random

from PIL import Image


N = 32
K = 2
C = 4
SEED = 20260418


def build_grid():
    rng = random.Random(SEED)
    return [[rng.randint(1, C) for _ in range(N)] for _ in range(N)]


def write_input(path, grid):
    with path.open("w", encoding="utf-8") as fh:
        fh.write(f"{N} {K} {C}\n")
        for row in grid:
            fh.write(" ".join(str(v) for v in row))
            fh.write("\n")


def write_output(path, grid):
    actions = 0
    with path.open("w", encoding="utf-8") as fh:
        for i, row in enumerate(grid):
            for j, value in enumerate(row):
                fh.write(f"0 0 {i} {j} {value}\n")
                actions += 1
    return actions


def vis_rgb(value):
    return {
        0: (0xF8, 0xFA, 0xFC),
        1: (0x25, 0x63, 0xEB),
        2: (0xEF, 0x44, 0x44),
        3: (0x10, 0xB9, 0x81),
        4: (0xF5, 0x9E, 0x0B),
    }[value]


def write_preview(path, grid, scale):
    img = Image.new("RGB", (N, N))
    for i, row in enumerate(grid):
        for j, value in enumerate(row):
            img.putpixel((j, i), vis_rgb(value))

    if scale != 1:
        resampling = getattr(Image, "Resampling", Image).NEAREST
        img = img.resize((N * scale, N * scale), resampling)
    img.save(path)


def main():
    base = Path("src/make_input")
    grid = build_grid()
    nonzero = sum(value != 0 for row in grid for value in row)
    if nonzero < (N * N) // 2:
        raise RuntimeError(f"non-zero pixels too few: {nonzero}")

    input_path = base / "case3_random_input.txt"
    output_path = base / "case3_random_output.txt"
    preview_path = base / "case3_random_preview.png"
    preview_10x_path = base / "case3_random_preview_10x.png"

    write_input(input_path, grid)
    actions = write_output(output_path, grid)
    write_preview(preview_path, grid, scale=1)
    write_preview(preview_10x_path, grid, scale=10)

    counts = {color: 0 for color in range(1, C + 1)}
    for row in grid:
        for value in row:
            counts[value] += 1

    print(f"seed={SEED}")
    print(f"generated: {input_path}")
    print(f"generated: {output_path}")
    print(f"generated: {preview_path}")
    print(f"generated: {preview_10x_path}")
    print(f"nonzero={nonzero}, actions={actions}, counts={counts}")


if __name__ == "__main__":
    main()
