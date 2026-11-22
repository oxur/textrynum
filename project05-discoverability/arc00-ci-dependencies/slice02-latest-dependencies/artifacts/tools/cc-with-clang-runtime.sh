#!/bin/bash
set -euo pipefail

inject_runtime=false
for argument in "$@"; do
    if [[ "$argument" == "-nodefaultlibs" ]]; then
        has_nodefaultlibs=true
    fi
    if [[ "$argument" == "-Wl,-dead_strip" ]]; then
        has_dead_strip=true
    fi
done
if [[ "${has_nodefaultlibs:-false}" == true && "${has_dead_strip:-false}" == true ]]; then
    inject_runtime=true
fi

if [[ "$inject_runtime" == true ]]; then
    resource_dir=$(/usr/bin/xcrun clang --print-resource-dir)
    runtime="$resource_dir/lib/darwin/libclang_rt.osx.a"
    if [[ ! -r "$runtime" ]]; then
        echo "clang runtime archive is unavailable: $runtime" >&2
        exit 1
    fi
    if [[ -n "${CC_WRAPPER_LOG:-}" ]]; then
        {
            printf 'REAL_COMPILER=/usr/bin/cc\nRUNTIME=%s\nARGV=' "$runtime"
            printf ' %q' "$@"
            printf ' %q\n' "$runtime"
        } >> "$CC_WRAPPER_LOG"
    fi
    exec /usr/bin/cc "$@" "$runtime"
fi
exec /usr/bin/cc "$@"
