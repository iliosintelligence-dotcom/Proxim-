#!/usr/bin/env bash
# Runs each cargo test binary listed in $1 on the connected Android device.
# The tests read golden vectors and scan sources, so core/ (without target/)
# is copied over too and PROXIM_CORE_DIR points them at it.
set -euo pipefail
list=$1
dev=/data/local/tmp/proxim

stage=$(mktemp -d)
tar -C core --exclude=./target -cf - . | tar -xf - -C "$stage"
adb shell "rm -rf $dev && mkdir -p $dev/bin" < /dev/null
adb push "$stage" "$dev/core" > /dev/null < /dev/null
adb shell getprop ro.product.cpu.abi < /dev/null
adb shell getprop ro.build.version.release < /dev/null

failed=()
total=0
ran=0
# adb reads stdin: every adb call below gets </dev/null so it cannot eat the binary list.
while read -r exe; do
  name=$(basename "$exe")
  adb push "$exe" "$dev/bin/$name" > /dev/null < /dev/null
  echo "::group::$name"
  set +e
  adb shell "cd $dev && PROXIM_CORE_DIR=$dev/core ./bin/$name" < /dev/null | tee out.txt
  rc=${PIPESTATUS[0]}
  set -e
  echo "::endgroup::"
  n=$(tr -d '\r' < out.txt | sed -n 's/^test result: [A-Za-z]*\. \([0-9]*\) passed.*/\1/p')
  total=$((total + ${n:-0}))
  ran=$((ran + 1))
  if [ "$rc" -ne 0 ]; then failed+=("$name (exit $rc)"); fi
  adb shell rm "$dev/bin/$name" < /dev/null
done < "$list"

expected=$(wc -l < "$list")
echo "Android: $total tests passed across $ran of $expected binaries"
if [ "$ran" -ne "$expected" ] || [ "$total" -eq 0 ]; then
  echo "FAILED: not every test binary ran, or no test passed"
  exit 1
fi
if [ ${#failed[@]} -gt 0 ]; then
  printf 'FAILED: %s\n' "${failed[@]}"
  exit 1
fi
