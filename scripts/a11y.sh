#!/usr/bin/env bash
# 无障碍复查：对四个视图跑 axe-core（WCAG 2.1 A/AA + best-practice）。
#
# 为什么不在 CI 跑：GitHub 的 rust job 里没有浏览器，装 Chromium 要额外几百 MB，
# 为一条前端规则拖慢整条流水线不划算。这是**交付前本地门禁**，和「按用户的路径走一遍」配套。
#
# 用法：just a11y          （或 scripts/a11y.sh）
#       CHROME=/path/to/chrome scripts/a11y.sh
# 退出码：0 = 四个视图 violations 均为 0；非 0 = 有违规或扫描没跑完。
set -euo pipefail

PORT="${PORT:-8098}"
DIST="$(cd "$(dirname "$0")/.." && pwd)/crates/web/dist"
AXE_CACHE="${XDG_CACHE_HOME:-$HOME/.cache}/assay-axe"
AXE_VER="4.10.2"

if [ ! -f "$DIST/index.html" ]; then
  echo "先构建前端：just web" >&2
  exit 2
fi

# 1) 找浏览器。桌面 Chrome headless 有 500px 最小窗口宽，这里只扫桌面宽度。
if [ -z "${CHROME:-}" ]; then
  for c in \
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
    "$(command -v google-chrome || true)" \
    "$(command -v chromium || true)" \
    "$(command -v chromium-browser || true)"; do
    [ -n "$c" ] && [ -x "$c" ] && CHROME="$c" && break
  done
fi
[ -n "${CHROME:-}" ] || { echo "没找到 Chrome/Chromium，用 CHROME=... 指定" >&2; exit 2; }

# 2) axe-core 只下一次，缓存在本地
mkdir -p "$AXE_CACHE"
if [ ! -s "$AXE_CACHE/axe.min.js" ]; then
  echo "下载 axe-core $AXE_VER → $AXE_CACHE"
  curl -sSL --max-time 60 -o "$AXE_CACHE/axe.min.js" \
    "https://cdn.jsdelivr.net/npm/axe-core@$AXE_VER/axe.min.js"
fi
cp "$AXE_CACHE/axe.min.js" "$DIST/_axe.min.js"

# 3) 探针页：把 app 放进 iframe 里跑，axe 在 iframe 文档上执行（同源）
cat > "$DIST/_a11y.html" <<'HTML'
<!doctype html><meta charset="utf-8"><title>a11y</title>
<body style="margin:0">
<iframe id="f" style="width:1200px;height:900px;border:0"></iframe>
<pre id="o" style="font:12px/1.5 monospace;white-space:pre-wrap"></pre>
<script>
const ID = 'HuZoVx_QGmxtyEgbHu_Ulw%3D%3D';
const D = '%E5%AE%81%E6%B3%A2%E5%B8%82%E7%BB%9F%E8%AE%A1%E5%B1%80';  // 宁波市统计局
const MAP = {list:['/','榜单'],checks:['/#/checks','判定规则'],
  detail:['/#/detail/'+ID,'详情'],proof:['/#/proof/'+ID,'证明'],
  dept:['/#/dept/'+D,'部门']};
const [path,name] = MAP[new URLSearchParams(location.search).get('r')||'list'];
const f = document.getElementById('f');
const sleep = ms => new Promise(r => setTimeout(r,ms));
f.onload = async () => {
  const doc = f.contentDocument;
  for (let i=0;i<40;i++){ await sleep(250); if (doc.querySelector('h1')) break; }
  await new Promise((res,rej)=>{const s=doc.createElement('script');
    s.src='/_axe.min.js'; s.onload=res; s.onerror=rej; doc.head.appendChild(s);});
  await sleep(400);
  const r = await doc.defaultView.axe.run(doc,{runOnly:{type:'tag',
    values:['wcag2a','wcag2aa','wcag21a','wcag21aa','best-practice']}});
  const out = ['===== '+name+' '+path, 'violations='+r.violations.length];
  for (const v of r.violations){
    out.push('- ['+v.impact+'] '+v.id+': '+v.help+' ('+v.nodes.length+' 处)');
    for (const n of v.nodes.slice(0,3)){
      out.push('    target='+n.target.join(' '));
      out.push('    html='+(n.html||'').slice(0,140));
    }
  }
  for (const v of r.incomplete){
    out.push('~ [incomplete] '+v.id+' x'+v.nodes.length);
    for (const n of v.nodes.slice(0,3)) out.push('    target='+n.target.join(' '));
  }
  // 渲染残留：Leptos 里写成 "占全量 {share}" 会把花括号原样输出，
  // 编译不报错、页面也不报错，只有人眼能发现。这里扫一遍。
  const stray = doc.body.innerText.match(/\{[a-zA-Z_][\w.]*\}/g) || [];
  out.push('stray_placeholders='+stray.length+(stray.length?' '+JSON.stringify(stray.slice(0,3)):''));
  out.push('===== DONE');
  document.getElementById('o').textContent = out.join('\n');
};
f.src = path;
</script>
HTML

cleanup() {
  rm -f "$DIST/_a11y.html" "$DIST/_axe.min.js"
  [ -n "${SRV_PID:-}" ] && kill "$SRV_PID" 2>/dev/null || true
}
trap cleanup EXIT

python3 -m http.server "$PORT" --bind 127.0.0.1 --directory "$DIST" >/dev/null 2>&1 &
SRV_PID=$!
for _ in $(seq 40); do
  curl -s --max-time 2 -o /dev/null "http://127.0.0.1:$PORT/_a11y.html" && break
  sleep 0.25
done

fail=0
for r in list checks detail proof dept; do
  dom="$("$CHROME" --headless=new --no-sandbox --disable-gpu \
        --virtual-time-budget=30000 --window-size=1300,1200 \
        --dump-dom "http://127.0.0.1:$PORT/_a11y.html?r=$r" 2>/dev/null \
        | sed -n '/<pre/,/<\/pre>/p' | sed 's/<[^>]*>//g')"
  echo "$dom"
  if ! echo "$dom" | grep -q '^violations=0$'; then
    fail=1
  fi
  if ! echo "$dom" | grep -q '^stray_placeholders=0'; then
    echo "!! $r 渲染出未替换的占位符（Leptos 里把 {var} 写进了字符串字面量）" >&2
    fail=1
  fi
  if ! echo "$dom" | grep -q '===== DONE'; then
    echo "!! $r 没跑完（页面没渲染出内容或浏览器提前退出）" >&2
    fail=1
  fi
done

if [ "$fail" -ne 0 ]; then
  echo
  echo "❌ 有无障碍违规，见上。" >&2
  exit 1
fi
echo
echo "✅ 五个视图 violations=0，无渲染残留。incomplete 项要人工确认，不能当通过。"
