-- 查询后端的初始 schema。
--
-- 设计原则：**证据不进库**。原始响应、哈希、Lean 证明都是静态对象，
-- 库只存「谁在什么时候跑了哪次体检、每条结果是多少」。
-- 理由见 docs/DECISIONS.md D20：证据依赖服务在线才能验，就等于不可验。
--
-- 分两张表而不是一张宽表：
--   dataset     —— 数据集的身份（跨 run 不变）
--   observation —— 某次 run 里某条数据集的指标（每次 run 一行）
-- 这样「历史」是 observation 的行数增长，而不是 dataset 的列增长。

create table if not exists platform (
  id          smallint primary key,
  code        text not null unique,
  name        text not null,
  base_url    text
);

insert into platform (id, code, name, base_url)
values (1, 'zj', '浙江省公共数据开放平台', 'https://data.zjzwfw.gov.cn')
on conflict (id) do nothing;

create table if not exists run (
  id           bigserial primary key,
  platform_id  smallint not null references platform (id),
  -- ISO 文本而不是 date：ISO 日期按字符串排序即按时间排序，
  -- 为此引入一个时间库（chrono 已被判 unmaintained）不值得。
  -- core 里的公历换算本来就是自己写的，见 util::days_from_civil。
  captured_at  text not null check (captured_at ~ '^\d{4}-\d{2}-\d{2}$'),
  commit_sha   text,
  audited      integer not null,
  with_preview integer not null,
  -- 聚合结果整份留着：新增指标不需要改 schema，也不丢历史口径
  raw_summary  jsonb not null default '{}'::jsonb,
  unique (platform_id, captured_at)
);

create table if not exists dataset (
  id          bigserial primary key,
  platform_id smallint not null references platform (id),
  ext_id      text not null,
  name        text not null,
  dept        text,
  unique (platform_id, ext_id)
);

create table if not exists observation (
  run_id         bigint not null references run (id) on delete cascade,
  dataset_id     bigint not null references dataset (id) on delete cascade,
  score          real not null,
  lag_days       integer,
  -- 平台声明的更新周期，随每次抓取可能变（-1/999/不更新 = 停更）
  update_cycle   text not null default '',
  empty_rate     real not null,
  declared_cols  integer not null,
  sample_cols    integer not null,
  all_empty_cols integer not null,
  schema_kind    text not null,
  stopped        boolean not null,
  has_proof      boolean not null default false,
  primary key (run_id, dataset_id)
);

-- 榜单默认按问题分降序，走这条索引
create index if not exists observation_run_score_idx
  on observation (run_id, score desc);

-- 部门筛选与搜索都打 dataset，按平台前缀走
create index if not exists dataset_platform_dept_idx
  on dataset (platform_id, dept);

-- 中文名搜索用 trigram 会更好，但 pg_trgm 需要额外扩展；
-- 先用前缀/包含索引兜住「按部门查」这个主路径，模糊搜索留给数据量真上来时再优化。
create index if not exists dataset_name_idx
  on dataset (platform_id, name);
