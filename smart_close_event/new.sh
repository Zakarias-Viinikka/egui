#!/usr/bin/env bash
set -e

read -p "name of new project: " name

if [ -z "$name" ]; then
    echo "no name given"
    exit 1
fi

src="$(cd "$(dirname "$0")" && pwd)"
dest="$(dirname "$src")/$name"

if [ -e "$dest" ]; then
    echo "$dest already exists"
    exit 1
fi

mkdir -p "$dest"
tar -C "$src" --exclude=target --exclude=.git --exclude=Cargo.lock -cf - . | tar -C "$dest" -xf -

echo "created $dest"
