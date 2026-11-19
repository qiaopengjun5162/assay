//! 复核命令的纯字符串逻辑。刻意不依赖 Leptos / web-sys，
//! 这样能在 native 下单测——命令少一个字符，评委就验不了。

/// 把站点内的相对路径拼成绝对 URL。
///
/// 页面印出的命令会被评委复制到自己的电脑上跑。`curl -O evidence/...` 没有主机，
/// 干净环境下必然 `Could not resolve host`；开着代理时更糟，会拿到一张错误页
/// 假装成功。所以命令里必须是绝对地址。
///
/// `base` 取页头地址：`location.origin()` + `location.pathname()`，
/// 形如 `https://user.github.io/assay/`（结尾斜杠）或本地 `http://localhost:8080/`。
pub fn absolute_url(base: &str, file: &str) -> String {
    let base = base.trim_end_matches('/');
    let file = file.trim_start_matches('/');
    format!("{base}/{file}")
}

/// 生成一条可复制到任意机器执行的 SHA-256 复核命令。
///
/// 字节顺序必须与 `evidence::capture_one` 一致：detail ‖ "\n--split--\n" ‖ preview。
/// 文件格式不对（不足两个路径 / 没有目录）时返回空串，页面不显示空命令。
pub fn verify_command(base: &str, files: &[String]) -> String {
    if files.len() < 2 {
        return String::new();
    }
    let Some((dir, _)) = files.first().and_then(|f| f.rsplit_once('/')) else {
        return String::new();
    };
    let detail = files
        .first()
        .map(|f| absolute_url(base, f))
        .unwrap_or_default();
    let preview = files
        .get(1)
        .map(|f| absolute_url(base, f))
        .unwrap_or_default();
    format!(
        r#"mkdir -p {dir} && cd {dir} && curl -O {detail} && curl -O {preview}
(cat detail.json; printf '\n--split--\n'; cat preview.json) | shasum -a 256"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_path_becomes_absolute() {
        assert_eq!(
            absolute_url(
                "https://qiaopengjun5162.github.io/assay/",
                "evidence/a/detail.json"
            ),
            "https://qiaopengjun5162.github.io/assay/evidence/a/detail.json"
        );
    }

    #[test]
    fn base_without_trailing_slash_also_works() {
        assert_eq!(
            absolute_url("http://localhost:8080", "/evidence/a/preview.json"),
            "http://localhost:8080/evidence/a/preview.json"
        );
    }

    #[test]
    fn command_uses_absolute_urls_and_keeps_split_marker() {
        let files: Vec<String> = [
            "evidence/HuZoVx_QGmxtyEgbHu_Ulw__/detail.json",
            "evidence/HuZoVx_QGmxtyEgbHu_Ulw__/preview.json",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let cmd = verify_command("https://example.github.io/assay/", &files);
        assert!(
            cmd.contains("curl -O https://example.github.io/assay/evidence/HuZoVx_QGmxtyEgbHu_Ulw__/detail.json"),
            "命令里必须是绝对 URL，不能是相对的 evidence/...：{cmd}"
        );
        assert!(
            !cmd.contains("-O evidence/"),
            "相对 URL 会 curl 失败：{cmd}"
        );
        assert!(
            cmd.contains("printf '\\n--split--\\n'"),
            "分隔符丢了：{cmd}"
        );
        assert!(cmd.contains("shasum -a 256"), "少了哈希命令：{cmd}");
        assert!(cmd.starts_with("mkdir -p evidence/HuZoVx_QGmxtyEgbHu_Ulw__ && cd "));
    }

    #[test]
    fn malformed_files_produce_no_command() {
        assert_eq!(verify_command("https://x.io/", &[]), "");
        // 没有目录层级时抽不出工作目录，宁可不显示也不给一条会跑坏的命令
        let one = vec!["detail.json".to_string()];
        assert_eq!(verify_command("https://x.io/", &one), "");
    }
}
