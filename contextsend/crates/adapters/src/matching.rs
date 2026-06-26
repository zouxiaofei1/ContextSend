//! 上下文片段匹配：把用户复制 / 拖入的一段文本，匹配回某个本地应用里的完整会话。
//!
//! 用途：导出方向的「自动匹配正确的页面」。用户只需抓一小段上下文，
//! ContextSend 在各可读适配器的会话里定位出处，再把**整条完整会话**纳入存储库。
//!
//! 策略（从严到宽）：
//! 1. **长度门槛**：归一化后过短的片段直接拒绝（[`MIN_SNIPPET_CHARS`]），避免误匹配。
//! 2. **精确子串**：归一化后片段是某会话全文的子串 → 命中，得分 1.0。
//! 3. **字符 n-gram 模糊**：按 [`NGRAM_K`] 字符窗口比对，综合「整体重合比例」与
//!    「最长连续命中段长度」取分（见 [`match_score`]），超过 [`FUZZY_THRESHOLD`]
//!    取最高分者。后者让夹带公式渲染噪声的长片段，凭一段足够长的吻合文字即可命中。

use std::collections::HashSet;

use cs_core::Conversation;

use crate::{adapter_by_name, builtin_adapter_names, AdapterError};

/// 可靠匹配所需的最小片段长度（归一化后的字符数）。短于此值拒绝匹配。
pub const MIN_SNIPPET_CHARS: usize = 8;
/// 模糊匹配的字符 n-gram 窗口大小。
pub const NGRAM_K: usize = 10;
/// 模糊匹配判定为「命中」的最低重合比例。
pub const FUZZY_THRESHOLD: f32 = 0.55;
/// 「最长连续命中」判定为强匹配所需覆盖的字符数（归一化基准）。
/// 片段里存在一段 ≥ 此长度、与会话原文几乎逐字一致的文字，即足以命中，
/// 不被公式渲染等噪声稀释整体比例。
pub const ABS_MATCH_CHARS: usize = 40;

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

/// 片段与目标文本的匹配得分（`0..1`）。
///
/// 综合两个角度取较大者：
/// - **整体重合比例**：片段所有 `k` 字符窗口中、落在目标窗口集合里的比例。对短而
///   干净的片段有效，但会被长片段里的噪声稀释。
/// - **最长连续命中**：片段里与目标几乎逐字一致的最长一段所覆盖的字符数，归一化到
///   [`ABS_MATCH_CHARS`]。这让夹带大量噪声（如 KaTeX 渲染出的散字）的长片段，只要
///   含一段足够长的吻合文字即可命中，不被噪声拉低。
fn match_score(snippet_chars: &[char], text: &str) -> f32 {
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
    let mut hits = 0usize;
    let mut run = 0usize; // 当前连续命中的 k-gram 数
    let mut best_run = 0usize; // 最长连续命中
    for w in &snippet_grams {
        if text_grams.contains(w) {
            hits += 1;
            run += 1;
            best_run = best_run.max(run);
        } else {
            run = 0;
        }
    }
    let ratio = hits as f32 / snippet_grams.len() as f32;
    // best_run 个连续 k-gram 约覆盖 best_run + k - 1 个字符。
    let run_chars = if best_run > 0 { best_run + k - 1 } else { 0 };
    let run_score = (run_chars as f32 / ABS_MATCH_CHARS as f32).min(1.0);
    ratio.max(run_score)
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

        // 否则按综合得分做模糊匹配（比例 vs 最长连续命中），留最高分。
        let score = match_score(snippet_chars, &text);
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
    log::debug!("片段匹配开始: 归一化后 {} 字符", snippet_chars.len());

    let mut best: Option<ConversationMatch> = None;

    // 同步适配器：逐个 list_conversations。ChatBox 的同步实现返回 NotImplemented，
    // 会被跳过；其异步读取在下方单独处理。
    for name in builtin_adapter_names() {
        let Ok(adapter) = adapter_by_name(name) else {
            continue;
        };
        let Ok(convos) = adapter.list_conversations() else {
            log::debug!("适配器 {name} 不可读，跳过");
            continue; // 未安装 / 不可读 / 未实现的适配器跳过
        };
        log::debug!("在 {name} 的 {} 个会话中匹配", convos.len());
        if let Some(hit) = match_in_app(name, convos, &norm_snippet, &snippet_chars, &mut best) {
            log::debug!("片段在 {name} 精确命中");
            return Ok(Some(hit));
        }
    }

    // ChatBox：异步 CDP 读取。ChatBox 未带调试端口运行时返回 Err，直接跳过。
    if let Ok(convos) = crate::list_chatbox_conversations().await {
        log::debug!("在 ChatBox 的 {} 个会话中匹配", convos.len());
        if let Some(hit) = match_in_app("ChatBox", convos, &norm_snippet, &snippet_chars, &mut best)
        {
            log::debug!("片段在 ChatBox 精确命中");
            return Ok(Some(hit));
        }
    }

    match &best {
        Some(m) => log::debug!("片段模糊命中: app={} score={:.3}", m.app, m.score),
        None => log::debug!("片段未命中任何会话"),
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
        let score = match_score(&snippet_chars, &normalize(original));
        assert!(score > FUZZY_THRESHOLD, "score={score}");
    }

    #[test]
    fn ngram_overlap_low_for_unrelated() {
        let snippet_chars: Vec<char> = normalize("完全不相关的一段文字内容这里随便写写")
            .chars()
            .collect();
        let score = match_score(&snippet_chars, &normalize("另一篇讲述天气与温度的对话记录"));
        assert!(score < FUZZY_THRESHOLD, "score={score}");
    }

    #[test]
    fn dirty_snippet_matches_via_longest_run() {
        // 模拟从 ChatBox 拖拽 / 复制得到的脏片段：公式区被渲染成零散字符，
        // 但中间夹着一段足够长、与原会话逐字一致的中文叙述。
        let clean = "当方程的判别式大于零时该一元二次方程在实数范围内有两个不相等的实数解";
        let convo_text = normalize(&format!("数学问答 {clean} 你说得对"));
        let dirty = format!("a ≠ 0 N N 2 + b x + c = 0 {clean} √ π ∑ 1 / n ^ 2");
        let snippet_chars: Vec<char> = normalize(&dirty).chars().collect();
        let score = match_score(&snippet_chars, &convo_text);
        assert!(score >= FUZZY_THRESHOLD, "score={score}");
    }

    #[tokio::test]
    async fn too_short_snippet_is_rejected() {
        assert!(matches!(
            match_snippet("太短了").await,
            Err(AdapterError::SnippetTooShort(_))
        ));
    }
}
