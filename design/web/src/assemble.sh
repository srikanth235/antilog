#!/bin/bash
# Assemble ../Main.dc.html from the parts in this folder. Run from anywhere.
set -e
D="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUT="$D/../Main.dc.html"
{
cat <<'HEAD'
<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Ratchet</title>
<script src="./support.js"></script>
</head>
<body>
<x-dc>
<helmet>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Instrument+Sans:wght@400;500;600&family=JetBrains+Mono:wght@400;500&display=swap">
<style>
HEAD
cat "$D/style.css" "$D/extra.css"
echo '</style>'
echo '</helmet>'
cat "$D/m1.html" "$D/m2.html" "$D/m3.html" "$D/m4.html" "$D/m5.html"
echo '</x-dc>'
echo '<script type="text/x-dc" data-dc-script data-props='"'"'{"theme":{"editor":"enum","options":["system","dark","light"],"default":"system"},"$preview":{"width":1440,"height":1000}}'"'"'>'
cat "$D/logic1.js" "$D/logic2.js" "$D/logic3.js"
echo '</script>'
echo '</body>'
echo '</html>'
} > "$OUT.tmp"
python3 "$D/strip.py" "$OUT.tmp" "$OUT"
rm "$OUT.tmp"
if command -v node >/dev/null; then
  CHK="$(mktemp).js"
  cat "$D/logic1.js" "$D/logic2.js" "$D/logic3.js" > "$CHK"
  node --check "$CHK" && echo "syntax ok"
  rm -f "$CHK"
fi
wc -c "$OUT"
