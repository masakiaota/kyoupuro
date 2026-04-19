#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.9"
# dependencies = [
#   "qrcode[pil]>=8.2",
# ]
# ///

from pathlib import Path

import qrcode
from PIL import Image


N = 32
K = 5
C = 2
QR_VERSION = 1
QR_BORDER = 4
QR_SIZE_WITH_BORDER = 21 + QR_BORDER * 2
PAYLOAD = "HTTPS://ATCODER.JP/"


def build_qr_matrix():
    qr = qrcode.QRCode(
        version=QR_VERSION,
        error_correction=qrcode.constants.ERROR_CORRECT_M,
        box_size=1,
        border=QR_BORDER,
    )
    qr.add_data(PAYLOAD)
    qr.make(fit=False)
    matrix = qr.get_matrix()
    if len(matrix) != QR_SIZE_WITH_BORDER or any(
        len(row) != QR_SIZE_WITH_BORDER for row in matrix
    ):
        raise RuntimeError(
            f"unexpected QR matrix size: {len(matrix)}x{len(matrix[0])}"
        )
    return matrix


def build_grid(matrix):
    top = (N - QR_SIZE_WITH_BORDER) // 2
    left = (N - QR_SIZE_WITH_BORDER) // 2
    grid = [[0 for _ in range(N)] for _ in range(N)]
    for i, row in enumerate(matrix):
        for j, dark in enumerate(row):
            grid[top + i][left + j] = 2 if dark else 1
    return grid, top, left


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
                if value == 0:
                    continue
                fh.write(f"0 0 {i} {j} {value}\n")
                actions += 1
    return actions


def write_preview(path, matrix, scale):
    img = Image.new("L", (N, N), 255)
    top = (N - QR_SIZE_WITH_BORDER) // 2
    left = (N - QR_SIZE_WITH_BORDER) // 2
    for i, row in enumerate(matrix):
        for j, dark in enumerate(row):
            img.putpixel((left + j, top + i), 0 if dark else 255)

    if scale != 1:
        resampling = getattr(Image, "Resampling", Image).NEAREST
        img = img.resize((N * scale, N * scale), resampling)
    img.save(path)


def main():
    base = Path("src/make_input")
    matrix = build_qr_matrix()
    grid, top, left = build_grid(matrix)
    nonzero = sum(value != 0 for row in grid for value in row)
    if nonzero < (N * N) // 2:
        raise RuntimeError(f"non-zero pixels too few: {nonzero}")

    input_path = base / "case2_qr_input.txt"
    output_path = base / "case2_qr_output.txt"
    preview_path = base / "case2_qr_preview.png"
    preview_10x_path = base / "case2_qr_preview_10x.png"

    write_input(input_path, grid)
    actions = write_output(output_path, grid)
    write_preview(preview_path, matrix, scale=1)
    write_preview(preview_10x_path, matrix, scale=10)

    print(f"payload={PAYLOAD}")
    print(f"generated: {input_path}")
    print(f"generated: {output_path}")
    print(f"generated: {preview_path}")
    print(f"generated: {preview_10x_path}")
    print(f"offset=({top},{left}), nonzero={nonzero}, actions={actions}")


if __name__ == "__main__":
    main()
