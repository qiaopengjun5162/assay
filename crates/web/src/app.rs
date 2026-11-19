//! 单页应用：榜单 / 详情 / 证明三个视图。
//!
//! 路由走 hash（`#/detail/<id>`），刷新可直达；不引第三方路由库，
//! 因为只有三个视图、两条跳转。

use crate::model::{Data, Row};
use crate::verify::verify_command;
use leptos::prelude::*;
use sha2::Digest;

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
                <div class="brand"><span class="mark">"数验"</span>" Assay"</div>
                <div class="top-meta">"每条结论都能自己验"</div>
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

    // 首屏「最离谱的一条」：声明了列、但实际给的列数远多于声明。
    // 声明 0 列的（字段清单根本没填）是另一种毛病，这里排掉，免得被说成挑刺。
    let exhibit = data
        .datasets
        .iter()
        .filter(|r| r.declared_cols > 0 && r.sample_cols > r.declared_cols)
        .max_by_key(|r| r.sample_cols - r.declared_cols)
        .cloned();
    let exhibit_view = exhibit.map(|e| {
        let id = e.id.clone();
        view! {
            <section class="exhibit">
                <div class="exhibit-head">
                    <span class="tag">"最离谱的一条"</span>
                    <span class="exhibit-dept">{e.dept.clone()}</span>
                </div>
                <h3>{e.name.clone()}</h3>
                <div class="vs">
                    <div class="vs-col">
                        <div class="vs-h">"页面上写着"</div>
                        <div class="vs-num">{e.declared_cols}" 个字段"</div>
                    </div>
                    <div class="vs-arrow">"→"</div>
                    <div class="vs-col bad">
                        <div class="vs-h">"实际给到"</div>
                        <div class="vs-num">{e.sample_cols}" 列"</div>
                        {(e.all_empty_cols > 0).then(|| view! {
                            <div class="vs-note">"其中 "{e.all_empty_cols}" 列整列是空的"</div>
                        })}
                    </div>
                </div>
                <a class="exhibit-link" href=format!("#/detail/{}", urlencoding(&id))
                   on:click=move |_| { route.set(Route::Detail(id.clone())); }>
                    "看这条的原始证据和机器证明 →"
                </a>
            </section>
        }
    });

    // 部门维度：按停更数量排，取前 12
    let mut dept_top: Vec<(String, crate::model::DeptRow)> = s
        .by_dept
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    dept_top.sort_by(|a, b| b.1.stopped.cmp(&a.1.stopped).then(b.1.n.cmp(&a.1.n)));
    dept_top.truncate(12);

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
            <p class="kicker">
                "浙江 · "{fmt_num(s.audited)}" 个数据集全查（"{fmt_num(s.with_preview)}" 个拿到样例）· 体检于 "{data.generated_at.clone()}
            </p>
            <h1>"政府开放数据「货不对板」榜"</h1>
            <p class="lede">
                "页面写着 7 个字段，实际给你 92 列；标着「每年更新」，两年没动。"
                "有问题的排成榜，每条结论你都能自己验——不用信我。"
            </p>
        </section>
        {exhibit_view}

        <section class="stats">
            <div class="stat"><div class="num">{fmt_num(s.audited)}</div><div class="lbl">"纳入体检的数据集"</div></div>
            <div class="stat bad"><div class="num">{format!("{:.1}%", if s.audited > 0 { s.lag_over90_days as f64 / s.audited as f64 * 100.0 } else { 0.0 })}</div>
                <div class="lbl">{format!("超过 90 天未更新（{} 个）", fmt_num(s.lag_over90_days))}</div></div>
            <div class="stat bad"><div class="num">{fmt_num(s.empty_rate_over30)}</div><div class="lbl">"样例空值率 > 30%"</div></div>
            <div class="stat bad"><div class="num">{fmt_num(s.all_empty_column_datasets)}</div><div class="lbl">"存在整列为空"</div></div>
            <div class="stat warn"><div class="num">{fmt_num(s.schema_mismatch)}</div><div class="lbl">"声明列数与实际不符"</div></div>
            <div class="stat"><div class="num">{format!("{:.1}%", s.empty_rate_mean.unwrap_or(0.0) * 100.0)}</div><div class="lbl">"样例级平均空值率"</div></div>
            <div class="stat"><div class="num">{fmt_num(s.stopped_update)}</div><div class="lbl">"标注停更"</div></div>
            <div class="stat"><div class="num">{s.lag_median_days.unwrap_or(0)}</div><div class="lbl">"更新滞后中位数（天）"</div></div>
        </section>
        <p class="coverage">
            "榜上 "<b>{s.evidence_count}</b>" 条全部有证据包（点开就能自己重算哈希）· 其中 "
            <b>{s.proof_count}</b>" 条有机器证明 · AI 解读 "<b>{s.explain_count}</b>" 条"
            <span class="hint">"（剩下那些是平台只给了表头、没有数据行，没有可证的命题）"</span>
        </p>

        <section class="deptboard">
            <h2>"问题最多的部门"</h2>
            <p class="hint">"按「停更数据集数量」排序，取前 12 个部门。平均空值率来自样例（每份 5 行左右），只作参考。"</p>            <div class="deptgrid">
                {dept_top.into_iter().map(|(name, d)| view! {
                    <div class="deptcard">
                        <div class="dname">{name}</div>
                        <div class="dmeta">{format!(
                            "{} 个数据集 · 停更 {} · 字段对不上 {} · 平均空值率 {}",
                            d.n,
                            d.stopped,
                            d.mismatch,
                            d.empty_avg
                                .map(|v| format!("{:.1}%", v * 100.0))
                                .unwrap_or_else(|| "无样例".to_string())
                        )}</div>
                    </div>
                }).collect_view()}
            </div>
        </section>

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
            "体检于 " {data.generated_at.clone()} " · 共 " {fmt_num(data.total as u64)} " 个数据集，展示前 " {rows.get().len()} " 个 · 数据来源 浙江省公共数据开放平台"
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
    // 判定「问题较多」的依据要写在脸上：只给结论不给理由，读者只能猜。
    let mut reasons: Vec<String> = Vec::new();
    if r.declared_cols != r.sample_cols {
        reasons.push(format!(
            "声明 {} 列、实际 {} 列",
            r.declared_cols, r.sample_cols
        ));
    }
    if r.all_empty_cols > 0 {
        reasons.push(format!("{} 列整列全空", r.all_empty_cols));
    }
    if r.stopped() {
        reasons.push(format!(
            "平台标了停更，距上次更新 {} 天",
            r.lag_days.unwrap_or(0)
        ));
    }
    let why = reasons.join("；");
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
            format!("{:.1}%", r.fill_pct()),
            format!(
                "非空比例 · 样例 {} 行 × {} 列",
                r.sample_rows, r.sample_cols
            ),
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
    let report = r.report;

    let report_href = r.report_path();

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
            {report.then(|| view! { <span class="chip ok">"有书面报告"</span> })}
        </div>
        <div class=format!("verdict{}", if bad { " bad" } else { "" })>
            <b>{verdict}</b>
            {(!why.is_empty()).then(|| view! { <span>"：" {why.clone()}</span> })}
            " · 问题分只用于排序，不是科学指标。准确性不在评价范围内。"
            {report.then(|| view! {
                <a class="btn" href=report_href.clone() download>"下载体检报告（Markdown）"</a>
            })}
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
                <h3>"自己验一遍"</h3>
                <p class="hint">
                    "不用装我们的东西。先在本页点按钮，浏览器会把两个原始文件取回来，在本地重算哈希并跟上表的 SHA-256 逐字符比对；也可以把原始文件下到同一个目录，跑下面这条命令，输出必须与上表相同。对不上，就是这份证据被动过。"
                </p>
                <VerifyButton sha256=e.sha256.clone() files=e.files.clone() />
                {{
                    // 页头地址 = origin + pathname。命令要复制到评委自己的电脑上跑，
                    // 相对路径 `evidence/...` 会 curl 报 Could not resolve host，
                    // 开着代理时更糟——会拿到一张错误页假装成功。补成绝对 URL。
                    let loc = web_sys::window().map(|w| w.location());
                    let base = loc
                        .as_ref()
                        .and_then(|l| l.origin().ok().zip(l.pathname().ok()))
                        .map(|(o, p)| format!("{o}{p}"))
                        .unwrap_or_default();
                    let cmd = verify_command(&base, &e.files);
                    (!cmd.is_empty()).then(|| view! {
                        <pre class="mono-block">{cmd.clone()}</pre>
                        <p class="verify"><CopyButton text=cmd /></p>
                        <p class="hint">"Linux 把 shasum -a 256 换成 sha256sum 即可，输出格式一致。命令里已是完整网址，复制到任意电脑的终端都能跑。"</p>
                    })
                }}
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

/// 浏览器内复核：把两个原始文件取回来，在本地算 SHA-256，和索引里的值逐字符对照。
///
/// 等于把 `assay verify-evidence` 跑在页面里——评委不用装 Rust、不用开终端，
/// 只用看页面自己的结果。取值必须走原始字节（`binary()`），转成文本再算会漂。
#[component]
fn VerifyButton(sha256: String, files: Vec<String>) -> impl IntoView {
    let state = RwSignal::new(State::Idle);
    let expected = sha256.clone();

    let run = move |_| {
        let files = files.clone();
        let expected = expected.clone();
        state.set(State::Running);
        leptos::task::spawn_local(async move {
            state.set(match recompute(&files).await {
                Ok(d) if d == expected => State::Ok(d),
                Ok(d) => State::Mismatch(d),
                Err(e) => State::Failed(e),
            });
        });
    };

    view! {
        <p class="verify">
            <button class="btn" on:click=run disabled=move || state.get() == State::Running>
                {move || if state.get() == State::Running { "正在重算…" } else { "在浏览器里重算 SHA-256" }}
            </button>
            {move || match state.get() {
                State::Idle => view! { <span class="hint">"只读本页静态文件，不发任何写请求。"</span> }.into_any(),
                State::Running => view! { <span class="hint">"取原始字节中…"</span> }.into_any(),
                State::Ok(d) => view! {
                    <span class="chip ok">"一致"</span>
                    <span class="mono-sm">{d}</span>
                }.into_any(),
                State::Mismatch(d) => view! {
                    <span class="chip bad">"不一致，这份证据被动过"</span>
                    <span class="mono-sm">{d}</span>
                }.into_any(),
                State::Failed(e) => view! { <span class="chip warn">{e}</span> }.into_any(),
            }}
        </p>
    }
}

/// 一键把复核命令复制到剪贴板。clipboard API 只在安全上下文可用
/// （HTTPS 或 localhost）；不可用时如实提示手动选中，不假装成功。
#[component]
fn CopyButton(text: String) -> impl IntoView {
    let copied = RwSignal::new(false);
    let failed = RwSignal::new(String::new());

    let copy = move |_| {
        let text = text.clone();
        leptos::task::spawn_local(async move {
            match copy_to_clipboard(&text).await {
                Ok(()) => {
                    copied.set(true);
                    failed.set(String::new());
                }
                Err(e) => failed.set(e),
            }
        });
    };

    view! {
        <button class="btn" on:click=copy>"复制命令"</button>
        {move || if copied.get() {
            view! { <span class="chip ok">"已复制，粘到终端回车即可"</span> }.into_any()
        } else if !failed.get().is_empty() {
            let msg = failed.get();
            view! { <span class="chip warn">{format!("复制失败：{msg}；请手动选中上面的命令")}</span> }.into_any()
        } else {
            view! { <span class="hint">"复制到任意电脑的终端即可运行。"</span> }.into_any()
        }}
    }
}

/// 写剪贴板。web-sys 的 `write_text` 返回 Promise，用已有的 futures crate 等它。
async fn copy_to_clipboard(text: &str) -> Result<(), String> {
    let clipboard = web_sys::window()
        .map(|w| w.navigator().clipboard())
        .ok_or_else(|| "浏览器未开放剪贴板（需要 HTTPS 或 localhost）".to_string())?;
    let promise = clipboard.write_text(text);
    wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|_| "剪贴板写入未完成".to_string())?;
    Ok(())
}

#[derive(Clone, Debug, PartialEq)]
enum State {
    Idle,
    Running,
    Ok(String),
    Mismatch(String),
    Failed(String),
}

/// 与 `evidence::capture_one` 同一字节顺序：detail ‖ "\n--split--\n" ‖ preview。
///
/// 只依赖 `sha2`，不引 `assay-core`——core 拖着 ureq/rustls，进 wasm 不值当。
async fn recompute(files: &[String]) -> Result<String, String> {
    let [detail_url, preview_url] = files else {
        return Err(format!("证据条目只给了 {} 个文件", files.len()));
    };
    let mut buf: Vec<u8> = Vec::new();
    for (i, url) in [detail_url, preview_url].iter().enumerate() {
        let resp = gloo_net::http::Request::get(url)
            .send()
            .await
            .map_err(|e| format!("取不到 {}：{e}", url.rsplit('/').next().unwrap_or(url)))?;
        if !resp.ok() {
            return Err(format!("{} 返回 HTTP {}", url, resp.status()));
        }
        if i == 1 {
            buf.extend_from_slice(b"\n--split--\n");
        }
        buf.extend_from_slice(
            &resp
                .binary()
                .await
                .map_err(|e| format!("读不出 {url}：{e}"))?,
        );
    }
    Ok(hex(&sha2::Sha256::digest(&buf)))
}

/// 与 `evidence::hex` 相同的十六进制小写输出。
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
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
    // 这里只放格式示意，不放可执行代码：页面拿不到真实定理片段，
    // 编一条 `= 0` 出来反而会让懂行的人怀疑整份证明是假的。
    // 真实定理在 TopProofs.lean 里，下载入口就在下面。
    let theorem = format!(
        "-- 每条定理长这样（本数据集共 {} 条，真实内容见下方 TopProofs.lean）：\n\
         --\n\
         --   theorem empty_col_37 : countEmpty rows 37 = 2 := by decide\n\
         --\n\
         -- rows 是这份数据的样例矩阵，1 表示空值；\n\
         -- 等号右边是内核要验的结论，改一位数字就报错。",
        p.cols
    );
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
