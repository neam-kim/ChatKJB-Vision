#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
venv="$root/.venv"
patch="$root/scripts/rasterminal-reload.patch"
dir=${1:-"$root/vendor/rasterminal"}
case "$dir" in /*) ;; *) dir="$root/$dir" ;; esac

python3 -m venv "$venv"
"$venv/bin/python" -m pip install -q trimesh numpy scipy

test -d "$dir/.git" || git clone https://github.com/PavolUlicny/rasterminal.git "$dir"
git -C "$dir" checkout --detach 4dc56a378eb0094e2ad5991b3e0bce08580125db
if git -C "$dir" apply --reverse --check "$patch" 2>/dev/null; then
    echo "rasterminal patch already applied"
else
    git -C "$dir" apply --check "$patch"
    git -C "$dir" apply "$patch"
fi

cmake -S "$dir" -B "$dir/build" -DCMAKE_BUILD_TYPE=Release
cmake --build "$dir/build" --config Release
