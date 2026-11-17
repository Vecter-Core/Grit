#!/bin/bash
# THỦ CÔNG (KHÔNG nằm trong cổng hay test mặc định): thử `spawn` nhiều luồng cùng sống để quan sát giới hạn tài nguyên (GÓI 3 S2.b(v)).
#   bash tools/stress_spawn.sh [K_tối_đa=200] [hồ_sơ=dev]
# Lý do tồn tại: lần thử 3000 luồng x 256 MiB (R11b, 2026-11-17) làm cạn tài nguyên máy (bash không fork được). Chốt chặn:
#  - K_tối_đa mặc định 200; lớn hơn 200 CHỈ khi đặt STRESS_ALLOW_MORE=1 (ghi rõ rủi ro); không bao giờ > 1000.
#  - chạy tools/preflight.sh trước (FAIL ⇒ thoát 2); trước MỖI bước, nếu bộ nhớ ảo trống (Windows) hoặc RAM khả dụng (Linux) < ngưỡng (mặc định 4 GB,
#    STRESS_MIN_FREE_GB) ⇒ DỪNG, không thử bước kế.
#  - mỗi bước chạy dưới `timeout 60`; bước tăng dần 10, 25, 50, 100, 200, …
# Kết quả: dòng `STRESS K=<n> rc=<mã> ...`. Chỉ ghi nhận cách tái hiện; không kết luận "an toàn".
cd "$(dirname "$0")/.." || exit 2
KMAX="${1:-200}"; PROFILE="${2:-dev}"
if [ "$KMAX" -gt 200 ] && [ "${STRESS_ALLOW_MORE:-0}" != "1" ]; then echo "STRESS: K>200 cần STRESS_ALLOW_MORE=1"; exit 3; fi
[ "$KMAX" -le 1000 ] || { echo "STRESS: K tối đa 1000"; exit 3; }
bash tools/preflight.sh "$(pwd)" || { echo "STRESS: preflight FAIL — từ chối chạy"; exit 2; }
MINGB="${STRESS_MIN_FREE_GB:-4}"
free_gb() {
  case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*) kb=$(powershell.exe -NoProfile -Command "(Get-CimInstance Win32_OperatingSystem).FreeVirtualMemory" 2>/dev/null | tr -d '\r\n '); echo $(( ${kb:-0} / 1048576 )) ;;
    *) kb=$(awk '/^MemAvailable:/ {print $2}' /proc/meminfo 2>/dev/null); echo $(( ${kb:-0} / 1048576 )) ;;
  esac
}
export PYTHONIOENCODING=utf-8
GRITC="${GRITC:-target/debug/gritc.exe}"; [ -x "$GRITC" ] || GRITC="target/debug/gritc"
[ -x "$GRITC" ] || { echo "STRESS: không thấy gritc (dựng: cargo build -p gt-driver)"; exit 3; }
TMPD="${TMPDIR:-/tmp}/grit_stress.$$"; mkdir -p "$TMPD"; trap 'rm -rf "$TMPD"' EXIT
for K in 10 25 50 100 200 400 800 1000; do
  [ "$K" -le "$KMAX" ] || break
  g=$(free_gb)
  if [ "$g" -lt "$MINGB" ]; then echo "STRESS: dừng trước K=$K: còn trống ${g} GB < ${MINGB} GB"; exit 4; fi
  cat > "$TMPD/stress_prog.gt" <<GT
■ UP: Atomic<u64> ≔ Atomic::new(0u64);
■ GO: Atomic<u64> ≔ Atomic::new(0u64);
■ DONE: Atomic<u64> ≔ Atomic::new(0u64);
ƒ worker(n: u32) ! ∅ {
  UP.fetch_add(1u64, AtomicOrdering::SeqCst);
  μs ≔ 0u64;
  ⟳ s < 3000000000u64 ∧ GO.load(AtomicOrdering::Acquire) < 1u64 ↓ 3000000000u64 - s { s ← s + 1u64; }
  DONE.fetch_add(1u64, AtomicOrdering::SeqCst);
}
ƒ main() → u64 ! ∅ {
  μi ≔ 0u32;
  ⟳ i < ${K}u32 ↓ ${K}u32 - i { spawn(worker, i); i ← i + 1u32; }
  μw ≔ 0u64;
  ⟳ w < 3000000000u64 ∧ UP.load(AtomicOrdering::Acquire) < ${K}u64 ↓ 3000000000u64 - w { w ← w + 1u64; }
  up ≔ UP.load(AtomicOrdering::SeqCst);
  GO.store(1u64, AtomicOrdering::Release);
  μd ≔ 0u64;
  ⟳ d < 3000000000u64 ∧ DONE.load(AtomicOrdering::Acquire) < ${K}u64 ↓ 3000000000u64 - d { d ← d + 1u64; }
  up
}
GT
  out=$(timeout 60 "$GRITC" run --backend=llvm --profile="$PROFILE" "$TMPD/stress_prog.gt" 2>&1); rc=$?
  echo "STRESS K=$K rc=$rc libre_gb_truoc=$g $(echo "$out" | tail -1 | cut -c1-100)"
  [ "$rc" = 0 ] && : || echo "$out" | head -8
  [ "$rc" = 0 ] || { echo "STRESS: bước K=$K không thành công — dừng (đây là giới hạn tài nguyên, xem KNOWN_LIMITATIONS)"; exit 0; }
done
echo "STRESS: xong tới K=$KMAX"
