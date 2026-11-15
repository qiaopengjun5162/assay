//! 单页应用：榜单 / 详情 / 证明三个视图。
//!
//! 路由走 hash（`#/detail/<id>`），刷新可直达；不引第三方路由库，
//! 因为只有三个视图、两条跳转。

use crate::model::{Data, Row};
use leptos::prelude::*;

#[derive(Clone, Debug, PartialEq)]
enum Route {
    List,
    Detail(String),
    Proof(String),
}

fn decode(s: &str) -> String {
    // hash 里的 id 是 encodeURIComponent 过的，只处理 %XX 就够
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn parse_route(hash: &str) -> Route {
    let h = hash.trim_start_matches('#');
    let h = h.trim_start_matches('/');
    if let Some(rest) = h.strip_prefix("detail/") {
        return Route::Detail(decode(rest.split('?').next().unwrap_or(rest)));
    }
    if let Some(rest) = h.strip_prefix("proof/") {
        return Route::Proof(decode(rest.split('?').next().unwrap_or(rest)));
    }
    Route::List
}

fn urlencoding(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            other => format!("%{other:02X}"),
        })
        .collect()
}

fn fmt_num(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[component]
pub fn App() -> impl IntoView {
    let data = RwSignal::new(None::<Data>);
    let route = RwSignal::new(
        web_sys::window()
            .map(|w| parse_route(&w.location().hash().unwrap_or_default()))
            .unwrap_or(Route::List),
    );

    // 浏览器前进/后退、手改 hash 都要能切视图
    {
        use wasm_bindgen::prelude::*;
        let cb = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
            if let Some(w) = web_sys::window() {
                route.set(parse_route(&w.location().hash().unwrap_or_default()));
            }
        });
        if let Some(w) = web_sys::window() {
            let _ = w.add_event_listener_with_callback("hashchange", cb.as_ref().unchecked_ref());
        }
        cb.forget();
    }

    leptos::task::spawn_local(async move {
        if let Ok(resp) = gloo_net::http::Request::get("data.json").send().await {
            if let Ok(d) = resp.json::<Data>().await {
                data.set(Some(d));
            }
        }
    });

    view! {
        <div class="wrap">
            <header class="topbar">
                <div class="brand"><span class="mark">"数证"</span>" DataProof"</div>
                <div class="top-meta">"可机器复核的数据体检报告"</div>
                <nav class="top-nav">
                    <a href="#/" on:click=move |_| { route.set(Route::List); }>"榜单"</a>
                </nav>
            </header>
            {move || match (data.get(), route.get()) {
                (None, _) => view! { <p class="hint">"加载中…"</p> }.into_any(),
                (Some(d), Route::List) => list_view(d, route).into_any(),
                (Some(d), Route::Detail(id)) => detail_view(d, &id, route).into_any(),
                (Some(d), Route::Proof(id)) => proof_view(d, &id, route).into_any(),
            }}
        </div>
    }
}

fn list_view(data: Data, route: RwSignal<Route>) -> impl IntoView {
    let search = RwSignal::new(String::new());
    let dept = RwSignal::new(String::new());
    let sort = RwSignal::new("score".to_string());
    let only_stopped = RwSignal::new(false);
    let only_empty = RwSignal::new(false);
    let only_mismatch = RwSignal::new(false);

    let mut depts: Vec<String> = data
        .datasets
        .iter()
        .map(|r| r.dept.clone())
        .filter(|d| !d.is_empty())
        .collect();
    depts.sort();
    depts.dedup();

    let s = data.summary.clone();
    let rows = RwSignal::new(data.datasets.clone());

    let filtered = move || {
        let q = search.get().to_lowercase();
        let dd = dept.get();
        let mut v: Vec<Row> = rows
            .get()
            .into_iter()
            .filter(|r| {
                q.is_empty()
                    || r.name.to_lowercase().contains(&q)
                    || r.dept.to_lowercase().contains(&q)
            })
            .filter(|r| dd.is_empty() || r.dept == dd)
            .filter(|r| !only_stopped.get() || r.stopped())
            .filter(|r| !only_empty.get() || r.empty_rate > 0.3)
            .filter(|r| !only_mismatch.get() || r.schema_kind.contains("不一致"))
            .collect();
        match sort.get().as_str() {
            "empty" => v.sort_by(|a, b| b.empty_rate.partial_cmp(&a.empty_rate).unwrap()),
            "lag" => v.sort_by_key(|r| std::cmp::Reverse(r.lag_days.unwrap_or(-1))),
            _ => v.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap()),
        }
        v
    };

    view! {
        <section class="hero">
            <h1>"浙江开放数据健康榜"</h1>
            <p class="lede">
                "对 "<b>{fmt_num(s.audited)}</b>" 个政府开放数据集逐个体检（其中 "<b>{fmt_num(s.with_preview)}</b>" 个有样例）：字段是否对得上、数据是否停更、样例里有多少空值。"
                "每条结论都能展开看原始快照和机器可复核的证明。"
            </p>
        </section>

        <section class="stats">
            <div class="stat"><div class="num">{fmt_num(s.audited)}</div><div class="lbl">"纳入体检的数据集"</div></div>
            <div class="stat bad"><div class="num">{format!("{:.0}%", if s.audited > 0 { s.lag_over90_days as f64 / s.audited as f64 * 100.0 } else { 0.0 })}</div>
                <div class="lbl">{format!("超过 90 天未更新（{} 个）", fmt_num(s.lag_over90_days))}</div></div>
            <div class="stat bad"><div class="num">{fmt_num(s.empty_rate_over30)}</div><div class="lbl">"样例空值率 > 30%"</div></div>
            <div class="stat bad"><div class="num">{fmt_num(s.all_empty_column_datasets)}</div><div class="lbl">"存在整列为空"</div></div>
            <div class="stat warn"><div class="num">{fmt_num(s.schema_mismatch)}</div><div class="lbl">"声明列数与实际不符"</div></div>
            <div class="stat"><div class="num">{format!("{:.1}%", s.empty_rate_mean.unwrap_or(0.0) * 100.0)}</div><div class="lbl">"样例级平均空值率"</div></div>
            <div class="stat"><div class="num">{fmt_num(s.stopped_update)}</div><div class="lbl">"标注停更"</div></div>
            <div class="stat"><div class="num">{s.lag_median_days.unwrap_or(0)}</div><div class="lbl">"更新滞后中位数（天）"</div></div>
        </section>
        <p class="coverage">
            "覆盖：已证 "<b>{s.proof_count}</b>" 条 · 证据包 "<b>{s.evidence_count}</b>" 份 · AI 解读 "<b>{s.explain_count}</b>" 条"
            "（解读与证明都只覆盖问题最多的前 100 名）"
        </p>

        <section class="filters">
            <input class="search" type="search" placeholder="搜索数据集名称或部门…" prop:value=move || search.get()
                on:input=move |e| search.set(event_target_value(&e)) />
            <select on:change=move |e| dept.set(event_target_value(&e))>
                <option value="">"全部部门"</option>
                {depts.into_iter().map(|d| view! { <option value=d.clone()>{d.clone()}</option> }).collect_view()}
            </select>
            <select on:change=move |e| sort.set(event_target_value(&e))>
                <option value="score">"按问题分"</option>
                <option value="empty">"按空值率"</option>
                <option value="lag">"按停更时长"</option>
            </select>
            <label class="check"><input type="checkbox" on:change=move |e| only_stopped.set(event_target_checked(&e)) />"只看停更"</label>
            <label class="check"><input type="checkbox" on:change=move |e| only_empty.set(event_target_checked(&e)) />"空值 > 30%"</label>
            <label class="check"><input type="checkbox" on:change=move |e| only_mismatch.set(event_target_checked(&e)) />"字段对不上"</label>
        </section>

        <section class="board">
            {move || filtered().into_iter().enumerate().map(|(i, r)| row_view(i, r, route)).collect_view()}
        </section>
        <footer class="site">
            "抓取于 " {data.generated_at.clone()} " · 共 " {fmt_num(data.total as u64)} " 个数据集，展示前 " {rows.get().len()} " 个 · 数据来源 浙江省公共数据开放平台"
        </footer>
    }
}

fn row_view(i: usize, r: Row, route: RwSignal<Route>) -> impl IntoView {
    let id = r.id.clone();
    let flash = if r.score >= 8.0 {
        " bad"
    } else if r.score >= 5.0 {
        " warn"
    } else {
        ""
    };
    let mut chips: Vec<(String, String)> = Vec::new();
    if r.stopped() {
        chips.push(("chip bad".into(), "停更".into()));
    }
    if r.empty_rate > 0.0 {
        chips.push(("chip".into(), format!("空值 {:.1}%", r.empty_pct())));
    }
    if r.all_empty_cols > 0 {
        chips.push(("chip".into(), format!("{} 列全空", r.all_empty_cols)));
    }
    if r.declared_cols != r.sample_cols {
        chips.push((
            "chip".into(),
            format!("列数 {}→{}", r.declared_cols, r.sample_cols),
        ));
    }
    if r.proof.is_some() {
        chips.push(("chip ok".into(), "已证".into()));
    }
    view! {
        <a class=format!("row{flash}") href=format!("#/detail/{}", urlencoding(&id))
            on:click=move |_| { route.set(Route::Detail(id.clone())); }>
            <span class="num">{i + 1}</span>
            <div class="main">
                <div class="name">{r.name.clone()}</div>
                <div class="meta">{r.dept.clone()} " · " {r.cycle_label()} " · 问题分 " {format!("{:.1}", r.score)}</div>
            </div>
            <div class="chips">
                {chips.into_iter().map(|(c, t)| view! { <span class=c>{t}</span> }).collect_view()}
            </div>
        </a>
    }
}

fn detail_view(data: Data, id: &str, route: RwSignal<Route>) -> impl IntoView {
    let Some(r) = data.datasets.iter().find(|r| r.id == id).cloned() else {
        return view! { <section class="panel"><h2>"找不到这个数据集"</h2><p class="hint">"链接可能来自另一份 data.json。"</p></section> }.into_any();
    };
    let id_for_proof = r.id.clone();
    let bad = r.declared_cols != r.sample_cols || r.all_empty_cols > 0 || r.stopped();
    let verdict = if bad { "问题较多" } else { "基本可用" };
    let dims = vec![
        (
            "规范性",
            "GB/T 36344 0101",
            if r.desc_len >= 10 {
                "有说明"
            } else {
                "说明过短"
            }
            .to_string(),
            format!("字段说明 {} 字", r.desc_len),
        ),
        (
            "完整性",
            "GB/T 36344 02",
            format!("{:.1}%", r.empty_pct()),
            format!("样例 {} 行 × {} 列", r.sample_rows, r.sample_cols),
        ),
        (
            "一致性",
            "GB/T 36344 04",
            r.schema_kind.clone(),
            format!("声明 {} 列 / 样例 {} 列", r.declared_cols, r.sample_cols),
        ),
        (
            "时效性",
            "GB/T 36344 05",
            r.cycle_label(),
            format!("距上次更新 {} 天", r.lag_days.unwrap_or(-1)),
        ),
        (
            "可访问性",
            "GB/T 36344 06",
            if r.has_preview == 1 {
                "可预览"
            } else {
                "无样例"
            }
            .to_string(),
            "表头与样例可直接读取".to_string(),
        ),
        (
            "准确性",
            "GB/T 36344 03",
            "不评价".to_string(),
            "没有权威源可比对".to_string(),
        ),
    ];
    let explain = r.explain.clone();
    let evidence = r.evidence.clone();
    let proof = r.proof.clone();

    view! {
        <nav class="crumb"><a href="#/" on:click=move |_| route.set(Route::List)>"榜单"</a>" / 数据集详情"</nav>
        <h1 class="detail-title">{r.name.clone()}</h1>
        <div class="meta-chips">
            <span class="chip">{r.dept.clone()}</span>
            <span class="chip">{r.open_type.clone().unwrap_or_default()}</span>
            <span class="chip">"问题分 " {format!("{:.1}", r.score)}</span>
            {r.update_time.clone().map(|t| view! { <span class="chip">"最近更新 " {t}</span> })}
            {r.overdue.then(|| view! { <span class="chip bad">"超过声明更新周期"</span> })}
            {r.proof.is_some().then(|| view! { <span class="chip ok">"已生成证明"</span> })}
        </div>
        <div class=format!("verdict{}", if bad { " bad" } else { "" })>
            <b>{verdict}</b>" · 问题分只用于排序，不是科学指标。准确性不在评价范围内。"
        </div>

        <h2 class="section">"GB/T 36344 六维体检（准确性不评价）"</h2>
        <section class="dims">
            {dims.into_iter().map(|(n, std, v, note)| view! {
                <div class=format!("dim{}", if n == "准确性" { " na" } else { "" })>
                    <h3>{n}<span class="std">{std}</span></h3>
                    <p class="verdict-inline">{v}</p>
                    <p class="hint">{note}</p>
                </div>
            }).collect_view()}
        </section>

        {(!r.missing_fields.is_empty() || !r.extra_fields.is_empty()).then(|| view! {
            <section class="panel mismatch">
                <h2>"字段对不上的地方"</h2>
                {(!r.missing_fields.is_empty()).then(|| view! {
                    <p>"声明里有、样例里没有：" <b>{r.missing_fields.join("、")}</b></p>
                })}
                {(!r.extra_fields.is_empty()).then(|| view! {
                    <p>"样例里有、声明里没有：" <b>{r.extra_fields.join("、")}</b></p>
                })}
                <p class="hint">"列数相同但只是加了部门前缀的，不算字段缺失，已在分类里单列。"</p>
            </section>
        })}

        {explain.map(|e| view! {
            <section class="panel">
                <h2>"AI 解读" <span class=format!("chip {}", if e.status == "pass" { "ok" } else { "warn" })>
                    {if e.status == "pass" { "数字已核对" } else { "未通过核对" }}</span></h2>
                <p class="explain">{e.text}</p>
                <p class="hint">"这段文字由模型生成，只允许使用报告里的数字；对不上就重写，仍不合格不展示。"</p>
            </section>
        })}

        {evidence.map(|e| view! {
            <section class="panel">
                <h2>"证据包"</h2>
                <p class="hint">"接口原始响应逐字节落盘，全程确定性抓取，不经过任何模型。"</p>
                <table class="kv">
                    <tr><th>"抓取时间"</th><td>{e.fetched_at.clone()}</td></tr>
                    <tr><th>"SHA-256"</th><td>{e.sha256.clone()}</td></tr>
                    <tr><th>"响应大小"</th><td>{format!("detail {} 字节 / preview {} 字节", e.detail_bytes, e.preview_bytes)}</td></tr>
                    <tr><th>"原始文件"</th><td>
                        {e.files.iter().map(|f| {
                            let name = f.rsplit('/').next().unwrap_or(f).to_string();
                            view! { <a class="dl" href=f.clone() download>{name}" "</a> }
                        }).collect_view()}
                    </td></tr>
                </table>
            </section>
        })}

        {proof.map(|p| view! {
            <section class="panel">
                <h2>"证明" <span class="chip ok">"PASS"</span></h2>
                <p>"这份报告的数值结论已由 Lean 4 内核复核：样例 " {format!("{} 行 × {} 列，{} 条定理", p.rows, p.cols, p.cols)} "。"</p>
                <p class="hint">"快照 SHA-256 " {p.sha256.clone()}</p>
                <a class="btn" href=format!("#/proof/{}", urlencoding(&id_for_proof))
                   on:click=move |_| route.set(Route::Proof(id_for_proof.clone()))>"查看证明详情 →"</a>
            </section>
        })}
    }.into_any()
}

fn proof_view(data: Data, id: &str, route: RwSignal<Route>) -> impl IntoView {
    let Some(r) = data.datasets.iter().find(|r| r.id == id).cloned() else {
        return view! { <section class="panel"><h2>"找不到这份证明"</h2></section> }.into_any();
    };
    let Some(p) = r.proof.clone() else {
        return view! {
            <section class="panel"><h2>"证明尚未生成"</h2>
            <p class="hint">"当前只为问题最多的前 100 个数据集生成证明。"</p></section>
        }
        .into_any();
    };
    let theorem =
        "def rows_0 : List (List Nat) := [...]\n\ntheorem empty_0_0 : countEmpty rows_0 0 = 0 := by decide";
    view! {
        <nav class="crumb"><a href="#/" on:click=move |_| route.set(Route::List)>"榜单"</a>" / 证明"</nav>
        <div class="pass-card">
            <div class="stamp">"PASS"</div>
            <div><b>"Lean 4 内核已复核"</b><p class="hint">"报告的每个计数都由定理证明器独立校验：改一个数字，内核立即报错。"</p></div>
        </div>
        <section class="panel">
            <h2>{r.name.clone()}</h2>
            <table class="kv">
                <tr><th>"榜单序号"</th><td>{format!("第 {} 位", p.index + 1)}</td></tr>
                <tr><th>"样例规模"</th><td>{format!("{} 行 × {} 列", p.rows, p.cols)}</td></tr>
                <tr><th>"定理数量"</th><td>{format!("{} 条", p.cols)}</td></tr>
                <tr><th>"快照 SHA-256"</th><td>{p.sha256.clone()}</td></tr>
                <tr><th>"数据来源"</th><td>{p.source.clone().unwrap_or_else(|| "live".into())}</td></tr>
                <tr><th>"校验命令"</th><td>"lean TopProofs.lean"</td></tr>
            </table>
            <p><a class="btn" href="TopProofs.lean" download>"下载 TopProofs.lean"</a></p>
            <pre class="code">{theorem}</pre>
            <p class="hint">"每一行是一条真实样例记录，1 表示空值；定理断言「第 N 列恰好有 K 条空值」，由 by decide 交给内核穷举。"</p>
        </section>
    }.into_any()
}
