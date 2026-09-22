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

python3 - "$dest/Cargo.toml" <<'PY'
import sys
p = sys.argv[1]
with open(p) as f:
    lines = f.readlines()
out = []
done = False
for line in lines:
    out.append(line)
    if line.strip() == 'resolver = "2"' and not done:
        out.append('default-members = ["app"]\n')
        done = True
if not done:
    raise SystemExit(f'resolver line not found in {p}')
with open(p, "w") as f:
    f.writelines(out)
PY

echo "created $dest"
