//! 配置读取：模型 key 只从环境变量或本机配置读，绝不打印、绝不入库。

/// 解释层使用的模型。
pub const MODEL: &str = "deepseek-flash";

/// 新名环境变量。
pub const ENV_LLM_KEY: &str = "ASSAY_LLM_KEY";
/// 新名环境变量：覆盖配置文件路径。
pub const ENV_CONFIG: &str = "ASSAY_CONFIG";

/// 改名过渡期兼容的旧名。**不要新增引用**，只在读取时兜底。
const LEGACY_LLM_KEY: &str = "DATAPROOF_LLM_KEY";
const LEGACY_CONFIG: &str = "DATAPROOF_CONFIG";

/// 读取 LLM key。优先级：
/// 1. `ASSAY_LLM_KEY` 环境变量（旧名 `DATAPROOF_LLM_KEY` 仍兼容）
/// 2. Hermes profile 的 `~/.hermes/profiles/ai/config.yaml`（本地开发，
///    路径可用 `ASSAY_CONFIG` / `DATAPROOF_CONFIG` 覆盖）
///
/// 找不到就返回 None，调用方应给出可操作的报错，而不是 panic。
///
/// 为什么双读而不是直接改名：本机 Hermes 配置里写的是旧名，一次性切换会让
/// `explain` 静默报「找不到 key」。旧名先保留一个发布周期，用 `legacy_env`
/// 给出提示，等外部配置都迁完再删。
pub fn llm_key() -> Option<String> {
    if let Some((name, k)) = read_env(&[ENV_LLM_KEY, LEGACY_LLM_KEY]) {
        if name == LEGACY_LLM_KEY {
            warn_legacy(name, ENV_LLM_KEY);
        }
        return Some(k);
    }
    let path = match read_env(&[ENV_CONFIG, LEGACY_CONFIG]) {
        Some((name, v)) => {
            if name == LEGACY_CONFIG {
                warn_legacy(name, ENV_CONFIG);
            }
            v
        }
        None => format!("{}/.hermes/profiles/ai/config.yaml", home()),
    };
    read_key_from_yaml(&path)
}

/// 按顺序取第一个非空环境变量，返回 (命中的变量名, 值)。
fn read_env(names: &[&'static str]) -> Option<(&'static str, String)> {
    for name in names {
        if let Ok(v) = std::env::var(name) {
            let v = v.trim().to_string();
            if !v.is_empty() {
                return Some((name, v));
            }
        }
    }
    None
}

/// 旧名只提示一次，避免每次调用都刷屏。
fn warn_legacy(legacy: &str, current: &str) {
    use std::sync::Once;
    static WARNED: Once = Once::new();
    WARNED.call_once(|| {
        eprintln!("提示：环境变量 {legacy} 已改名，请改用 {current}（旧名下个版本移除）");
    });
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
    use std::sync::Mutex;

    /// 环境变量是进程级共享状态，改它的用例必须串行。
    static ENV_LOCK: Mutex<()> = Mutex::new(());

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

    #[test]
    fn read_env_takes_first_non_empty() {
        // 用独立变量名，避免和真实配置项抢环境
        std::env::set_var("ASSAY_TEST_ENV_A", "");
        std::env::set_var("ASSAY_TEST_ENV_B", "b-value");
        let names: &[&'static str] = &["ASSAY_TEST_ENV_A", "ASSAY_TEST_ENV_B"];
        assert_eq!(
            read_env(names),
            Some(("ASSAY_TEST_ENV_B", "b-value".to_string()))
        );
        std::env::remove_var("ASSAY_TEST_ENV_A");
        std::env::remove_var("ASSAY_TEST_ENV_B");
    }

    #[test]
    fn new_env_name_wins_over_legacy() {
        // 双读过渡期的核心约束：新名优先，旧名只兜底。
        // 环境变量是进程级共享状态，下面两个用例必须串行。
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var(ENV_LLM_KEY, "new-key");
        std::env::set_var(LEGACY_LLM_KEY, "old-key");
        assert_eq!(llm_key(), Some("new-key".into()));
        std::env::remove_var(ENV_LLM_KEY);
        std::env::remove_var(LEGACY_LLM_KEY);
    }

    #[test]
    fn legacy_env_name_still_readable() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var(ENV_LLM_KEY);
        std::env::set_var(LEGACY_LLM_KEY, "old-key");
        assert_eq!(llm_key(), Some("old-key".into()));
        std::env::remove_var(LEGACY_LLM_KEY);
    }
}
