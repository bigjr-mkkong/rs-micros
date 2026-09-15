#!/bin/sh

set -u

test_log=$(mktemp)
trap 'rm -f "$test_log"' EXIT

qemu_status=0
(
    sleep 1
    # Keep stdin open and retry the byte while QEMU transitions its serial
    # backend into raw mode; a one-shot pipe can race that transition.
    for _attempt in 1 2 3 4; do
        printf 'x'
        sleep 1
    done
) | timeout 8s "${QEMU}" \
    -machine "${MACH}" \
    -smp 4 \
    -cpu "${CPU}" \
    -m "${MEM}" \
    -display none \
    -monitor none \
    -serial stdio \
    -bios none \
    -kernel "${KERNEL}" \
    >"${test_log}" 2>&1 || qemu_status=$?

cat "${test_log}"

# The normal kernel intentionally keeps running, so timeout(1) status 124 is
# expected after the injected byte has been handled.
if { [ "${qemu_status}" -ne 0 ] && [ "${qemu_status}" -ne 124 ]; } || \
    ! grep -q 'Uart extint at CPU#0: x' "${test_log}" || \
    grep -q 'Unexpected M-mode' "${test_log}"; then
    echo "delegated UART test failed: qemu_status=${qemu_status}"
    exit 1
fi

echo "delegated UART test passed: source 10 was handled through S-mode"
