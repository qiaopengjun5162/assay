/* 数证 DataProof —— 静态站逻辑（无依赖、无构建） */

const $ = (s, el = document) => el.querySelector(s);
const $$ = (s, el = document) => Array.from(el.querySelectorAll(s));

const esc = (s) => String(s ?? "").replace(/[&<>"']/g, (c) => ({
  "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
}[c]));

const pct = (x) => (x == null ? "—" : (x * 100).toFixed(1) + "%");

async function loadData() {
  const r = await fetch("data.json", { cache: "no-store" });
  if (!r.ok) throw new Error("读不到 data.json（请用 just serve 启动本地服务）");
  return r.json();
}

const STOPPED = new Set(["-1", "999", "不更新"]);

/* ---------- 单数据集判定 ---------- */
function buildDims(d) {
  const previewMissing = !d.hasPreview;
  const emptyRate = d.emptyRate ?? 0;
  const schemaBad = d.schemaKind === "列数不一致" || d.schemaKind === "字段集合不一致";
  const descBad = (d.descLen ?? 0) < 10;

  return [
    {
      name: "规范性", std: "GB/T 36344 0101",
      value: descBad ? "说明不足" : "有说明",
      status: descBad ? "bad" : "ok",
      note: `字段说明 ${d.descLen ?? 0} 字` + (d.schemaKind === "仅命名不一致" ? "；声明名带前缀" : ""),
    },
    {
      name: "完整性", std: "GB/T 36344 02",
      value: d.sampleRows ? pct(emptyRate) : "无样例",
      status: !d.sampleRows ? "warn" : emptyRate > 0.3 ? "bad" : emptyRate > 0.1 ? "warn" : "ok",
      note: d.sampleRows
        ? `样例 ${d.sampleRows} 行 × ${d.sampleCols} 列` + (d.allEmptyCols ? `；${d.allEmptyCols} 列整列为空` : "")
        : "平台未提供样例数据",
    },
    {
      name: "一致性", std: "GB/T 36344 04",
      value: d.schemaKind,
      status: schemaBad ? "bad" : d.schemaKind === "仅命名不一致" ? "warn" : d.schemaKind === "无样例" ? "warn" : "ok",
      note: `声明 ${d.declaredCols ?? 0} 列 / 样例 ${d.sampleCols ?? 0} 列`,
    },
    {
      name: "时效性", std: "GB/T 36344 05",
      value: STOPPED.has(String(d.updateCycle)) ? "停更" : d.lagDays != null ? `${d.lagDays} 天未更新` : "—",
      status: STOPPED.has(String(d.updateCycle)) ? "bad" : d.overdue ? "bad" : (d.lagDays ?? 0) > 365 ? "warn" : "ok",
      note: `更新周期 ${d.updateCycle ?? "—"}${d.updateDetail ? " · " + d.updateDetail : ""}` +
            (d.updateTime ? `；最后更新 ${d.updateTime.slice(0, 10)}` : ""),
    },
    {
      name: "可访问性", std: "GB/T 36344 06",
      value: d.hasPreview ? "可预览" : "仅目录",
      status: d.hasPreview ? "ok" : "warn",
      note: d.hasPreview ? "表头与样例可直接读取" : "无预览，需申请或用接口",
    },
    {
      name: "准确性", std: "GB/T 36344 03",
      value: "不评价",
      status: "na",
      note: "没有权威数据源可比对，评了就是编",
    },
  ];
}

function verdictOf(d) {
  const dims = buildDims(d).filter((x) => x.status !== "na");
  const bad = dims.filter((x) => x.status === "bad").length;
  const warn = dims.filter((x) => x.status === "warn").length;
  if (bad >= 2) return { cls: "", label: "问题较多", why: `${bad} 项指标不达标，${warn} 项需注意` };
  if (bad === 1) return { cls: "", label: "有明确问题", why: `1 项指标不达标，${warn} 项需注意` };
  if (warn >= 1) return { cls: "good", label: "总体可用", why: `${warn} 项需注意，无硬伤` };
  return { cls: "good", label: "各项正常", why: "五项可计算指标均通过" };
}

/* ---------- 榜单页 ---------- */
async function initIndex() {
  const data = await loadData();
  const s = data.summary;
  const all = data.datasets;

  $("#meta").textContent =
    `数据抓取 ${data.generatedAt} · 全量 ${s.audited.toLocaleString()} 个数据集 · 免登录接口`;
  $("#stat-total").textContent = s.audited.toLocaleString();
  $("#stat-lag").textContent = ((s.lagOver90Days / s.audited) * 100).toFixed(0) + "%";
  $("#stat-lag-n").textContent = s.lagOver90Days.toLocaleString();
  $("#stat-empty").textContent = s.emptyRateOver30.toLocaleString();
  $("#stat-allcols").textContent = s.allEmptyColumnDatasets.toLocaleString();
  $("#stat-schema").textContent = (s.mismatchKinds["列数不一致"] || 0).toLocaleString();

  const depts = [...new Set(all.map((d) => d.dept).filter(Boolean))].sort();
  const sel = $("#dept");
  depts.forEach((d) => {
    const o = document.createElement("option");
    o.value = d; o.textContent = d;
    sel.appendChild(o);
  });

  const state = { q: "", dept: "", stopped: false, empty30: false, schema: false, sort: "score" };

  const apply = () => {
    let rows = all.filter((d) => {
      if (state.q && !(d.name + d.dept).toLowerCase().includes(state.q.toLowerCase())) return false;
      if (state.dept && d.dept !== state.dept) return false;
      if (state.stopped && !STOPPED.has(String(d.updateCycle))) return false;
      if (state.empty30 && !((d.emptyRate ?? 0) > 0.3)) return false;
      if (state.schema && d.schemaKind !== "列数不一致" && d.schemaKind !== "字段集合不一致") return false;
      return true;
    });
    if (state.sort === "score") rows.sort((a, b) => b.score - a.score);
    if (state.sort === "empty") rows.sort((a, b) => (b.emptyRate ?? 0) - (a.emptyRate ?? 0));
    if (state.sort === "lag") rows.sort((a, b) => (b.lagDays ?? -1) - (a.lagDays ?? -1));
    if (state.sort === "name") rows.sort((a, b) => a.name.localeCompare(b.name, "zh"));

    $("#list").innerHTML = rows.map((d, i) => {
      const chips = [];
      const lag = d.lagDays;
      if (STOPPED.has(String(d.updateCycle))) chips.push(`<span class="chip bad">停更</span>`);
      else if (lag != null) chips.push(`<span class="chip ${d.overdue ? "bad" : lag > 365 ? "warn" : ""}">${lag} 天未更</span>`);
      if ((d.emptyRate ?? 0) > 0) chips.push(`<span class="chip ${d.emptyRate > 0.3 ? "bad" : d.emptyRate > 0.1 ? "warn" : ""}">空值 ${pct(d.emptyRate)}</span>`);
      if (d.allEmptyCols) chips.push(`<span class="chip bad">${d.allEmptyCols} 列全空</span>`);
      if (d.schemaKind === "列数不一致") chips.push(`<span class="chip bad">列数 ${d.declaredCols}→${d.sampleCols}</span>`);
      if (d.proof) chips.push(`<span class="chip ok">已证</span>`);
      return `
        <a class="row" href="detail.html?id=${encodeURIComponent(d.id)}">
          <div class="rank">${i + 1}</div>
          <div>
            <div class="name">${esc(d.name)}</div>
            <div class="sub">${esc(d.dept || "未知部门")}<span class="dot">·</span>${esc(d.openType === "A" ? "无条件开放" : "受限开放")}<span class="dot">·</span>问题分 ${d.score.toFixed(1)}</div>
          </div>
          <div class="chips">${chips.join("")}</div>
        </a>`;
    }).join("") || `<div class="empty">没有符合条件的数据集</div>`;

    $("#count").textContent = `显示 ${rows.length} / ${all.length} 条（按问题分排序的是全量 24,322 条中最差的 ${all.length} 个）`;
  };

  $("#q").addEventListener("input", (e) => { state.q = e.target.value.trim(); apply(); });
  $("#dept").addEventListener("change", (e) => { state.dept = e.target.value; apply(); });
  $("#sort").addEventListener("change", (e) => { state.sort = e.target.value; apply(); });
  $("#f-stopped").addEventListener("change", (e) => { state.stopped = e.target.checked; apply(); });
  $("#f-empty").addEventListener("change", (e) => { state.empty30 = e.target.checked; apply(); });
  $("#f-schema").addEventListener("change", (e) => { state.schema = e.target.checked; apply(); });
  apply();
}

/* ---------- 详情页 ---------- */
async function initDetail() {
  const data = await loadData();
  const id = new URLSearchParams(location.search).get("id");
  const d = data.datasets.find((x) => x.id === id);
  if (!d) {
    $("#body").innerHTML = `<div class="panel"><h2>找不到这个数据集</h2><p class="hint">链接可能来自另一份 data.json，或 id 被截断。</p><p><a class="btn" href="index.html">回到榜单</a></p></div>`;
    return;
  }
  document.title = `${d.name} · 数证 DataProof`;
  $("#title").textContent = d.name;
  $("#meta-chips").innerHTML = [
    `<span class="chip">${esc(d.dept || "未知部门")}</span>`,
    `<span class="chip">${d.openType === "A" ? "A 类 · 无条件开放" : "B 类 · 受限开放"}</span>`,
    `<span class="chip mono">问题分 ${d.score.toFixed(1)}</span>`,
    d.proof ? `<span class="chip ok">已生成证明</span>` : "",
  ].join("");

  const v = verdictOf(d);
  $("#verdict").className = "verdict " + v.cls;
  $("#verdict").innerHTML = `
    <div class="big">${v.label}</div>
    <div class="why">${v.why}。问题分只用于排序，不是科学指标；口径见下方。</div>`;

  $("#dims").innerHTML = buildDims(d).map((x) => `
    <div class="dim ${x.status === "na" ? "na" : ""}">
      <h3>${x.name}<span class="std">${x.std}</span></h3>
      <div class="val" style="color:${x.status === "bad" ? "var(--bad)" : x.status === "warn" ? "var(--warn)" : x.status === "ok" ? "var(--ok)" : "var(--muted)"}">${esc(x.value)}</div>
      <div class="note">${esc(x.note)}</div>
    </div>`).join("");

  /* 字段一致性的明细 */
  const issue = [];
  if (d.schemaKind === "列数不一致") {
    issue.push(`官方字段清单 ${d.declaredCols} 列，样例实际 ${d.sampleCols} 列 —— 二者对不上。`);
  }
  if (d.missingFields.length) {
    issue.push(`声明有、样例里没有的字段（${d.missingFields.length}）：${d.missingFields.slice(0, 12).map(esc).join("、")}${d.missingFields.length > 12 ? " …" : ""}`);
  }
  if (d.extraFields.length) {
    issue.push(`样例里有、声明里没有的字段（${d.extraFields.length}）：${d.extraFields.slice(0, 12).map(esc).join("、")}${d.extraFields.length > 12 ? " …" : ""}`);
  }
  if (!d.hasPreview) issue.push("平台未提供样例数据，完整性只能判为「未知」。");
  $("#issues").innerHTML = issue.length ? `<ul>${issue.map((x) => `<li>${x}</li>`).join("")}</ul>`
    : `<p class="hint">未发现字段层面的不一致。</p>`;

  /* AI 解读（数字护栏） */
  if (d.explain && d.explain.text) {
    const ok = d.explain.status === "pass";
    $("#explain").innerHTML = `
      <h2>AI 解读 <span class="chip ${ok ? "ok" : "warn"}">${ok ? "数字已核对" : "未通过核对"}</span></h2>
      <p style="font-size:15px;margin:8px 0 4px">${esc(d.explain.text)}</p>
      <p class="hint">这段文字由模型生成，只允许使用报告里的数字：输出中的每个数字都会和事实逐个比对，对不上就重写；重写仍不合格就不展示。模型不参与取数。</p>`;
  } else {
    $("#explain").innerHTML = `
      <h2>AI 解读</h2>
      <p class="hint">这份数据集还没有生成解读。当前为问题最多的前 ${data.summary.explainCount || 0} 个数据集生成了带数字护栏的解读。</p>`;
  }

  /* 证据块 */
  if (d.evidence) {
    const files = (d.evidence.files || []).map((f) =>
      `<a class="btn" href="${esc(f)}" download>${esc(f.split("/").pop())}</a>`).join(" ");
    $("#evidence").innerHTML = `
      <h2>证据包</h2>
      <p class="hint">平台两个接口的原始响应，逐字节保存后哈希。全程确定性抓取，不经过任何模型。</p>
      <table class="kv">
        <tr><th>抓取时间</th><td>${esc((d.evidence.fetchedAt || "").replace("T", " ").slice(0, 19))}</td></tr>
        <tr><th>SHA-256</th><td>${esc(d.evidence.sha256)}</td></tr>
        <tr><th>响应大小</th><td>detail ${d.evidence.detailBytes} 字节 / preview ${d.evidence.previewBytes} 字节</td></tr>
      </table>
      <p>${files || '<span class="hint">原始响应未随站点分发</span>'}</p>
      <p class="hint">为什么不直接抓网页：实测平台详情页返回空文档，模型仍会「抄」出并不存在的数字。存证只能走确定性抓取。</p>`;
  } else {
    $("#evidence").innerHTML = `
      <h2>证据包</h2>
      <p class="hint">当前只为问题最多的前 ${data.summary.evidenceCount} 个数据集保存了原始响应快照。</p>`;
  }

  /* 证明块 */
  if (d.proof) {
    $("#proof").innerHTML = `
      <div class="proof-head">
        <div class="pass-stamp">PASS</div>
        <div>
          <div style="font-weight:700">这份报告的数值结论已由 Lean 4 内核复核</div>
          <div class="hint">样例 ${d.proof.rows} 行 × ${d.proof.cols} 列，${d.proof.cols} 条定理；改任一数字，内核报错。</div>
        </div>
      </div>
      <table class="kv">
        <tr><th>快照 SHA-256</th><td>${esc(d.proof.sha256)}</td></tr>
        <tr><th>校验命令</th><td>lean TopProofs.lean</td></tr>
      </table>
      <p><a class="btn" href="proof.html?id=${encodeURIComponent(d.id)}">查看证明详情 →</a></p>`;
  } else {
    $("#proof").innerHTML = `
      <h2>证明</h2>
      <p class="hint">这份数据集的证明尚未生成。当前为问题最多的前 ${data.summary.proofCount} 个数据集生成了证明；扩到全量只需改一个参数。</p>
      <p class="hint">说明：证明覆盖的是「从样例快照推出的计数与比率」，不包括平台数据本身的真伪。</p>`;
  }
}

/* ---------- 证明页 ---------- */
async function initProof() {
  const data = await loadData();
  const id = new URLSearchParams(location.search).get("id");
  const d = data.datasets.find((x) => x.id === id);
  if (!d || !d.proof) {
    $("#body").innerHTML = `<div class="panel"><h2>这份数据集没有证明</h2><p class="hint">当前只为问题最多的前 20 个数据集生成证明。</p><p><a class="btn" href="index.html">回到榜单</a></p></div>`;
    return;
  }
  const p = d.proof;
  document.title = `证明 · ${d.name}`;
  $("#name").textContent = d.name;
  $("#sha").textContent = p.sha256;
  $("#shape").textContent = `${p.rows} 行 × ${p.cols} 列`;
  $("#count").textContent = `${p.cols} 条定理`;
  $("#cmd").textContent = "lean TopProofs.lean";
  $("#theorem").textContent =
    `def countEmpty (rs : List (List Nat)) (col : Nat) : Nat :=\n` +
    `  (rs.filter (fun r => r.getD col 0 == 1)).length\n\n` +
    `theorem empty_${p.index}_0 : countEmpty rows_${p.index} 0 = 0 := by decide`;
}
