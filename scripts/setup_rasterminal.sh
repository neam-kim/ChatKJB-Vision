#!/bin/sh
set -eu
python3 -m pip install --user trimesh numpy
dir=${1:-vendor/rasterminal}
test -d "$dir/.git" || git clone https://github.com/PavolUlicny/rasterminal.git "$dir"
git -C "$dir" checkout 4dc56a378eb0094e2ad5991b3e0bce08587d0125db
git -C "$dir" apply "$(dirname "$0")/rasterminal-reload.patch" 2>/dev/null || true
cmake -S "$dir" -B "$dir/build" -DCMAKE_BUILD_TYPE=Release
cmake --build "$dir/build" --config Release
