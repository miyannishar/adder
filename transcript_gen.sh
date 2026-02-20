#!/usr/bin/env bash
set -e
rm -f transcript.txt
for snek in test/*.snek; do
  base="${snek%.snek}"
  run="${base}.run"
  echo "=== $snek ===" | tee -a transcript.txt
  echo "--- Source ---" | tee -a transcript.txt
  cat "$snek" | tee -a transcript.txt
  echo "--- Compile ---" | tee -a transcript.txt
  make "$run" 2>&1 | tee -a transcript.txt
  echo "--- Assembly ---" | tee -a transcript.txt
  cat "${base}.s" | tee -a transcript.txt
  echo "--- Output ---" | tee -a transcript.txt
  "./$run" | tee -a transcript.txt
  echo "" | tee -a transcript.txt
done
