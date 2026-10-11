#!/usr/bin/env bash
# Regenerates docs/CALIBRATION.md: both match engine tiers against
# real-football targets, on seeds the engine was not tuned on.
# Exits non-zero if any check is out of range.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --quiet --release -p fs-cli
out=docs/CALIBRATION.md
status=0
{
  echo "# Match engine calibration"
  echo
  echo "Each tier plays 40 fresh 20-club seasons (seeds 5000-5039; tuning used"
  echo "other seeds) and is compared with typical top-flight European league"
  echo "values. The tactic rows show season points gained per step of each"
  echo "slider; a large value would mean one style simply wins. The tactical"
  echo "choices table does the same for roles, duties and opposition"
  echo "instructions."
  echo
  echo "Regenerate with \`scripts/calibration-report.sh\`."
  echo
} > "$out"
for tier in 1 2; do
  ./target/release/fs-cli calibrate --tier "$tier" --seasons 40 --seed 5000 \
    | sed 's/^# Calibration:/## Calibration:/' >> "$out" || status=1
  echo >> "$out"
done
./target/release/fs-cli compare-tiers --runs 10000 >> "$out"
echo >> "$out"
./target/release/fs-cli choices --seasons 10 --seed 5000 >> "$out" || status=1
echo "wrote $out"
exit $status
