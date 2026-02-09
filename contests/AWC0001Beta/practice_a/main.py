import sys


def main() -> None:
    tokens = sys.stdin.read().split()
    a = int(tokens[0])
    b = int(tokens[1])
    c = int(tokens[2])
    s = tokens[3]
    print(a + b + c, s)


if __name__ == "__main__":
    main()
