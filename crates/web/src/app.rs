//! 单页应用：榜单 / 详情 / 证明三个视图。
//!
//! 路由走 hash（`#/detail/<id>`），刷新可直达；不引第三方路由库，
//! 因为只有三个视图、两条跳转。

use crate::charts::{self, Bar};
use crate::model::{CheckRow, Data, Row};
use crate::spark;
use crate::verify::verify_command;
use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use sha2::Digest;

#[derive(Clone, Debug, PartialEq)]
enum Route {
    List,
    Detail(String),
    Proof(String),
    Dept(String),
    Depts,
    Proofs,
    Checks,
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
    if let Some(rest) = h.strip_prefix("dept/") {
        // 部门名里有中文和空格，链接里是 encodeURIComponent 过的
        return Route::Dept(decode(rest.split('?').next().unwrap_or(rest)));
    }
    if h == "checks" {
        return Route::Checks;
    }
    // 列表级视图：部门榜 / 证明索引。放在单条路由之后，避免吃掉 detail/dept 前缀。
    if h == "depts" {
        return Route::Depts;
    }
    if h == "proofs" {
        return Route::Proofs;
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

/// 从 hash 里取查询参数：`#/?dept=xxx` 这种。
///
/// 详情页的部门标签用它跳回榜单并预置筛选——读者看完一条自然会问
/// 「这个部门还有哪些」，不该让他自己再去下拉里找一遍。
fn hash_query(key: &str) -> String {
    web_sys::window()
        .and_then(|w| w.location().hash().ok())
        .and_then(|h| h.split_once('?').map(|(_, q)| q.to_string()))
        .and_then(|q| {
            q.split('&').find_map(|kv| {
                let (k, v) = kv.split_once('=')?;
                (k == key).then(|| decode(v))
            })
        })
        .unwrap_or_default()
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

/// 从首屏 CTA 跳到详情页的证据锚点。路由是 hash，浏览器不会等待新视图渲染，
/// 所以延迟一个事件循环再 scrollIntoView，否则元素还不存在、滚了个空。
fn scroll_to_evidence() {
    use wasm_bindgen::prelude::*;
    let cb = Closure::<dyn FnMut()>::new(move || {
        let el = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("evidence"));
        if let Some(el) = el {
            let _ = el.dyn_ref::<web_sys::Element>().map(|e| {
                e.scroll_into_view_with_bool(true);
            });
        }
    });
    if let Some(w) = web_sys::window() {
        let _ =
            w.set_timeout_with_callback_and_timeout_and_arguments_0(cb.as_ref().unchecked_ref(), 0);
    }
    cb.forget();
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
            {/* 键盘用户按 Tab 第一下就能跳过页头；榜单 500 行，一行行 Tab 过去没人受得了。
                不能让它真的改 hash——那是路由，`#main` 会被解析成「回榜单」。 */}
            <a class="skip" href="#main" on:click=move |e| {
                e.prevent_default();
                if let Some(el) = web_sys::window()
                    .and_then(|w| w.document())
                    .and_then(|d| d.get_element_by_id("main"))
                {
                    let _ = el.dyn_ref::<web_sys::HtmlElement>().map(|h| h.focus());
                }
            }>"跳到主要内容"</a>
            <header class="topbar">
                <div class="brand"><span class="mark">"数验"</span>" Assay"</div>
                <div class="top-meta">"每条结论都能自己验"</div>
                <nav class="top-nav" aria-label="主导航">
                    <a href="#/" on:click=move |_| { route.set(Route::List); }>"榜单"</a>
                    <a href="#/depts" on:click=move |_| { route.set(Route::Depts); }>"部门"</a>
                    <a href="#/proofs" on:click=move |_| { route.set(Route::Proofs); }>"证明"</a>
                    <a href="#/checks" on:click=move |_| { route.set(Route::Checks); }>"判定规则"</a>
                </nav>
            </header>
            <main id="main" tabindex="-1">
            {move || match (data.get(), route.get()) {
                (None, _) => view! {
                    <div class="skeleton" role="status" aria-label="正在加载榜单">
                        <div class="sk-line sk-title"></div>
                        <div class="sk-line sk-lede"></div>
                        <div class="sk-card"></div>
                        <div class="sk-grid">
                            <div class="sk-stat"></div><div class="sk-stat"></div>
                            <div class="sk-stat"></div><div class="sk-stat"></div>
                        </div>
                        <p class="hint">"正在加载榜单…"</p>
                    </div>
                }.into_any(),
                (Some(d), Route::List) => list_view(d, route).into_any(),
                (Some(d), Route::Detail(id)) => detail_view(d, &id, route).into_any(),
                (Some(d), Route::Proof(id)) => proof_view(d, &id, route).into_any(),
                (Some(d), Route::Dept(n)) => dept_view(d, &n, route).into_any(),
                (Some(d), Route::Depts) => depts_index(d, route).into_any(),
                (Some(d), Route::Proofs) => proofs_index(d, route).into_any(),
                (Some(d), Route::Checks) => checks_page(d).into_any(),
            }}
            </main>
            {move || data.get().map(|d| view! {
                <SiteFooter
                    generated_at=d.generated_at.clone()
                    total=d.total as u64
                    shown=d.datasets.len()
                />
            })}
        </div>
    }
}

fn list_view(data: Data, route: RwSignal<Route>) -> impl IntoView {
    let search = RwSignal::new(String::new());
    // 详情页点部门标签会带 `#/?dept=…` 过来，初次进入就按它筛好
    let dept = RwSignal::new(hash_query("dept"));
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
    // 部门全量条数：筛完只显示榜上那几条，得能说清「其余不在前 500 名里」
    let by_dept = data.summary.by_dept.clone();
    let rows = RwSignal::new(data.datasets.clone());

    // 首屏「最离谱的一条」：声明了列、但实际给的列数远多于声明。
    // 声明 0 列的（字段清单根本没填）是另一种毛病，这里排掉，免得被说成挑刺。
    let exhibit = data
        .datasets
        .iter()
        .filter(|r| r.declared_cols > 0 && r.sample_cols > r.declared_cols)
        .max_by_key(|r| r.sample_cols - r.declared_cols)
        .cloned();

    // 首屏那句话必须和下面的卡片同源；纯函数实现见 hero_claim_text，可被单测锁住。
    let hero_claim: String = hero_claim_text(exhibit.as_ref());

    let exhibit_view = exhibit.map(|e| {
        let id = e.id.clone();
        let cta_id = e.id.clone();
        view! {
            <section class="exhibit">
                <div class="exhibit-head">
                    <span class="tag">"最离谱的一条"</span>
                    <span class="exhibit-dept">{e.dept.clone()}</span>
                </div>
                <h2>{e.name.clone()}</h2>
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
                <div class="exhibit-actions">
                    <a class="btn btn-primary" href=format!("#/detail/{}", urlencoding(&id))
                       on:click=move |_| {
                           route.set(Route::Detail(id.clone()));
                           scroll_to_evidence();
                       }>"点这里，现场验一条 →"</a>
                    <a class="exhibit-link" href=format!("#/detail/{}", urlencoding(&cta_id))
                       on:click=move |_| { route.set(Route::Detail(cta_id.clone())); }>
                        "或先看完整分析"
                    </a>
                </div>
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

    // 检查项清单：把「尺子」印在榜下面。
    //
    // 文字与命中数全部来自 data.json 的 `checks`（由 core 的 `checks.rs` 发布），
    // 前端不自己写一份文案——写一份就是下一条要分叉的口径。
    // 分母是 `c.of`（全量 24,322），不是榜单展示的行数，两个数必须各自说清。
    let total_checked = data.checks.first().map(|c| c.of).unwrap_or(0);
    let checks_view = (!data.checks.is_empty())
        .then(|| checks_section(&data.checks, total_checked, rows.get().len()));

    // 跑批历史：只画原始计数。
    //
    // 一个点不画线——单点连不成趋势，画成一小段横线会被读成「稳定」。
    // 零个点整块不渲染：空白图例比没有图更糟。
    let history_view = {
        let pts = data.history.clone();
        (!pts.is_empty()).then(|| {
            if pts.len() < 2 {
                let p = &pts[0];
                return view! {
                    <section class="history">
                        <h2>"指标在变好还是变坏"</h2>
                        <p class="hint">
                            {format!(
                                "本周（{}）已记下第一条：滞后 > 90 天 {} 个、字段对不上 {} 个、整列全空 {} 个。周度巡检每周自动追加一条，攒够两条这里就会长出趋势线——每一条都是真实跑批，不会补点凑数。",
                                p.date,
                                fmt_num(p.lag_over90),
                                fmt_num(p.mismatch),
                                fmt_num(p.all_empty),
                            )}
                        </p>
                    </section>
                }
                .into_any();
            }

            let first = pts.first().map(|p| p.date.clone()).unwrap_or_default();
            let last = pts.last().map(|p| p.date.clone()).unwrap_or_default();
            let lag: Vec<f64> = pts
                .iter()
                .map(|p| {
                    if p.audited > 0 {
                        p.lag_over90 as f64 / p.audited as f64 * 100.0
                    } else {
                        0.0
                    }
                })
                .collect();
            let mism: Vec<f64> = pts.iter().map(|p| p.mismatch as f64).collect();
            let empty: Vec<f64> = pts.iter().map(|p| p.all_empty as f64).collect();

            let cards: Vec<(String, String, String, String)> = [
                ("超过 90 天未更新", &lag, " 个百分点", true),
                ("声明列数与实际不符", &mism, " 个", false),
                ("存在整列全空", &empty, " 个", false),
            ]
            .into_iter()
            .map(|(label, values, unit, is_ratio)| {
                let cur = *values.last().unwrap_or(&0.0);
                let prev = *values.get(values.len().saturating_sub(2)).unwrap_or(&0.0);
                let shown = if is_ratio {
                    format!("{cur:.1}%")
                } else {
                    fmt_num(cur as u64)
                };
                (
                    label.to_string(),
                    shown,
                    spark::trend(cur, prev, unit),
                    spark::path(values, 240.0, 44.0, 4.0),
                )
            })
            .collect();

            view! {
                <section class="history">
                    <h2>"指标在变好还是变坏"</h2>
                    <p class="hint">
                        {format!(
                            "每次跑批记一条，只记原始计数——问题分随权重变，分数进不了这条线。{first} → {last}，共 {} 次跑批。",
                            pts.len()
                        )}
                    </p>
                    <div class="sparkgrid">
                        {cards
                            .into_iter()
                            .map(|(label, shown, trend, d)| {
                                view! {
                                    <div class="spark">
                                        <div class="spark-h">{label}</div>
                                        <div class="spark-num">{shown}</div>
                                        <svg viewBox="0 0 240 44" preserveAspectRatio="none">
                                            <path d=d fill="none" stroke="currentColor" stroke-width="1.6" />
                                        </svg>
                                        <div class="spark-trend">{trend}</div>
                                    </div>
                                }
                            })
                            .collect_view()}
                    </div>
                </section>
            }
            .into_any()
        })
    };

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
            .filter(|r| !only_empty.get() || r.empty_rate > crate::constants::EMPTY_RATE_HIGH)
            .filter(|r| !only_mismatch.get() || r.schema_kind.contains("不一致"))
            .collect();
        crate::sort::sort_rows(&mut v, sort.get().as_str());
        v
    };

    view! {
        <section class="hero">
            <p class="kicker">
                "浙江 · "{fmt_num(s.audited)}" 个数据集全查（"{fmt_num(s.with_preview)}" 个拿到样例）· 体检于 "
                <span class="nw">{data.generated_at.clone()}</span>
            </p>
            <h1>"政府开放数据「货不对板」榜"</h1>
            <p class="lede">
                {hero_claim}
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

        <Dashboard data=data.clone() route=route />

        <ol class="howto">
            <li><b>"① 看榜"</b>"按问题分排序，左边带红条的是停更或字段对不上的。"</li>
            <li><b>"② 点开一条"</b>"六维体检、具体哪几列对不上、AI 解读，都在同一页。"</li>
            <li><b>"③ 自己验"</b>"点按钮当场重算 SHA-256；也可以把命令复制到自己终端跑。"</li>
        </ol>

        {history_view}

        <section class="deptboard">
            <h2>"问题最多的部门"</h2>
            <p class="hint">"按「停更数据集数量」排序，取前 12 个部门。点进去看这个部门的体检结论。平均空值率来自样例（每份 5 行左右），只作参考。"</p>
            <div class="deptgrid">
                {dept_top.into_iter().map(|(name, d)| {
                    let href = format!("#/dept/{}", urlencoding(&name));
                    let pick = name.clone();
                    view! {
                    <a class="deptcard" href=href on:click=move |_| { route.set(Route::Dept(pick.clone())); }>
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
                    </a>
                }}).collect_view()}
            </div>
        </section>

        <section class="filters" aria-label="筛选与排序">
            <input class="search" type="search" placeholder="搜索数据集名称或部门…" aria-label="搜索数据集名称或部门"
                prop:value=move || search.get()
                on:input=move |e| search.set(event_target_value(&e)) />
            <select aria-label="按部门筛选" on:change=move |e| dept.set(event_target_value(&e))>
                <option value="" selected=move || dept.get().is_empty()>"全部部门"</option>
                {depts.into_iter().map(|d| {
                    let val = d.clone();
                    let label = d.clone();
                    // 用 option 的 selected 而不是 select 的 value：
                    // `#/?dept=…` 进来时 options 还没渲染，prop:value 会被吞掉，
                    // 结果列表筛好了、下拉却显示「全部部门」。
                    view! {
                        <option value=val.clone() selected=move || dept.get() == val>{label}</option>
                    }
                }).collect_view()}
            </select>
            <select aria-label="排序方式" on:change=move |e| sort.set(event_target_value(&e))>
                <option value="score" selected=move || sort.get() == "score">"按问题分"</option>
                <option value="empty" selected=move || sort.get() == "empty">"按空值率"</option>
                <option value="lag" selected=move || sort.get() == "lag">"按停更时长"</option>
            </select>
            <label class="check"><input type="checkbox" on:change=move |e| only_stopped.set(event_target_checked(&e)) />"只看停更"</label>
            <label class="check"><input type="checkbox" on:change=move |e| only_empty.set(event_target_checked(&e)) />"空值 > 30%"</label>
            <label class="check"><input type="checkbox" on:change=move |e| only_mismatch.set(event_target_checked(&e)) />"字段对不上"</label>
        </section>

        {/* 任何筛选生效都报命中数：搜一个词只剩 5 条时，读者需要知道「是筛出来的，不是只有 5 条」。
            部门卡说的「这个部门有多少」是全量，榜单只放前 500 名，两个数不写清楚会被读成藏数据。 */}
        {move || {
            let q = search.get();
            let dd = dept.get();
            let any = !q.is_empty()
                || !dd.is_empty()
                || only_stopped.get()
                || only_empty.get()
                || only_mismatch.get();
            any.then(|| {
                let n = filtered().len();
                let listed = rows.get().len();
                let kw = if q.is_empty() {
                    String::new()
                } else {
                    format!(" · 关键词「{q}」")
                };
                // 部门全量 vs 榜上条数：用「只按部门筛」的数算隐藏条数，不受搜索词干扰
                let dept_detail = if dd.is_empty() {
                    None
                } else {
                    let total = by_dept.get(&dd).map(|v| v.n).unwrap_or(0);
                    let in_board = rows.get().iter().filter(|r| r.dept == dd).count() as u64;
                    let hidden = total.saturating_sub(in_board);
                    Some(format!(
                        " · 「{dd}」全量 {total} 个，其余 {hidden} 个不在前 {listed} 名内"
                    ))
                };
                view! {
                    <p class="filter-note">
                        "筛出 " <b>{n.to_string()}</b> " 条（榜上共 "{listed.to_string()}" 条）"
                        {kw}
                        {dept_detail}
                        " "
                        <button class="linkbtn" on:click=move |_| {
                            search.set(String::new());
                            dept.set(String::new());
                            only_stopped.set(false);
                            only_empty.set(false);
                            only_mismatch.set(false);
                        }>"清除筛选"</button>
                    </p>
                }
            })
        }}

        <section class="board" id="board">
            {move || filtered().into_iter().enumerate().map(|(i, r)| row_view(i, r, route)).collect_view()}
        </section>
        {checks_view}
    }
}

/// 首屏导语：只讲这条 exhibit 真实支撑的字段，和下方卡片同源。
///
/// 早期这里写死「每年更新、两年没动」，但快照里 exhibit 这条的
/// `update_cycle` 是 `999`（平台语义=停更，被误译成"每年"）、`lag_days` 只有 35。
/// 现在停更走 `stopped()` 判定、滞后取真实 lag，缺 lag 就不提时效，绝不补假数
/// （不变量 9 + 不造假）。
fn hero_claim_text(exhibit: Option<&crate::model::Row>) -> String {
    match exhibit {
        Some(e) => {
            let mut parts = vec![format!(
                "页面上写着 {} 个字段，实际给你 {} 列",
                e.declared_cols, e.sample_cols
            )];
            if e.all_empty_cols > 0 {
                parts.push(format!("其中 {} 列整列是空的", e.all_empty_cols));
            }
            // '999'/'不更新' 这类码在平台语义里是停更，前端统一由 stopped() 判定；
            // 早期写死成「每年更新」是把 999 误译了，这里走同一处判定不重造口径。
            if e.stopped() {
                parts.push("平台标了停更".to_string());
            }
            if let Some(lag) = e.lag_days {
                if lag > 0 {
                    parts.push(format!("距上次更新 {lag} 天"));
                }
            }
            format!("{}。", parts.join("，"))
        }
        None => "有问题的开放数据集，我们排成了榜。".to_string(),
    }
}

/// 把更新周期的原始码值翻成人话。data.json 里 key 是 `-1` / `999` / `6` 这类码，
/// 直接印在图上没人看得懂；映射只在这里做一次，不回写数据。
///
/// 注意：数字码与文字 key 在平台数据里是**不同的来源**（如 `365` 7321 条、
/// `每年` 1280 条），不能翻译成同一个名字——那会在图例里出现两个「每年」，
/// 读者以为重复渲染了。所以数字码翻人话后保留码值（`每年（365）`），
/// 和纯文字 key（`每年`）分得开，也让人看得懂这个码是什么意思。
fn cycle_label(key: &str) -> String {
    let human = match key {
        "-1" => Some("不更新"),
        "999" => Some("长期停更"),
        "1" => Some("每日"),
        "6" => Some("每周"),
        "30" => Some("每月"),
        "365" => Some("每年"),
        _ => None,
    };
    match human {
        Some(h) => format!("{h}（{key}）"),
        None => key.to_string(),
    }
}

/// 环形图配色：语义色沿用 CSS 变量对应的 hex，图例与扇区一一对应。
/// 只用了 6 色，多于 6 个来源时后面的归到「其他」，避免屏幕上出现彩虹。
const SLICE_COLORS: [&str; 6] = [
    "#b3261e", "#8a5300", "#1f4ed8", "#0b6b3a", "#7a5ab8", "#5f6b7a",
];

/// 四张概览图：更新周期环形、错位类型条形、部门停更横向条（可点筛选）、滞后分布直方图。
/// 所有数据来自 `data.json`，前端不自己造数；图只是把已有聚合画出来。
#[component]
fn Dashboard(data: Data, route: RwSignal<Route>) -> impl IntoView {
    // 环形图：更新周期分布。按计数降序取前 6，其余合并成「其他」。
    let mut cycles: Vec<(String, u64)> = data
        .summary
        .update_cycle
        .iter()
        .map(|(k, v)| (cycle_label(k), *v))
        .collect();
    cycles.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    if cycles.len() > SLICE_COLORS.len() {
        let rest: u64 = cycles[SLICE_COLORS.len()..].iter().map(|(_, v)| *v).sum();
        cycles.truncate(SLICE_COLORS.len());
        cycles.push(("其他".to_string(), rest));
    }
    let cycle_slices = charts::slices(&cycles);
    let cycle_bounds = charts::arc_bounds(&cycle_slices);
    let cycle_arcs: Vec<String> = cycle_slices
        .iter()
        .zip(cycle_bounds.iter())
        .filter_map(|(_, (a, b))| {
            let d = charts::donut_arc(60.0, 60.0, 52.0, 32.0, *a, *b);
            (!d.is_empty()).then_some(d)
        })
        .collect();
    let cycle_legend: Vec<(String, String, String)> = cycle_slices
        .iter()
        .enumerate()
        .map(|(i, s)| {
            (
                SLICE_COLORS[i % SLICE_COLORS.len()].to_string(),
                s.label.clone(),
                format!("{:.1}%", s.pct),
            )
        })
        .collect();

    // 错位类型条形图：只展示有命中的种类，「一致」占绝大多数会压扁其它条，
    // 所以按值降序取前 5，并在文案里说明分母。
    let mismatch: Vec<(String, u64)> = data
        .summary
        .mismatch_kinds
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    let mism_bars = charts::top_bars(&mismatch, 5);
    let mism_max = mism_bars.first().map(|b| b.value).unwrap_or(0);

    // 部门横向条：按停更数排前 10，每条可点，直接把 route 切到部门页。
    let mut depts: Vec<(String, u64)> = data
        .summary
        .by_dept
        .iter()
        .map(|(k, v)| (k.clone(), v.stopped))
        .filter(|(_, n)| *n > 0)
        .collect();
    depts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    depts.truncate(10);
    let dept_bars = charts::top_bars(&depts, 10);
    let dept_max = dept_bars.first().map(|b| b.value).unwrap_or(0);

    // 滞后直方图：用榜单前 500 条明细（summary 里只有聚合数，画不了分布）。
    // 分母必须写清是「榜上 500 条」，不能让人误以为是全量 24,322。
    let lags: Vec<Option<i64>> = data.datasets.iter().map(|r| r.lag_days).collect();
    let lag_bars = charts::histogram(&lags, 30, 8);
    let lag_max = lag_bars.iter().map(|b| b.value).max().unwrap_or(0);

    view! {
        <section class="dash" aria-label="体检结果概览图">
            <div class="dash-head">
                <h2>"一眼看懂这批数据"</h2>
                <p class="hint">"下面四张图只读本页数据，不重新抓、不重新算；点部门条可直接筛到该部门。"</p>
            </div>
            <div class="dashgrid">
                <div class="dcard">
                    <h3>"更新周期分布"</h3>
                    <div class="donut-row">
                        <svg class="donut" viewBox="0 0 120 120" role="img" aria-label="更新周期分布环形图">
                            {cycle_arcs.into_iter().enumerate().map(|(i, d)| view! {
                                <path d=d fill=SLICE_COLORS[i % SLICE_COLORS.len()] />
                            }).collect_view()}
                            <text x="60" y="56" text-anchor="middle" class="donut-num">{fmt_num(data.summary.audited)}</text>
                            <text x="60" y="70" text-anchor="middle" class="donut-cap">"个数据集"</text>
                        </svg>
                        <ul class="legend">
                            {cycle_legend.into_iter().map(|(color, label, pct)| view! {
                                <li><span class="sw" style=format!("background:{color}")></span>{label}" "{pct}</li>
                            }).collect_view()}
                        </ul>
                    </div>
                </div>

                <div class="dcard">
                    <h3>"字段与声明对不对得上"</h3>
                    <p class="dsub">"分母是全部 "{fmt_num(data.summary.audited)}" 条，不是榜上前 "{data.datasets.len()}" 条"</p>
                    <div class="bars">
                        {mism_bars.into_iter().map(|b: Bar| {
                            let h = charts::bar_height(b.value, mism_max, 100.0);
                            view! {
                                <div class="bar-row">
                                    <span class="bar-label">{b.label}</span>
                                    <div class="bar-track"><div class="bar-fill" style=format!("height:{h:.1}%")></div></div>
                                    <span class="bar-num">{fmt_num(b.value)}</span>
                                </div>
                            }
                        }).collect_view()}
                    </div>
                </div>

                <div class="dcard">
                    <h3>"停更最多的部门"</h3>
                    <p class="dsub">"点部门名即可筛到该部门，看它的具体条目"</p>
                    <div class="bars">
                        {dept_bars.into_iter().map(|b: Bar| {
                            let h = charts::bar_height(b.value, dept_max, 100.0);
                            let picked = b.label.clone();
                            let title = b.label.clone();
                            let shown = b.label.clone();
                            view! {
                                <button class="bar-row bar-link" on:click=move |_| route.set(Route::Dept(picked.clone()))>
                                    <span class="bar-label" title=title>{shown}</span>
                                    <div class="bar-track"><div class="bar-fill bad" style=format!("height:{h:.1}%")></div></div>
                                    <span class="bar-num">{fmt_num(b.value)}</span>
                                </button>
                            }
                        }).collect_view()}
                    </div>
                </div>

                <div class="dcard">
                    <h3>"更新滞后分布"</h3>
                    <p class="dsub">"只统计榜上 "{data.datasets.len()}" 条有日期的明细；无日期单列，不当成 0 天"</p>
                    <div class="bars hist">
                        {lag_bars.into_iter().map(|b: Bar| {
                            let h = charts::bar_height(b.value, lag_max, 100.0);
                            view! {
                                <div class="bar-row">
                                    <span class="bar-label">{b.label}</span>
                                    <div class="bar-track"><div class="bar-fill" style=format!("height:{h:.1}%")></div></div>
                                    <span class="bar-num">{fmt_num(b.value)}</span>
                                </div>
                            }
                        }).collect_view()}
                    </div>
                </div>
            </div>
        </section>
    }
}

/// 页脚三个视图共用。详情页、证明页也要能一眼看到「这是哪天的数据、代码在哪」——
/// 只挂在榜单页的话，从群里点进来直接落在详情页的读者就找不到出处。
#[component]
fn SiteFooter(generated_at: String, total: u64, shown: usize) -> impl IntoView {
    view! {
        <footer class="site">
            <p>
                "体检于 " <span class="nw">{generated_at}</span> " · 共 " {fmt_num(total)}
                " 个数据集，展示前 " {shown.to_string()} " 个 · 数据来源 浙江省公共数据开放平台"
            </p>
            <p class="foot-links">
                <a href="#/checks">"判定规则（10 条检查项）"</a>
                " · "
                <a href="https://github.com/qiaopengjun5162/assay" target="_blank" rel="noopener">"源码"</a>
                " · "
                <a href="https://github.com/qiaopengjun5162/assay/releases/latest"
                   target="_blank" rel="noopener">"下载校验器（离线复核）"</a>
                " · "
                <a href="https://github.com/qiaopengjun5162/assay/blob/main/docs/DECISIONS.md"
                   target="_blank" rel="noopener">"方法与决策记录"</a>
                " · "
                <a href="https://github.com/qiaopengjun5162/assay/issues"
                   target="_blank" rel="noopener">"发现算错了？开 issue"</a>
            </p>
            <p class="hint">
                "这是一个纯静态页面：没有账号、没有埋点、不向服务器发任何写请求。"
                "「在浏览器里重算」用的是你自己这台机器上的 CPU。"
            </p>
        </footer>
    }
}

/// 判定规则独立页：`#/checks`。
///
/// 榜单页把这张表放在 500 行之后，正常读者不会滚到那儿；页脚和首屏都要能一键到这里。
fn checks_page(data: Data) -> impl IntoView {
    let total_checked = data.checks.first().map(|c| c.of).unwrap_or(0);
    let shown = data.datasets.len();
    view! {
        <nav class="crumb" aria-label="面包屑"><a href="#/">"榜单"</a>" / 判定规则"</nav>
        <h1 class="detail-title">"判定规则：每条结论的出处"</h1>
        <p class="lede">
            "这里列出全部检查项、阈值、命中数和它在排序里的权重。"
            "榜单上任何一条，都能在这张表里找到它为什么被排在那儿。"
        </p>
        <p class="verify"><a class="btn" href="#/">"← 回榜单"</a></p>
        {checks_section(&data.checks, total_checked, shown)}
    }
}

/// 部门索引 `#/depts`：首页只放前 12 个部门，这里放全量部门表。
/// 数字全部来自 `byDept`（覆盖全量），榜单条目仍只列前 500 名。
fn depts_index(data: Data, route: RwSignal<Route>) -> impl IntoView {
    let mut names: Vec<&String> = data.summary.by_dept.keys().collect();
    names.sort_by(|a, b| {
        let sa = data.summary.by_dept.get(*a).map(|v| v.stopped).unwrap_or(0);
        let sb = data.summary.by_dept.get(*b).map(|v| v.stopped).unwrap_or(0);
        sb.cmp(&sa).then(a.cmp(b))
    });
    let cards = names
        .into_iter()
        .map(|name| {
            let d = data.summary.by_dept.get(name).cloned();
            let href = format!("#/dept/{}", urlencoding(name));
            let pick = name.clone();
            let meta = d
                .map(|v| {
                    format!(
                        "{} 个数据集 · 停更 {} · 字段对不上 {}",
                        v.n, v.stopped, v.mismatch
                    )
                })
                .unwrap_or_default();
            view! {
                <a class="deptcard" href=href on:click=move |_| route.set(Route::Dept(pick.clone()))>
                    <div class="dname">{name.clone()}</div>
                    <div class="dmeta">{meta}</div>
                </a>
            }
        })
        .collect_view();
    view! {
        <nav class="crumb" aria-label="面包屑"><a href="#/">"榜单"</a>" / 全部部门"</nav>
        <h1 class="detail-title">"全部部门"</h1>
        <p class="lede">
            "共 "{fmt_num(data.summary.by_dept.len() as u64)}" 个部门，按停更数据集数量排序。"
            "点部门看它的全量体检数字——这里统计覆盖全部数据集，不只是榜上前 500 名。"
        </p>
        <p class="verify"><a class="btn" href="#/">"← 回榜单"</a></p>
        <div class="deptgrid">{cards}</div>
    }
    .into_any()
}

/// 证明索引 `#/proofs`：首页说「391 条有机器证明」却没给出处，这里把所有可证条目列出来。
/// 只收录 `proof` 非空的行——口径与榜单、证据完全同源，不自己造数。
fn proofs_index(data: Data, route: RwSignal<Route>) -> impl IntoView {
    let proven: Vec<(usize, Row)> = data
        .datasets
        .iter()
        .cloned()
        .enumerate()
        .filter(|(_, r)| r.proof.is_some())
        .collect();
    let count = proven.len();
    let rows = proven
        .into_iter()
        .map(|(i, r)| row_view(i, r, route))
        .collect_view();
    view! {
        <nav class="crumb" aria-label="面包屑"><a href="#/">"榜单"</a>" / 机器证明"</nav>
        <h1 class="detail-title">"机器证明索引"</h1>
        <p class="lede">
            "榜上有 "{fmt_num(count as u64)}" 条数据集的数值结论由 Lean 4 内核复核过。"
            "点开任意一条，进详情后点「查看证明详情」看定理与快照哈希。"
        </p>
        <p class="hint">
            "这里只列能证的条目。其余数据集要么排在证明覆盖之外，要么平台只给了表头、没有数据行——没有可证的命题，不是漏做。"
        </p>
        <p class="verify"><a class="btn" href="#/">"← 回榜单"</a></p>
        <section class="board">{rows}</section>
    }
    .into_any()
}

/// 10 条检查项的表格。榜单页尾和 `#/checks` 共用同一份渲染，
/// 避免两处阈值文案各写一遍就分叉。
fn checks_section(checks: &[CheckRow], total_checked: u64, shown: usize) -> impl IntoView {
    let rows_view = checks
        .iter()
        .map(|c| {
            let share = if c.of == 0 {
                String::new()
            } else {
                format!("{:.1}%", c.hits as f64 / c.of as f64 * 100.0)
            };
            let weight = if c.weight == 0.0 {
                "只标注".to_string()
            } else {
                format!("+{}", c.weight)
            };
            view! {
                <tr>
                    <td class="c-dim">{c.dim.clone()}</td>
                    <td class="c-title">
                        {c.title.clone()}
                        <span class="c-id">{c.id.clone()}</span>
                        {(!c.note.is_empty()).then(|| view! { <span class="c-note">{c.note.clone()}</span> })}
                    </td>
                    <td class="c-hit">{fmt_num(c.hits)}<span class="c-share">{share}</span></td>
                    <td class="c-w">{weight}</td>
                </tr>
            }
        })
        .collect_view();

    view! {
        <section class="checks" id="checks">
            <h2>"我们查了这 "{checks.len()}" 条，阈值全在这里"</h2>
            <p class="hint">
                "榜单上每个数字都出自这张表。命中数的分母是全量 "{fmt_num(total_checked)}
                " 条，不是榜单只展示的前 "{shown.to_string()}" 条。"
                "「问题分」是命中后加在排序分上的分值，「只标注」的项不进分数——"
                "不参与排序就该明说，而不是让人猜。"
            </p>
            <div class="tablewrap">
                <table class="checktable">
                    <thead>
                        <tr><th>"维度"</th><th>"检查项"</th><th>"命中"</th><th>"问题分"</th></tr>
                    </thead>
                    <tbody>{rows_view}</tbody>
                </table>
            </div>
            <p class="hint">
                "机器可读的版本：" <code>"assay checks --json"</code>
                " 或 data.json 的 "<code>"checks"</code>
                " 字段；定义源码在 "<code>"crates/core/src/checks.rs"</code>
                "。改阈值就是改结论，所以只允许在那里改一次。"
            </p>
        </section>
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

/// 部门页：全量数字来自 `byDept`（覆盖全部部门），列表只能列出**榜上前 500 名**里的那几条。
///
/// 两个数不是一回事，必须在页面上说清：N 是全量，M 是榜上能看到的。
/// 没进榜不等于没问题——榜单只发布前 500 名。上一轮「部门卡点进去只剩 1 条」
/// 就是没说这句话，被读成「藏了 268 条」。
fn dept_view(data: Data, name: &str, route: RwSignal<Route>) -> impl IntoView {
    let Some(d) = data.summary.by_dept.get(name).cloned() else {
        return view! {
            <nav class="crumb" aria-label="面包屑">
                <a href="#/" on:click=move |_| route.set(Route::List)>"榜单"</a>" / 部门"
            </nav>
            <section class="panel">
                <h1 class="detail-title">"找不到这个部门"</h1>
                <p class="hint">"部门名要和数据集登记的一致。回榜单用下拉框选，链接一定对。"</p>
                <p class="verify"><a class="btn" href="#/">"← 回榜单"</a></p>
            </section>
        }
        .into_any();
    };

    // 停更数量的名次：并列时按部门名定先后，写清楚，免得被当成「并列第 3」
    let mut order: Vec<(&String, u64)> = data
        .summary
        .by_dept
        .iter()
        .map(|(k, v)| (k, v.stopped))
        .collect();
    order.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    let total_depts = order.len();
    let rank = order
        .iter()
        .position(|(k, _)| k.as_str() == name)
        .map(|i| i + 1)
        .unwrap_or(0);

    // 榜上属于这个部门的行。保留全局名次，行首数字要和榜单一致。
    let listed: Vec<(usize, Row)> = data
        .datasets
        .iter()
        .cloned()
        .enumerate()
        .filter(|(_, r)| r.dept == name)
        .collect();
    let shown = listed.len();
    let audited = data.summary.audited;
    let share = if audited == 0 {
        String::new()
    } else {
        format!("{:.2}%", d.n as f64 / audited as f64 * 100.0)
    };
    let pct = |a: u64| {
        if d.n == 0 {
            String::new()
        } else {
            format!("{:.1}%", a as f64 / d.n as f64 * 100.0)
        }
    };
    let empty_avg = d
        .empty_avg
        .map(|v| format!("{:.1}%", v * 100.0))
        .unwrap_or_else(|| "无样例".to_string());
    let dept_href = format!("#/?dept={}", urlencoding(name));
    let rows_view = listed
        .iter()
        .map(|(i, r)| row_view(*i, r.clone(), route))
        .collect_view();
    let hidden = d.n.saturating_sub(shown as u64);

    view! {
        <nav class="crumb" aria-label="面包屑">
            <a href="#/" on:click=move |_| route.set(Route::List)>"榜单"</a>" / 部门"
        </nav>
        <h1 class="detail-title">{name.to_string()}</h1>
        <p class="kicker">
            "该部门共 "{fmt_num(d.n)}" 个数据集，占全量 "{share}" · 体检于 "
            <span class="nw">{data.generated_at.clone()}</span>
        </p>

        <section class="stats">
            <div class="stat bad">
                <div class="num">{fmt_num(d.stopped)}</div>
                <div class="lbl">{format!("标了停更（占部门内 {}）", pct(d.stopped))}</div>
            </div>
            <div class="stat warn">
                <div class="num">{fmt_num(d.mismatch)}</div>
                <div class="lbl">{format!("声明列数与实际不符（占 {}）", pct(d.mismatch))}</div>
            </div>
            <div class="stat">
                <div class="num">{empty_avg}</div>
                <div class="lbl">"样例级平均空值率"</div>
            </div>
            <div class="stat">
                <div class="num">{fmt_num(rank as u64)}</div>
                <div class="lbl">{format!("停更排名（共 {} 个部门）", fmt_num(total_depts as u64))}</div>
            </div>
        </section>

        <p class="filter-note">
            "这些是全量统计（"{fmt_num(audited)}" 个数据集都查过）。下面只列"
            <b>"榜上前 500 名"</b>"里属于这个部门的 "<b>{shown.to_string()}</b>" 条；"
            {(!name.is_empty() && hidden > 0).then(|| format!(
                "另外 {hidden} 条没进榜——榜只发布前 500 名，没进榜不代表没问题，只代表问题分没排进前 500。"
            ))}
            " 排名并列时按部门名定先后。"
        </p>

        {if shown == 0 {
            view! {
                <section class="panel">
                    <h2>"这个部门的榜单条目"</h2>
                    <p class="hint">
                        "该部门的 "{fmt_num(d.n)}" 个数据集都不在前 500 名内，所以这里没有可列的条目。"
                        "这不等于它们没问题：榜单只发布前 500 名。"
                    </p>
                    <p class="verify"><a class="btn" href="#/">"← 回榜单"</a></p>
                </section>
            }
            .into_any()
        } else {
            view! {
                <section class="board">
                    <h2 class="section">"榜上前 500 名里的这个部门"</h2>
                    {rows_view}
                </section>
                <p class="verify">
                    <a class="btn" href=dept_href.clone()>"在榜单里只看这 "{shown.to_string()}" 条"</a>
                    <a class="btn" href="#/">"← 回榜单"</a>
                </p>
            }
            .into_any()
        }}
        <p class="hint">
            "平均空值率只统计拿到样例的数据集（每份 5 行左右），拿不到样例的记作「无样例」，不是 0。"
        </p>
    }
    .into_any()
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
            if r.desc_len >= crate::constants::DESC_MIN {
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
        <nav class="crumb" aria-label="面包屑"><a href="#/" on:click=move |_| route.set(Route::List)>"榜单"</a>" / 数据集详情"</nav>
        <h1 class="detail-title">{r.name.clone()}</h1>
        <div class="meta-chips">
            <a class="chip" href=format!("#/?dept={}", urlencoding(&r.dept))
               title="回榜单，只看这个部门">{r.dept.clone()}</a>
            <span class="chip">{r.open_type.clone().unwrap_or_default()}</span>
            <span class="chip" title="问题分只用于把问题排先后，不是科学评分，也不代表数据真假">"问题分 " {format!("{:.1}", r.score)}</span>
            {r.update_time.clone().map(|t| view! { <span class="chip">"最近更新 " {t}</span> })}
            {r.overdue.then(|| view! { <span class="chip bad">"超过声明更新周期"</span> })}
            {r.proof.is_some().then(|| view! { <span class="chip ok">"已生成证明"</span> })}
            {report.then(|| view! { <span class="chip ok">"有书面报告"</span> })}
        </div>
        <div class=format!("verdict{}", if bad { " bad" } else { "" })>
            <b>{verdict}</b>
            {(!why.is_empty()).then(|| view! { <span>"：" {why.clone()}</span> })}
            " · 问题分只用于排序，不是科学指标。准确性不在评价范围内。"
            <p class="verify">
                <button class="btn btn-primary" on:click=move |_| scroll_to_evidence()>"↓ 跳到证据，当场重算"</button>
                <ShareLinkButton />
                {report.then(|| view! {
                    <a class="btn" href=report_href.clone() target="_blank" rel="noopener">"新标签看报告（.md）"</a>
                    <a class="btn" href=report_href.clone() download>"下载报告（.md 纯文本）"</a>
                })}
            </p>
        </div>

        <h2 class="section">"GB/T 36344 六维体检（准确性不评价）"</h2>
        <section class="dims">
            {dims.into_iter().map(|(n, std, v, note)| view! {
                <div class=format!("dim{}", if n == "准确性" { " na" } else { "" })>
                    // 国标条款号收进悬停：普通读者看「规范性/完整性」就够，
                    // 要对着国标核查的人把鼠标放上去就能拿到编号。
                    <h3 title=format!("GB/T 36344 对应条款：{std}")>{n}</h3>
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
            <section class="panel" id="evidence">
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
                        <pre class="mono-block" tabindex="0" role="region" aria-label="复核命令">{cmd.clone()}</pre>
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

/// 复制当前页地址。详情页的地址本身就能直达（hash 路由），
/// 但让用户自己去地址栏选中一长串 `%3D%3D` 不现实，所以给个按钮。
#[component]
fn ShareLinkButton() -> impl IntoView {
    let done = RwSignal::new(false);
    let err = RwSignal::new(String::new());

    let copy = move |_| {
        let url = web_sys::window()
            .and_then(|w| w.location().href().ok())
            .unwrap_or_default();
        if url.is_empty() {
            err.set("拿不到当前地址".to_string());
            return;
        }
        leptos::task::spawn_local(async move {
            match copy_to_clipboard(&url).await {
                Ok(()) => {
                    done.set(true);
                    err.set(String::new());
                }
                Err(e) => {
                    done.set(false);
                    err.set(e);
                }
            }
        });
    };

    view! {
        <button class="btn" on:click=copy>
            {move || if done.get() { "已复制，可直接粘贴" } else { "复制本条链接" }}
        </button>
        {move || {
            let e = err.get();
            (!e.is_empty()).then(|| view! { <span class="chip warn">{format!("复制失败：{e}")}</span> })
        }}
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
        // 数字不写死：证明覆盖面是随跑批变的，写死一个「前 100」就是下一条要分叉的口径。
        let done = data.summary.proof_count;
        return view! {
            <section class="panel"><h2>"这条没有证明"</h2>
            <p class="hint">{format!(
                "证明覆盖榜上能证的 {done} 条：这条要么排在覆盖范围之外，要么平台只给了表头、没有数据行——没有可证的命题。"
            )}</p></section>
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
    // 证明页的读者多半是从报告或群里直接点进来的，必须能一步回到判定现场。
    let back_id = r.id.clone();
    let xref_id = r.id.clone();
    let report_href = r.report_path();
    view! {
        <nav class="crumb" aria-label="面包屑">
            <a href="#/" on:click=move |_| route.set(Route::List)>"榜单"</a>" / "
            <a href=format!("#/detail/{}", urlencoding(&back_id))
               on:click=move |_| route.set(Route::Detail(back_id.clone()))>"详情"</a>" / 证明"
        </nav>
        <h1 class="detail-title">{r.name.clone()}</h1>
        <div class="pass-card">
            <div class="stamp">"PASS"</div>
            <div><b>"Lean 4 内核已复核"</b><p class="hint">"报告的每个计数都由定理证明器独立校验：改一个数字，内核立即报错。"</p></div>
        </div>
        <nav class="xrefs" aria-label="相关材料">
            <a class="btn" href=format!("#/detail/{}", urlencoding(&xref_id))
               on:click=move |_| route.set(Route::Detail(xref_id.clone()))>"← 返回详情"</a>
            {r.report.then(|| view! {
                <a class="btn" href=report_href.clone() target="_blank" rel="noopener">"新标签看报告（.md）"</a>
                <a class="btn" href=report_href.clone() download>"下载报告（.md 纯文本）"</a>
            })}
        </nav>
        <section class="panel">
            <h2>"内核复核明细"</h2>
            <table class="kv">
                <tr><th>"榜单序号"</th><td>{format!("第 {} 位", p.index + 1)}</td></tr>
                <tr><th>"样例规模"</th><td>{format!("{} 行 × {} 列", p.rows, p.cols)}</td></tr>
                <tr><th>"定理数量"</th><td>{format!("{} 条", p.cols)}</td></tr>
                <tr><th>"快照 SHA-256"</th><td>{p.sha256.clone()}</td></tr>
                <tr><th>"数据来源"</th><td>{p.source.clone().unwrap_or_else(|| "live".into())}</td></tr>
                <tr><th>"校验命令"</th><td>"lean TopProofs.lean"</td></tr>
            </table>
            <p><a class="btn" href="TopProofs.lean" download>"下载 TopProofs.lean"</a></p>
            <pre class="code" tabindex="0" role="region" aria-label="定理示例">{theorem}</pre>
            <p class="hint">"每一行是一条真实样例记录，1 表示空值；定理断言「第 N 列恰好有 K 条空值」，由 by decide 交给内核穷举。"</p>
        </section>
    }.into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    // 首屏导语必须和 exhibit 同源：无 lag 时绝不冒出「两年没动」这种编造的时效话。
    #[test]
    fn hero_claim_never_invents_staleness_without_lag() {
        let e = crate::model::Row {
            declared_cols: 7,
            sample_cols: 92,
            all_empty_cols: 46,
            lag_days: None,
            ..Default::default()
        };
        let text = hero_claim_text(Some(&e));
        assert!(text.contains("7 个字段"), "{text}");
        assert!(text.contains("92 列"), "{text}");
        assert!(text.contains("46 列整列是空的"), "{text}");
        assert!(!text.contains("更新"), "没有 lag 就不该提时效，{text}");
        assert!(!text.contains("两年"), "绝不补假时效，{text}");
    }

    // 真实 exhibit 的 update_cycle='999' 是平台停更码，绝不能误译成「每年更新」。
    #[test]
    fn hero_claim_reads_stopped_from_real_cycle_code() {
        let e = crate::model::Row {
            declared_cols: 7,
            sample_cols: 92,
            update_cycle: serde_json::json!("999"),
            lag_days: Some(35),
            ..Default::default()
        };
        let text = hero_claim_text(Some(&e));
        assert!(text.contains("平台标了停更"), "999 必须判停更，{text}");
        assert!(text.contains("距上次更新 35 天"), "{text}");
        assert!(!text.contains("每年更新"), "999 被误译，{text}");
        assert!(!text.contains("两年"), "lag 只有 35 天，{text}");
    }

    #[test]
    fn hero_claim_uses_real_lag_when_present() {
        let e = crate::model::Row {
            declared_cols: 3,
            sample_cols: 10,
            all_empty_cols: 0,
            lag_days: Some(123),
            ..Default::default()
        };
        let text = hero_claim_text(Some(&e));
        assert!(text.contains("距上次更新 123 天"), "{text}");
        assert!(!text.contains("整列是空的"), "0 列全空不该印，{text}");
    }

    #[test]
    fn hero_claim_handles_no_exhibit() {
        assert_eq!(hero_claim_text(None), "有问题的开放数据集，我们排成了榜。");
    }

    // 部门名里有中文和空格，链接是 encodeURIComponent 过的。
    // 这条路径一旦解析错，页面上会显示一串 %E5%AE%81…，而且不报错。
    #[test]
    fn dept_route_decodes_percent_escapes() {
        assert_eq!(
            parse_route("#/dept/%E5%AE%81%E6%B3%A2%E5%B8%82%E7%BB%9F%E8%AE%A1%E5%B1%80"),
            Route::Dept("宁波市统计局".into())
        );
    }

    #[test]
    fn dept_route_drops_query_string() {
        assert_eq!(parse_route("#/dept/foo?x=1"), Route::Dept("foo".into()));
    }

    // `dept/` 与 `detail/` 都以 d 开头，改前缀顺序时容易互相吃掉。
    #[test]
    fn detail_route_still_wins_over_dept() {
        assert_eq!(parse_route("#/detail/abc"), Route::Detail("abc".into()));
        assert_eq!(parse_route("#/proof/abc"), Route::Proof("abc".into()));
    }

    #[test]
    fn dept_without_slash_is_not_a_dept_page() {
        assert_eq!(parse_route("#/dept"), Route::List);
        assert_eq!(parse_route(""), Route::List);
    }

    // 顶栏两个新入口必须能解析，否则点「部门」「证明」只会回到榜单。
    #[test]
    fn index_routes_parse() {
        assert_eq!(parse_route("#/depts"), Route::Depts);
        assert_eq!(parse_route("#/proofs"), Route::Proofs);
    }

    // 单条路由不能被列表路由吃掉：`dept/` 前缀 must win over `depts`。
    #[test]
    fn single_item_routes_win_over_index_routes() {
        assert_eq!(parse_route("#/dept/foo"), Route::Dept("foo".into()));
        assert_eq!(parse_route("#/proof/foo"), Route::Proof("foo".into()));
    }
}
