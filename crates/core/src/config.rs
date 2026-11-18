//! 配置读取：模型地址、模型名、key 只从环境变量或本机配置读，绝不打印、绝不入库。
//!
//! 为什么不是几个常量：解释层原来把 `https://api.deepseek.com/anthropic` 写死在
//! `call_model` 里，key 的读取也只认这一个域名。一旦换渠道（本机换成
//! `https://platform.yeta.ai/v1`），`explain` 就报「找不到 LLM key」——不是 key 没了，
//! 是读法只认老地址。渠道会换，代码不该跟着改。

use std::fs;

/// 兜底模型名。配置里也找不到时才用。
pub const DEFAULT_MODEL: &str = "deepseek-flash";
/// 兜底端点：DeepSeek 的 Anthropic 兼容口。
pub const DEFAULT_BASE: &str = "https://api.deepseek.com/anthropic";

/// 新名环境变量。
pub const ENV_LLM_KEY: &str = "ASSAY_LLM_KEY";
/// 覆盖端点，例如 `https://platform.yeta.ai/v1`。
pub const ENV_LLM_BASE: &str = "ASSAY_LLM_BASE";
/// 覆盖模型名，例如 `deepseek-v4.1-flash`。
pub const ENV_LLM_MODEL: &str = "ASSAY_LLM_MODEL";
/// 覆盖代理，例如 `socks5h://127.0.0.1:7890`。
pub const ENV_LLM_PROXY: &str = "ASSAY_LLM_PROXY";
/// 新名环境变量：覆盖配置文件路径。
pub const ENV_CONFIG: &str = "ASSAY_CONFIG";

/// 改名过渡期兼容的旧名。**不要新增引用**，只在读取时兜底。
const LEGACY_LLM_KEY: &str = "DATAPROOF_LLM_KEY";
const LEGACY_CONFIG: &str = "DATAPROOF_CONFIG";

/// 两家上游的请求格式不一样，只有这两处差异。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    /// `/v1/messages`，`x-api-key` 头，`system` 是顶层字段。
    Anthropic,
    /// `/v1/chat/completions`，`Authorization: Bearer`，`system` 是第一条 message。
    OpenAi,
}

impl Protocol {
    pub fn as_str(self) -> &'static str {
        match self {
            Protocol::Anthropic => "anthropic-messages",
            Protocol::OpenAi => "openai-chat",
        }
    }
}

/// 配置从哪儿来的。`explain --probe` 会把它打出来，省得猜「为什么用的是这个 key」。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// 全部来自环境变量。
    Env,
    /// 来自本机配置文件。
    File(String),
    /// 什么配置都没有，用的全是兜底值。
    Default,
}

#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    /// 只在显式配置时使用。不读 `HTTPS_PROXY`：本机代理端口经常漂，
    /// 误读一个不通的代理比不读更难查。
    pub proxy: Option<String>,
    pub source: Source,
}

impl LlmConfig {
    /// 按 环境变量 → 本机配置 → 兜底值 的顺序解析。
    ///
    /// 三段分别独立取，所以可以只设 `ASSAY_LLM_MODEL` 换模型、继续用配置文件里的 key。
    pub fn resolve() -> std::result::Result<Self, String> {
        let path = config_path();
        let file = read_llm_from_yaml(&path);
        let env_key = read_env(&[ENV_LLM_KEY, LEGACY_LLM_KEY]);

        let api_key = match &env_key {
            Some((name, k)) => {
                if *name == LEGACY_LLM_KEY {
                    warn_legacy(name, ENV_LLM_KEY);
                }
                k.clone()
            }
            None => file
                .as_ref()
                .and_then(|y| y.api_key.clone())
                .ok_or_else(|| {
                    format!("找不到 LLM key：设置 {ENV_LLM_KEY}，或在 {path} 里写 api_key")
                })?,
        };

        let base_url = read_env(&[ENV_LLM_BASE])
            .map(|(_, v)| v)
            .or_else(|| file.as_ref().and_then(|y| y.base_url.clone()))
            .unwrap_or_else(|| DEFAULT_BASE.to_string());
        let model = read_env(&[ENV_LLM_MODEL])
            .map(|(_, v)| v)
            .or_else(|| file.as_ref().and_then(|y| y.model.clone()))
            .unwrap_or_else(|| DEFAULT_MODEL.to_string());
        let proxy = read_env(&[ENV_LLM_PROXY]).map(|(_, v)| v);

        let source = if env_key.is_some() {
            Source::Env
        } else if file.is_some() {
            Source::File(path)
        } else {
            Source::Default
        };

        Ok(Self {
            api_key,
            base_url,
            model,
            proxy,
            source,
        })
    }

    /// 端点风格由地址决定：带 `/anthropic` 的走 messages，其余一律当 OpenAI 兼容。
    pub fn protocol(&self) -> Protocol {
        if self.base_url.contains("/anthropic") {
            Protocol::Anthropic
        } else {
            Protocol::OpenAi
        }
    }

    /// 拼出完整请求地址。基址带不带 `/v1` 都认。
    pub fn endpoint(&self) -> String {
        let base = self.base_url.trim_end_matches('/');
        let has_v1 = base.ends_with("/v1");
        match (self.protocol(), has_v1) {
            (Protocol::Anthropic, true) => format!("{base}/messages"),
            (Protocol::Anthropic, false) => format!("{base}/v1/messages"),
            (Protocol::OpenAi, true) => format!("{base}/chat/completions"),
            (Protocol::OpenAi, false) => format!("{base}/v1/chat/completions"),
        }
    }

    /// 打日志用的脱敏 key：留头 6 位和尾 4 位，够认出是哪一把，不够拿来用。
    pub fn masked_key(&self) -> String {
        let n = self.api_key.chars().count();
        if n <= 12 {
            return "***".to_string();
        }
        let head: String = self.api_key.chars().take(6).collect();
        let tail: String = self.api_key.chars().skip(n - 4).collect();
        format!("{head}…{tail}")
    }
}

/// 配置文件路径：`ASSAY_CONFIG` / 旧名 → 默认 Hermes profile。
pub fn config_path() -> String {
    match read_env(&[ENV_CONFIG, LEGACY_CONFIG]) {
        Some((name, v)) => {
            if name == LEGACY_CONFIG {
                warn_legacy(name, ENV_CONFIG);
            }
            v
        }
        None => format!("{}/.hermes/profiles/ai/config.yaml", home()),
    }
}

/// 从配置文件里抠出 key / 端点 / 模型。
///
/// 不用 YAML 库：只需要读一个 `model:` 块，为此引入 serde_yaml 并在
/// RUSTSEC 上多养一个依赖不值得。读不到就返回 None，由调用方兜底。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct YamlLlm {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
}

pub fn read_llm_from_yaml(path: &str) -> Option<YamlLlm> {
    let text = fs::read_to_string(path).ok()?;
    let lines: Vec<&str> = text.lines().collect();

    // key：全文第一个 api_key。本机配置里有 21 个，第一个就是主模型。
    let key_idx = lines
        .iter()
        .position(|l| l.trim_start().starts_with("api_key:"))?;
    let api_key = field_value(lines[key_idx], "api_key")?;

    // 模型块：从 `model:` 表头往下读到缩进回退，块里的 default / base_url 优先。
    let mut model = None;
    let mut base_url = None;
    if let Some(h) = lines
        .iter()
        .position(|l| l.trim() == "model:" && !l.trim_start().starts_with('-'))
    {
        let head_indent = indent_of(lines[h]);
        for l in &lines[h + 1..] {
            if l.trim().is_empty() || l.trim_start().starts_with('#') {
                continue;
            }
            if indent_of(l) <= head_indent {
                break;
            }
            if model.is_none() {
                model = field_value(l, "default").or_else(|| field_value(l, "model"));
            }
            if base_url.is_none() {
                base_url = field_value(l, "base_url");
            }
        }
    }

    // 兜底：key 上下几行里找，兼容「扁平写」的配置。
    if base_url.is_none() {
        let start = key_idx.saturating_sub(6);
        let end = (key_idx + 7).min(lines.len());
        base_url = lines[start..end]
            .iter()
            .find_map(|l| field_value(l, "base_url"));
    }
    if model.is_none() {
        let start = key_idx.saturating_sub(6);
        let end = (key_idx + 7).min(lines.len());
        model = lines[start..end]
            .iter()
            .find_map(|l| field_value(l, "model"));
    }

    Some(YamlLlm {
        api_key: Some(api_key),
        base_url,
        model,
    })
}

/// 取 `key: value` 的 value，去掉引号和空值。
fn field_value(line: &str, key: &str) -> Option<String> {
    let t = line.trim().trim_start_matches("- ").trim();
    let rest = t.strip_prefix(key)?.strip_prefix(':')?;
    // `model:` 后面紧跟换行时 rest 为空，会在下面被滤掉
    if !rest.starts_with(' ') && !rest.is_empty() {
        return None;
    }
    let v = rest.trim().trim_matches('"').trim_matches('\'');
    if v.is_empty() {
        None
    } else {
        Some(v.to_string())
    }
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::Mutex;

    /// 环境变量是进程级共享状态，改它的用例必须串行。
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn write_tmp(name: &str, body: &str) -> String {
        let path = std::env::temp_dir().join(name);
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(body.as_bytes()).unwrap();
        path.to_str().unwrap().to_string()
    }

    fn cfg(base: &str) -> LlmConfig {
        LlmConfig {
            api_key: "sk-abcdefghijkl".into(),
            base_url: base.into(),
            model: "m".into(),
            proxy: None,
            source: Source::Default,
        }
    }

    #[test]
    fn reads_key_next_to_endpoint() {
        let p = write_tmp(
            "assay-cfg-flat.yaml",
            "providers:\n  deepseek:\n    base_url: https://api.deepseek.com/anthropic\n    api_key: sk-test-123\n",
        );
        let y = read_llm_from_yaml(&p).unwrap();
        assert_eq!(y.api_key.as_deref(), Some("sk-test-123"));
        assert_eq!(
            y.base_url.as_deref(),
            Some("https://api.deepseek.com/anthropic")
        );
    }

    /// 本机 2026-11 的配置长这样：`model:` 块里是 `default:` 而不是 `model:`。
    /// 老实现靠搜 `api.deepseek.com` 找 key，换渠道后直接读不到。
    #[test]
    fn reads_hermes_model_block() {
        let p = write_tmp(
            "assay-cfg-block.yaml",
            "model:\n  default: deepseek-v4.1-flash\n  provider: custom\n  base_url: https://platform.yeta.ai/v1\n  api_key: sk-live-xyz\nfallback_providers:\n  - model: other\n    api_key: sk-second\n",
        );
        let y = read_llm_from_yaml(&p).unwrap();
        assert_eq!(y.api_key.as_deref(), Some("sk-live-xyz"));
        assert_eq!(y.base_url.as_deref(), Some("https://platform.yeta.ai/v1"));
        assert_eq!(y.model.as_deref(), Some("deepseek-v4.1-flash"));
    }

    /// 块里没有 `default:` 时，`model:` 也算。
    #[test]
    fn model_key_inside_block() {
        let p = write_tmp(
            "assay-cfg-modelkey.yaml",
            "model:\n  model: glm-5.3-flash\n  base_url: https://x/v1\n  api_key: sk-1\n",
        );
        let y = read_llm_from_yaml(&p).unwrap();
        assert_eq!(y.model.as_deref(), Some("glm-5.3-flash"));
    }

    #[test]
    fn missing_file_is_none() {
        assert!(read_llm_from_yaml("/nonexistent/config.yaml").is_none());
    }

    #[test]
    fn no_key_is_none() {
        let p = write_tmp("assay-cfg-nokey.yaml", "model:\n  default: x\n");
        assert!(read_llm_from_yaml(&p).is_none());
    }

    #[test]
    fn endpoint_accepts_base_with_or_without_v1() {
        assert_eq!(
            cfg("https://platform.yeta.ai/v1").endpoint(),
            "https://platform.yeta.ai/v1/chat/completions"
        );
        assert_eq!(
            cfg("https://platform.yeta.ai").endpoint(),
            "https://platform.yeta.ai/v1/chat/completions"
        );
        // 尾斜杠不能拼出 `//`
        assert_eq!(
            cfg("https://platform.yeta.ai/v1/").endpoint(),
            "https://platform.yeta.ai/v1/chat/completions"
        );
    }

    #[test]
    fn anthropic_base_uses_messages() {
        let c = cfg("https://api.deepseek.com/anthropic");
        assert_eq!(c.protocol(), Protocol::Anthropic);
        assert_eq!(
            c.endpoint(),
            "https://api.deepseek.com/anthropic/v1/messages"
        );
    }

    #[test]
    fn masked_key_hides_middle() {
        assert_eq!(cfg("x").masked_key(), "sk-abc…ijkl");
        let short = LlmConfig {
            api_key: "short".into(),
            ..cfg("x")
        };
        assert_eq!(short.masked_key(), "***");
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
    fn env_overrides_file_and_legacy_still_readable() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var(ENV_LLM_KEY, "new-key");
        std::env::set_var(LEGACY_LLM_KEY, "old-key");
        std::env::set_var(ENV_LLM_MODEL, "only-model");
        let c = LlmConfig::resolve().unwrap();
        assert_eq!(c.api_key, "new-key");
        assert_eq!(c.model, "only-model");
        assert_eq!(c.source, Source::Env);
        std::env::remove_var(ENV_LLM_KEY);
        std::env::remove_var(LEGACY_LLM_KEY);
        std::env::remove_var(ENV_LLM_MODEL);
    }
}
