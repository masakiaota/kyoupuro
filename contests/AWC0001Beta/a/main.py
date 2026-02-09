import sys


def main() -> None:
    k = int(sys.stdin.buffer.readline())
    # Sizes that can appear are exactly {2^0, 2^1, ..., 2^K}.
    print(k + 1)


if __name__ == "__main__":
    main()
