#!/bin/bash
set -euo pipefail

has_nodefaultlibs=false
has_dead_strip=false
has_ort_archive=false
for argument in "$@"; do
    [[ "$argument" == "-nodefaultlibs" ]] && has_nodefaultlibs=true
    [[ "$argument" == "-Wl,-dead_strip" ]] && has_dead_strip=true
    [[ "$argument" == *libort_sys-* ]] && has_ort_archive=true
done

if [[ "$has_nodefaultlibs" == true && "$has_dead_strip" == true && "$has_ort_archive" == true ]]; then
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
