#!/bin/bash
# Remove only harness-owned temporary roots, including portal FUSE mounts.
remove_private_runtime() {
    local directory=$1 mount
    [[ $directory == /tmp/asena-preview.* || $directory == /tmp/asena-smoke.* || $directory == /tmp/asena-wayfire.* ]] || return 1
    [[ ${directory%/*} == /tmp && -d $directory && -O $directory && ! -L $directory ]] || return 1
    for mount in "$directory/runtime/doc" "$directory/runtime/gvfs"; do
        if mountpoint -q -- "$mount"; then fusermount3 -uz -- "$mount" || return 1; fi
    done
    rm -rf -- "$directory"
}
