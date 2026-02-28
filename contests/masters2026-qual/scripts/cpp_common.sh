#!/usr/bin/env sh

detect_cxx() {
    if [ -n "${CXX:-}" ]; then
        printf "%s\n" "$CXX"
        return 0
    fi

    for cxx in g++ clang++ c++; do
        if command -v "$cxx" >/dev/null 2>&1; then
            printf "%s\n" "$cxx"
            return 0
        fi
    done

    return 1
}

detect_cpp_std_flag() {
    cxx_bin=$1
    tmp_bin=$(mktemp "${TMPDIR:-/tmp}/cppstd.XXXXXX")
    rm -f "$tmp_bin"

    for flag in -std=gnu++23 -std=c++23 -std=gnu++2b -std=c++2b; do
        if printf 'int main(){return 0;}\n' | "$cxx_bin" "$flag" -x c++ - -o "$tmp_bin" >/dev/null 2>&1; then
            rm -f "$tmp_bin"
            printf "%s\n" "$flag"
            return 0
        fi
    done

    rm -f "$tmp_bin"
    return 1
}

compile_cpp() {
    cxx_bin=$1
    std_flag=$2
    src_file=$3
    out_file=$4
    shift 4 || true

    "$cxx_bin" "$std_flag" -O2 -pipe -Wall -Wextra "$@" "$src_file" -o "$out_file"
}
