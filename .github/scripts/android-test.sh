#!/usr/bin/env bash
# Runs each cargo test binary listed in $1 on the connected Android device.
# The tests read golden vectors and scan sources, so core/ (without target/)
# is copied over too and PROXIM_CORE_DIR points them at it.
set -euo pipefail
list=$1
dev=/data/local/tmp/proxim

stage=$(mktemp -d)
tar -C core --exclude=./target -cf - . | tar -xf - -C "$stage"
adb shell "rm -rf $dev && mkdir -p $dev/bin"
adb push "$stage" "$dev/core" > /dev/null
adb shell getprop ro.product.cpu.abi
adb shell getprop ro.build.version.release

failed=()
total=0
while read -r exe; do
  name=$(basename "$exe")
  adb push "$exe" "$dev/bin/$name" > /dev/null
  echo "::group::$name"
  set +e
  adb shell "cd $dev && PROXIM_CORE_DIR=$dev/core ./bin/$name" | tee out.txt
  rc=${PIPESTATUS[0]}
  set -e
  echo "::endgroup::"
  n=$(sed -n 's/^test result: [A-Z]*\. \([0-9]*\) passed.*/\1/p' out.txt | tr -d '\r')
  total=$((total + ${n:-0}))
  if [ "$rc" -ne 0 ]; then failed+=("$name (exit $rc)"); fi
  adb shell rm "$dev/bin/$name"
done < "$list"

echo "Android: $total tests passed across $(wc -l < "$list") binaries"
if [ ${#failed[@]} -gt 0 ]; then
  printf 'FAILED: %s\n' "${failed[@]}"
  exit 1
fi
