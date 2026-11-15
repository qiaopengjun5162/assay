//! 配置读取：模型 key 只从环境变量或本机配置读，绝不打印、绝不入库。

/// 解释层使用的模型。
pub const MODEL: &str = "deepseek-flash";

/// 读取 LLM key。优先级：
/// 1. `DATAPROOF_LLM_KEY` 环境变量
/// 2. Hermes profile 的 `~/.hermes/profiles/ai/config.yaml`（本地开发）
///
/// 找不到就返回 None，调用方应给出可操作的报错，而不是 panic。
pub fn llm_key() -> Option<String> {
    if let Ok(k) = std::env::var("DATAPROOF_LLM_KEY") {
        let k = k.trim().to_string();
        if !k.is_empty() {
            return Some(k);
        }
    }
    let path = std::env::var("DATAPROOF_CONFIG")
        .unwrap_or_else(|_| format!("{}/.hermes/profiles/ai/config.yaml", home()));
    read_key_from_yaml(&path)
}

fn home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| ".".into())
}

fn read_key_from_yaml(path: &str) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let lines: Vec<&str> = text.lines().collect();
    // 官方 anthropic 兼容端点优先；没有再退到任意 deepseek 配置块
    let mut hits: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.contains("api.deepseek.com"))
        .map(|(i, _)| i)
        .collect();
    hits.sort_by_key(|i| !lines[*i].contains("/anthropic"));
    for i in hits {
        let start = i.saturating_sub(4);
        let end = (i + 5).min(lines.len());
        for l in &lines[start..end] {
            let t = l.trim();
            if let Some(rest) = t.strip_prefix("api_key:") {
                let v = rest.trim().trim_matches('"').trim_matches('\'');
                if !v.is_empty() {
                    return Some(v.to_string());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn reads_key_next_to_endpoint() {
        let path = std::env::temp_dir().join("dp-config-test.yaml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "providers:").unwrap();
        writeln!(f, "  deepseek:").unwrap();
        writeln!(f, "    base_url: https://api.deepseek.com/anthropic").unwrap();
        writeln!(f, "    api_key: sk-test-123").unwrap();
        assert_eq!(
            read_key_from_yaml(path.to_str().unwrap()),
            Some("sk-test-123".into())
        );
    }

    #[test]
    fn missing_file_is_none() {
        assert!(read_key_from_yaml("/nonexistent/config.yaml").is_none());
    }
}
