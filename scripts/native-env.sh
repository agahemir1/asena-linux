#!/bin/bash
# Source this file, including in launch scripts, to use the verified local prefix.
asena_native_repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
asena_native_prefix=$asena_native_repo/target/native/gtk4-layer-shell-1.3.0
if [[ -f $asena_native_prefix/lib/pkgconfig/gtk4-layer-shell-0.pc ]]; then
    case :${PKG_CONFIG_PATH:-}: in
        *:"$asena_native_prefix/lib/pkgconfig":*) ;;
        *) export PKG_CONFIG_PATH=$asena_native_prefix/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH} ;;
    esac
    case :${LD_LIBRARY_PATH:-}: in
        *:"$asena_native_prefix/lib":*) ;;
        *) export LD_LIBRARY_PATH=$asena_native_prefix/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH} ;;
    esac
fi
unset asena_native_prefix asena_native_repo
