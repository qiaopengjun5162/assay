//! 打审计包：把一次体检的产物压成一个可交付的单文件。
//!
//! 目标是 `docs/SUBMISSION.md` 的第一优先：**验的人不需要装我们的东西**。
//! 所以包本身必须自解释——解压后有 `VERIFY.md` 写明怎么做，`MANIFEST.sha256`
//! 是标准 `shasum` 格式，一条系统命令就能验完全部文件。
//!
//! 两条原则：
//!
//! 1. **不重新计算任何业务口径**。`evidence.json`、`proofs.json`、`summary.json`
//!    原样进包，一个字不改。包只是搬运工。重新生成一遍等于让包里出现「我们这次
//!    算的另一种结果」，那正是可复核最怕的东西。
//! 2. **可重现**。mtime 固定 0、uid/gid 固定 0、文件顺序排序。同样的输入打两次，
//!    字节完全一致——这样「包的哈希」本身也能当指纹用。

use crate::{Error, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};

/// 进包的文件清单。顺序固定 = 打包结果可重现。
const ROOT_FILES: [&str; 4] = [
    "summary.json",
    "evidence.json",
    "proofs.json",
    "TopProofs.lean",
];

pub struct PackOptions<'a> {
    /// 跑批产物目录，如 `out-full`
    pub out_dir: &'a str,
    /// 输出文件，如 `dist/assay-audit-2026-11-18.tar.gz`
    pub dest: &'a str,
    /// 体检基准日 `YYYY-MM-DD`，写进 MANIFEST 与包内目录名
    pub captured_at: &'a str,
    pub commit: Option<&'a str>,
}

#[derive(Debug)]
pub struct PackOutcome {
    pub files: usize,
    pub evidence_entries: usize,
    pub bytes_raw: u64,
    pub bytes_packed: u64,
}

struct Item {
    /// 包内相对路径，始终用 `/` 分隔
    name: String,
    source: PathBuf,
    bytes: u64,
    sha256: String,
}

pub fn pack(opts: &PackOptions<'_>) -> Result<PackOutcome> {
    validate_date(opts.captured_at)?;
    let out = Path::new(opts.out_dir);
    let root = format!("assay-audit-{}", opts.captured_at);

    let mut items: Vec<Item> = Vec::new();
    for f in ROOT_FILES {
        let p = out.join(f);
        // TopProofs.lean 可能不存在（没跑 prove 时），其余三个是硬要求
        if !p.is_file() {
            if f == "TopProofs.lean" {
                continue;
            }
            return Err(Error::Other(format!(
                "缺少 {}，先跑完 audit → evidence → prove → report 再打包",
                p.display()
            )));
        }
        items.push(measure(&p, f)?);
    }
    for p in evidence_files(&out.join("evidence"))? {
        let rel = p
            .strip_prefix(out)
            .map_err(|_| Error::Other(format!("路径越界：{}", p.display())))?;
        let name = slash(rel);
        items.push(measure(&p, &name)?);
    }

    let entries = evidence_entries(&out.join("evidence.json"))?;
    let raw: u64 = items.iter().map(|i| i.bytes).sum();

    let manifest = json!({
        "tool": "assay",
        "version": env!("CARGO_PKG_VERSION"),
        "capturedAt": opts.captured_at,
        "commit": opts.commit,
        // 明确写死范围：不写清楚，买方会以为这是全量证据
        "scope": "sample",
        "scopeNote": "证据为样例级（受证据生成数量限制），指标来自目录全量元数据；\
                      两者在 README 与页面文案中都按此口径表述。",
        "evidenceEntries": entries.len(),
        "files": items
            .iter()
            .map(|i| json!({ "path": i.name, "bytes": i.bytes, "sha256": i.sha256 }))
            .collect::<Vec<_>>(),
        // 组合指纹：H(detail ‖ "\n--split--\n" ‖ preview)，与 evidence.json 同口径。
        // 它证明「这两个响应属于同一次抓取」，单文件哈希证明不了这件事。
        "entries": entries,
        "verify": "shasum -a 256 -c MANIFEST.sha256",
    });
    let manifest_text = serde_json::to_string_pretty(&manifest)?;
    let verify_md = verify_doc(
        opts.captured_at,
        entries.len(),
        items.len() + 3,
        opts.commit,
    )?;

    // MANIFEST.sha256 覆盖 MANIFEST.json 自身，形成完整链：改任何一个字节都对不上。
    // 它不包含自己（自我引用做不到）。
    let mut sha_lines = String::new();
    for (name, body) in [
        ("MANIFEST.json", manifest_text.as_bytes()),
        ("VERIFY.md", verify_md.as_bytes()),
    ] {
        sha_lines.push_str(&format!("{}  {}\n", hex(&Sha256::digest(body)), name));
    }
    for i in &items {
        sha_lines.push_str(&format!("{}  {}\n", i.sha256, i.name));
    }

    if let Some(dir) = Path::new(opts.dest).parent() {
        if !dir.as_os_str().is_empty() {
            fs::create_dir_all(dir)?;
        }
    }
    let file = File::create(opts.dest)?;
    let enc = flate2::GzBuilder::new()
        // 固定 mtime：否则同一份产物每次打包的字节都不同，包哈希就当不了指纹
        .mtime(0)
        .write(file, flate2::Compression::best());
    let mut tar = tar::Builder::new(enc);

    append_bytes(&mut tar, &root, "MANIFEST.json", manifest_text.as_bytes())?;
    append_bytes(&mut tar, &root, "VERIFY.md", verify_md.as_bytes())?;
    append_bytes(&mut tar, &root, "MANIFEST.sha256", sha_lines.as_bytes())?;
    for i in &items {
        append_file(&mut tar, &root, &i.name, &i.source)?;
    }
    let enc = tar.into_inner()?;
    enc.finish()?.flush()?;

    let bytes_packed = fs::metadata(opts.dest)?.len();
    Ok(PackOutcome {
        files: items.len() + 3,
        evidence_entries: entries.len(),
        bytes_raw: raw,
        bytes_packed,
    })
}

fn measure(path: &Path, name: &str) -> Result<Item> {
    let (sha256, bytes) = sha256_file(path)?;
    Ok(Item {
        name: name.to_string(),
        source: path.to_path_buf(),
        bytes,
        sha256,
    })
}

fn append_bytes<W: Write>(
    tar: &mut tar::Builder<W>,
    root: &str,
    name: &str,
    body: &[u8],
) -> Result<()> {
    let mut h = header(body.len() as u64);
    tar.append_data(&mut h, format!("{root}/{name}"), body)
        .map_err(|e| Error::Other(format!("打包 {name} 失败：{e}")))
}

/// 流式写：证据总量可能到几百 MB，不能整份读进内存。
fn append_file<W: Write>(
    tar: &mut tar::Builder<W>,
    root: &str,
    name: &str,
    source: &Path,
) -> Result<()> {
    let f = File::open(source)?;
    let size = f.metadata()?.len();
    let mut h = header(size);
    tar.append_data(&mut h, format!("{root}/{name}"), BufReader::new(f))
        .map_err(|e| Error::Other(format!("打包 {name} 失败：{e}")))
}

fn header(size: u64) -> tar::Header {
    let mut h = tar::Header::new_gnu();
    h.set_size(size);
    h.set_mode(0o644);
    // 全零元数据：让同样输入产出同样字节，包哈希才能当指纹
    h.set_mtime(0);
    h.set_uid(0);
    h.set_gid(0);
    h.set_cksum();
    h
}

fn evidence_files(dir: &Path) -> Result<Vec<PathBuf>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut dirs: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    let mut out = Vec::with_capacity(dirs.len() * 2);
    for d in dirs {
        for name in ["detail.json", "preview.json"] {
            let p = d.join(name);
            if p.is_file() {
                out.push(p);
            }
        }
    }
    Ok(out)
}

/// 从 `evidence.json` 里取组合指纹。**原样搬运，不重算**：重算等于在包里放
/// 「我们这次算的另一个值」，那正是可复核要避免的。
fn evidence_entries(path: &Path) -> Result<Vec<Value>> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let v: Value = serde_json::from_slice(&fs::read(path)?)?;
    let items = v["items"].as_array().cloned().unwrap_or_default();
    Ok(items
        .into_iter()
        .filter_map(|e| {
            let id = e["id"].as_str()?;
            Some(json!({
                "id": id,
                "name": e["name"],
                "sha256": e["sha256"],
                "files": e["files"],
            }))
        })
        .collect())
}

fn verify_doc(
    captured_at: &str,
    entries: usize,
    files: usize,
    commit: Option<&str>,
) -> Result<String> {
    let commit_line = commit
        .map(|c| format!("- 产生产物的 commit：`{c}`\n"))
        .unwrap_or_default();
    // 清单不列自己，所以列进行数比文件总数少 1
    let listed = files.saturating_sub(1);
    Ok(format!(
        r#"# 怎么验证这个包

包内容：{files} 个文件（其中 {listed} 条列进 `MANIFEST.sha256`——清单不列自己）、
{entries} 条证据，体检基准日 {captured_at}。
{commit_line}
## 一条命令验完所有文件

```bash
shasum -a 256 -c MANIFEST.sha256
```

macOS / Linux 自带 `shasum`。Windows 用 PowerShell：

```powershell
Get-Content MANIFEST.sha256 | ForEach-Object {{
  $h, $p = $_ -split '  ', 2
  if ((Get-FileHash $p -Algorithm SHA256).Hash.ToLower() -ne $h) {{ Write-Error "不匹配：$p" }}
}}
```

全部 `OK` 说明包内没有任何一个字节被改过。**这一步不需要装我们的软件。**

## 两级校验的区别

- **单文件哈希**（`MANIFEST.sha256`）：证明「这个文件没被改」。
- **组合指纹**（`MANIFEST.json` 的 `entries[].sha256`）：证明「同一条数据的
  detail 与 preview 来自同一次抓取」，算法是 `SHA-256(detail ‖ "\n--split--\n" ‖ preview)`。
  单文件哈希证明不了这件事——把别人家的两个响应拼在一起，单文件哈希照样全对。

组合指纹的复核需要工具，`assay verify-evidence` 做的就是这个（本项目内部用）。
包交给外部时，只要单文件哈希通过、而 `entries[].sha256` 与 `evidence.json`
里的记录一致，就已经排除了「事后调包单个文件」这一类问题。

## 范围声明

- **证据是样例级**：{entries} 条，受证据生成数量限制，不是全部数据集。
- **指标来自目录全量元数据**，与证据的条数不是一回事。
- 本项目**明确不评价数据准确性**——没有权威源，评了就是编。这里评的是
  规范性（字段是否一致、是否有空值、更新是否停更）与可复核性。

## 文件说明

| 文件 | 是什么 |
|---|---|
| `summary.json` | 本次体检的聚合指标 |
| `evidence.json` | 证据索引（每条的组合指纹与文件清单）|
| `evidence/` | 原始接口响应，`detail.json` + `preview.json`，未做任何改写 |
| `proofs.json` | 每条证明的结论与哈希 |
| `TopProofs.lean` | Lean 4 定理源文件，可独立交给 Lean 校验 |
| `MANIFEST.json` | 清单与元信息 |
| `MANIFEST.sha256` | 标准 shasum 清单 |
"#
    ))
}

fn sha256_file(path: &Path) -> Result<(String, u64)> {
    let mut f = File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    let mut total = 0u64;
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
        total += n as u64;
    }
    Ok((hex(&h.finalize()), total))
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// 包内路径一律 `/`：Windows 上 `Path::display()` 会给 `\`，解压到 Unix 就是一堆
/// 名字带反斜杠的平铺文件。
fn slash(p: &Path) -> String {
    p.components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("/")
}

fn validate_date(s: &str) -> Result<()> {
    let b = s.as_bytes();
    let ok = b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit());
    if !ok {
        return Err(Error::Other(format!(
            "--captured-at 要 YYYY-MM-DD（零填充），收到：{s}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slash_gives_posix_separators() {
        assert_eq!(
            slash(Path::new("evidence/a__/detail.json")),
            "evidence/a__/detail.json"
        );
    }

    #[test]
    fn date_validation_rejects_sloppy_input() {
        assert!(validate_date("2026-11-18").is_ok());
        for bad in ["2026/11/18", "2026-1-8", ""] {
            assert!(validate_date(bad).is_err(), "本该拒绝 {bad:?}");
        }
    }

    #[test]
    fn pack_is_reproducible_and_hashes_line_up() {
        let dir = std::env::temp_dir().join("assay-pack-test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("evidence/abc__")).unwrap();
        fs::write(dir.join("summary.json"), br#"{"audited":1}"#).unwrap();
        fs::write(
            dir.join("evidence.json"),
            r#"{"items":[{"id":"abc==","name":"样例","sha256":"deadbeef","files":["evidence/abc__/detail.json"]}]}"#
                .as_bytes(),
        )
        .unwrap();
        fs::write(dir.join("proofs.json"), b"[]").unwrap();
        fs::write(dir.join("evidence/abc__/detail.json"), b"{}").unwrap();
        fs::write(dir.join("evidence/abc__/preview.json"), b"{}").unwrap();

        let a = dir.join("a.tar.gz");
        let b = dir.join("b.tar.gz");
        let mk = |dest: &Path| {
            let dest = dest.to_string_lossy().to_string();
            pack(&PackOptions {
                out_dir: dir.to_str().unwrap(),
                dest: &dest,
                captured_at: "2026-11-18",
                commit: Some("abc1234"),
            })
            .unwrap()
        };
        let oa = mk(&a);
        mk(&b);
        assert_eq!(oa.evidence_entries, 1);
        assert!(oa.files >= 6, "至少 4 个根文件 + 2 个证据文件 + MANIFEST");

        let bytes_a = fs::read(&a).unwrap();
        let bytes_b = fs::read(&b).unwrap();
        assert_eq!(
            bytes_a, bytes_b,
            "同样输入两次打包必须字节一致（mtime 已固定为 0）"
        );

        // 解包核对：MANIFEST.sha256 的每一行都必须与包内实际文件对得上
        let dec = flate2::read::GzDecoder::new(&bytes_a[..]);
        let mut ar = tar::Archive::new(dec);
        let mut found: Vec<(String, Vec<u8>)> = Vec::new();
        for e in ar.entries().unwrap() {
            let mut e = e.unwrap();
            let path = e.path().unwrap().to_string_lossy().to_string();
            let mut buf = Vec::new();
            e.read_to_end(&mut buf).unwrap();
            found.push((path, buf));
        }
        let sha = found
            .iter()
            .find(|(p, _)| p.ends_with("MANIFEST.sha256"))
            .map(|(_, b)| String::from_utf8(b.clone()).unwrap())
            .expect("包里必须有 MANIFEST.sha256");
        assert!(!sha.is_empty());
        for line in sha.lines() {
            let (want, name) = line.split_once("  ").expect("shasum 格式是 两个空格");
            let (path, body) = found
                .iter()
                .find(|(p, _)| p.ends_with(name))
                .unwrap_or_else(|| panic!("清单里列了 {name}，包里没有"));
            assert!(
                path.starts_with("assay-audit-2026-11-18/"),
                "包根目录名不对：{path}"
            );
            assert_eq!(hex(&Sha256::digest(body)), want, "{name} 的哈希对不上");
        }

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_required_artifact_fails_loudly() {
        let dir = std::env::temp_dir().join("assay-pack-missing");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("summary.json"), b"{}").unwrap();
        let dest = dir.join("x.tar.gz").to_string_lossy().to_string();
        let err = pack(&PackOptions {
            out_dir: dir.to_str().unwrap(),
            dest: &dest,
            captured_at: "2026-11-18",
            commit: None,
        })
        .unwrap_err();
        assert!(
            err.to_string().contains("evidence.json"),
            "报错要说清缺哪个：{err}"
        );
    }
}
