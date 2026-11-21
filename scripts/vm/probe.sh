#!/bin/bash
# probe.sh '<js function body>' : runs it in the window, prints the result.
S=${XKYVM_SCRATCH:-$(mktemp -d)}; n=$(date +%s%N)
printf '%s' "$1" > "$S/p.js"
scp -q "$S/p.js" "xkyvm:C:/xkyvm/probe/$n.tmp" && ssh xkyvm "Rename-Item C:\\xkyvm\\probe\\$n.tmp $n.js" >/dev/null
for i in $(seq 1 ${2:-30}); do
  if ssh xkyvm "if (Test-Path C:\\xkyvm\\probe\\$n.json) { Get-Content -Raw C:\\xkyvm\\probe\\$n.json; Remove-Item C:\\xkyvm\\probe\\$n.json }" 2>/dev/null | grep -a . ; then exit 0; fi
  sleep 1
done
echo "no answer"
