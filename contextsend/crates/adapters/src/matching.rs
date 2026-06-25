//! 上下文片段匹配：把用户复制 / 拖入的一段文本，匹配回某个本地应用里的完整会话。
//!
//! 用途：导出方向的「自动匹配正确的页面」。用户只需抓一小段上下文，
//! ContextSend 在各可读适配器的会话里定位出处，再把**整条完整会话**纳入存储库。
//!
//! 策略（从严到宽）：
//! 1. **长度门槛**：归一化后过短的片段直接拒绝（[`MIN_SNIPPET_CHARS`]），避免误匹配。
//! 2. **精确子串**：归一化后片段是某会话全文的子串 → 命中，得分 1.0。
//! 3. **字符 n-gram 模糊**：按 [`NGRAM_K`] 字符窗口求重合比例，容忍复制时的轻微改动；
//!    超过 [`FUZZY_THRESHOLD`] 取最高分者。

use std::collections::HashSet;

use cs_core::Conversation;

use crate::{adapter_by_name, builtin_adapter_names, AdapterError};

/// 可靠匹配所需的最小片段长度（归一化后的字符数）。短于此值拒绝匹配。
pub const MIN_SNIPPET_CHARS: usize = 8;
/// 模糊匹配的字符 n-gram 窗口大小。
pub const NGRAM_K: usize = 10;
/// 模糊匹配判定为「命中」的最低重合比例。
pub const FUZZY_THRESHOLD: f32 = 0.55;

/// 一次成功的匹配结果。
pub struct ConversationMatch {
    /// 命中会话所属的应用名（如 `Jan`）。
    pub app: String,
    /// 匹配得分：精确子串为 `1.0`，模糊匹配为重合比例（`0..1`）。
    pub score: f32,
    /// 命中的完整会话。
    pub conversation: Conversation,
}

/// 归一化文本：折叠所有空白为单个空格、首尾去空白、统一小写（利于英文匹配）。
fn normalize(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// 把一段会话拼成可供匹配的全文（标题 + 各条消息文本）。
fn conversation_fulltext(c: &Conversation) -> String {
    let mut parts = Vec::new();
    if let Some(t) = &c.title {
        parts.push(t.clone());
    }
    for m in &c.messages {
        parts.push(m.text());
    }
    parts.join("\n")
}

/// 片段与目标文本的字符 n-gram 重合比例（`0..1`）。
///
/// 取片段所有 `k` 字符窗口，统计其中出现在目标文本窗口集合里的比例。
/// 仅个别字符被改动时，只有跨改动点的少数窗口失配，整体比例仍高 → 容忍轻微编辑。
fn ngram_overlap(snippet_chars: &[char], text: &str) -> f32 {
    let k = NGRAM_K.min(snippet_chars.len());
    if k == 0 {
        return 0.0;
    }
    let text_chars: Vec<char> = text.chars().collect();
    if text_chars.len() < k {
        return 0.0;
    }
    let text_grams: HashSet<&[char]> = text_chars.windows(k).collect();
    let snippet_grams: Vec<&[char]> = snippet_chars.windows(k).collect();
    if snippet_grams.is_empty() {
        return 0.0;
    }
    let hits = snippet_grams
        .iter()
        .filter(|w| text_grams.contains(*w))
        .count();
    hits as f32 / snippet_grams.len() as f32
}

/// 在一组会话里匹配片段，按需更新 `best`。
///
/// 返回 `Some` 表示命中**精确子串**（得分 1.0，可立即作为最优结果返回）；
/// 返回 `None` 表示没有精确命中，但可能已更新了 `best` 里的模糊最高分。
fn match_in_app(
    app: &str,
    convos: Vec<Conversation>,
    norm_snippet: &str,
    snippet_chars: &[char],
    best: &mut Option<ConversationMatch>,
) -> Option<ConversationMatch> {
    for convo in convos {
        let text = normalize(&conversation_fulltext(&convo));

        // 精确子串命中即最优。
        if !norm_snippet.is_empty() && text.contains(norm_snippet) {
            return Some(ConversationMatch {
                app: app.to_string(),
                score: 1.0,
                conversation: convo,
            });
        }

        // 否则按 n-gram 重合比例做模糊匹配，留最高分。
        let score = ngram_overlap(snippet_chars, &text);
        if score >= FUZZY_THRESHOLD && best.as_ref().is_none_or(|b| score > b.score) {
            *best = Some(ConversationMatch {
                app: app.to_string(),
                score,
                conversation: convo,
            });
        }
    }
    None
}

/// 在所有可读适配器的会话里匹配片段。
///
/// - 片段过短返回 [`AdapterError::SnippetTooShort`]。
/// - 命中（精确或模糊达阈值）返回 `Ok(Some(..))`，取得分最高者。
/// - 无命中返回 `Ok(None)`。
///
/// 覆盖范围：同步适配器（如 Jan 读文件）+ ChatBox（异步 CDP 读取）。单个适配器
/// 读取失败（未安装 / 未运行 / 未实现）会被跳过，不影响其它适配器。
pub async fn match_snippet(snippet: &str) -> Result<Option<ConversationMatch>, AdapterError> {
    let norm_snippet = normalize(snippet);
    let snippet_chars: Vec<char> = norm_snippet.chars().collect();
    if snippet_chars.len() < MIN_SNIPPET_CHARS {
        return Err(AdapterError::SnippetTooShort(MIN_SNIPPET_CHARS));
    }

    let mut best: Option<ConversationMatch> = None;

    // 同步适配器：逐个 list_conversations。ChatBox 的同步实现返回 NotImplemented，
    // 会被跳过；其异步读取在下方单独处理。
    for name in builtin_adapter_names() {
        let Ok(adapter) = adapter_by_name(name) else {
            continue;
        };
        let Ok(convos) = adapter.list_conversations() else {
            continue; // 未安装 / 不可读 / 未实现的适配器跳过
        };
        if let Some(hit) = match_in_app(name, convos, &norm_snippet, &snippet_chars, &mut best) {
            return Ok(Some(hit));
        }
    }

    // ChatBox：异步 CDP 读取。ChatBox 未带调试端口运行时返回 Err，直接跳过。
    if let Ok(convos) = crate::list_chatbox_conversations().await {
        if let Some(hit) = match_in_app("ChatBox", convos, &norm_snippet, &snippet_chars, &mut best)
        {
            return Ok(Some(hit));
        }
    }

    Ok(best)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cs_core::ChatMessage;

    fn convo(title: &str, msgs: &[&str]) -> Conversation {
        let mut c = Conversation::new();
        c.title = Some(title.to_string());
        for (i, m) in msgs.iter().enumerate() {
            c.messages.push(if i % 2 == 0 {
                ChatMessage::user(*m)
            } else {
                ChatMessage::assistant(*m)
            });
        }
        c
    }

    #[test]
    fn normalize_collapses_whitespace_and_case() {
        assert_eq!(normalize("  Hello\n\tWorld  "), "hello world");
    }

    #[test]
    fn exact_substring_scores_full() {
        let c = convo("天气", &["今天北京天气怎么样", "今天北京晴，气温二十五度"]);
        let text = normalize(&conversation_fulltext(&c));
        assert!(text.contains(&normalize("今天北京晴，气温二十五度")));
    }

    #[test]
    fn ngram_overlap_high_for_minor_edit() {
        // 仅改一个字，重合比例应仍然很高（远超阈值）。
        let original = "这是一段足够长的上下文用来测试模糊匹配能力";
        let edited = "这是一段足够长的上下文用来测试模糊比对能力";
        let snippet_chars: Vec<char> = normalize(edited).chars().collect();
        let score = ngram_overlap(&snippet_chars, &normalize(original));
        assert!(score > FUZZY_THRESHOLD, "score={score}");
    }

    #[test]
    fn ngram_overlap_low_for_unrelated() {
        let snippet_chars: Vec<char> = normalize("完全不相关的一段文字内容这里随便写写")
            .chars()
            .collect();
        let score = ngram_overlap(&snippet_chars, &normalize("另一篇讲述天气与温度的对话记录"));
        assert!(score < FUZZY_THRESHOLD, "score={score}");
    }

    #[tokio::test]
    async fn too_short_snippet_is_rejected() {
        assert!(matches!(
            match_snippet("太短了").await,
            Err(AdapterError::SnippetTooShort(_))
        ));
    }
}
