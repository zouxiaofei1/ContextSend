/**
 * 清洗从剪贴板 / 拖拽得到的脏文本片段，使其能安全经 Tauri IPC 传输并参与匹配。
 *
 * 背景：从 Electron 应用（如 ChatBox）拖拽 / 复制富文本时，文本里常混入：
 * - **未配对的代理项（lone surrogate）**：JSON 序列化后变成孤立代理，后端
 *   serde_json 解析报 “unexpected end of hex escape” 导致整条匹配请求失败；
 * - **零宽字符 / 控制字符**：KaTeX、富文本渲染插入，污染 n-gram 匹配。
 *
 * 这里统一剔除上述噪声，保留制表 / 换行 / 回车等正常空白（后端 normalize 会再折叠）。
 */
export function sanitizeSnippet(raw: string): string {
  return (
    raw
      // 未跟随低代理的高代理
      .replace(/[\uD800-\uDBFF](?![\uDC00-\uDFFF])/g, '')
      // 未跟随高代理的低代理
      .replace(/(?<![\uD800-\uDBFF])[\uDC00-\uDFFF]/g, '')
      // 零宽字符与 BOM（U+200B..U+200D, U+FEFF）
      .replace(/[\u200B-\u200D\uFEFF]/g, '')
      // 控制字符（保留 U+0009 制表、U+000A 换行、U+000D 回车）
      .replace(/[\u0000-\u0008\u000B\u000C\u000E-\u001F]/g, '')
  )
}
