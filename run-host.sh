#!/usr/bin/env bash
set -euo pipefail

profile=debug
build_cmd=(cargo build -p plugin-announcer)
run_cmd=(cargo run -p host)

if [[ "${1:-}" == "--release" ]]; then
    profile=release
    build_cmd+=(--release)
    run_cmd+=(--release)
    shift
fi

if [[ $# -ne 0 ]]; then
    echo "usage: $0 [--release]" >&2
    exit 2
fi

case "$(uname -s)" in
    Darwin)
        lib_name="libplugin_announcer.dylib"
        ;;
    Linux|*BSD)
        lib_name="libplugin_announcer.so"
        ;;
    MINGW*|MSYS*|CYGWIN*)
        lib_name="plugin_announcer.dll"
        ;;
    *)
        echo "unsupported platform: $(uname -s)" >&2
        exit 1
        ;;
esac

target_dir="${CARGO_TARGET_DIR:-target}"
plugin_path="${target_dir}/${profile}/deps/${lib_name}"

"${build_cmd[@]}"

if [[ ! -f "${plugin_path}" ]]; then
    echo "plugin dynamic library not found at ${plugin_path}" >&2
    exit 1
fi

"${run_cmd[@]}" -- "${plugin_path}"
