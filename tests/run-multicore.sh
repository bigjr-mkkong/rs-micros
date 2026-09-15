#!/bin/sh

set -u

test_log=$(mktemp)
trap 'rm -f "$test_log"' EXIT

qemu_status=0
timeout 30s "${QEMU}" \
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

ktask_records=$(awk '
    { sub(/\r$/, "") }
    /^\[KTASK-STRESS\] hart=[0-3] pid=[0-9]+ iter=[0-9]+$/ { count++ }
    END { print count + 0 }
' "${test_log}")

semaphore_records=$(awk '
    { sub(/\r$/, "") }
    /^\[KSEM-STRESS\] hart=[0-3] pid=[0-9]+ wake=[0-9]+$/ { count++ }
    END { print count + 0 }
' "${test_log}")

ktask_distribution_ok=1
semaphore_distribution_ok=1
for hart in 0 1 2 3; do
    hart_ktask_records=$(awk -v hart="${hart}" '
        { sub(/\r$/, "") }
        $0 ~ "^\\[KTASK-STRESS\\] hart=" hart " " { count++ }
        END { print count + 0 }
    ' "${test_log}")
    hart_semaphore_records=$(awk -v hart="${hart}" '
        { sub(/\r$/, "") }
        $0 ~ "^\\[KSEM-STRESS\\] hart=" hart " " { count++ }
        END { print count + 0 }
    ' "${test_log}")

    [ "${hart_ktask_records}" -eq 32 ] || ktask_distribution_ok=0
    [ "${hart_semaphore_records}" -eq 16 ] || semaphore_distribution_ok=0
done

if [ "${qemu_status}" -ne 0 ] || \
    [ "${ktask_records}" -ne 128 ] || \
    [ "${semaphore_records}" -ne 64 ] || \
    [ "${ktask_distribution_ok}" -ne 1 ] || \
    [ "${semaphore_distribution_ok}" -ne 1 ] || \
    ! grep -Eq '\[MULTICORE-TEST\] PASS .*machine_timer_ticks=[1-9][0-9]*' "${test_log}"; then
    echo "multicore test failed: qemu_status=${qemu_status} ktask_uart=${ktask_records}/128 semaphore_uart=${semaphore_records}/64"
    exit 1
fi

echo "multicore test passed: 128 ktask and 64 semaphore UART records were intact"
