// 代码块语言图标解析：把 highlight.js 的语言标识映射到「友好显示名 + 图标」。
//
// - 图标取自 src/assets/lang-icons/*.svg（经 Vite 打包，带 hash）。
// - 未收录图标的语言（如 JavaScript / Rust / Java 等）只给友好名，图标走
//   fallbackIconSvg 生成的「圆角矩形 + 右下角缩写」占位，保证一致观感。

// 文件名（去扩展名）→ 打包后 URL。
const modules = import.meta.glob('../assets/lang-icons/*.svg', {
  eager: true,
  query: '?url',
  import: 'default',
})
const ICONS: Record<string, string> = {}
for (const [path, url] of Object.entries(modules)) {
  const stem = path.split('/').pop()!.replace(/\.svg$/, '')
  ICONS[stem] = url as string
}

interface LangEntry {
  /** 友好显示名。 */
  name: string
  /** 图标文件名（不含扩展名）；缺省则走 fallback 生成器。 */
  icon?: string
}

// key：highlight.js 语言标识 / 常见别名（全小写）。
const LANG_TABLE: Record<string, LangEntry> = {
  // —— 有专属图标 ——
  python: { name: 'Python', icon: 'Python' },
  py: { name: 'Python', icon: 'Python' },
  c: { name: 'C', icon: 'C' },
  'c++': { name: 'C++', icon: 'Cpp' },
  cpp: { name: 'C++', icon: 'Cpp' },
  cc: { name: 'C++', icon: 'Cpp' },
  cxx: { name: 'C++', icon: 'Cpp' },
  'c#': { name: 'C#', icon: 'CSharp' },
  csharp: { name: 'C#', icon: 'CSharp' },
  cs: { name: 'C#', icon: 'CSharp' },
  typescript: { name: 'TypeScript', icon: 'TypeScript' },
  ts: { name: 'TypeScript', icon: 'TypeScript' },
  tsx: { name: 'TSX', icon: 'TSX' },
  jsx: { name: 'JSX', icon: 'JSX' },
  vue: { name: 'Vue', icon: 'Vue' },
  lua: { name: 'Lua', icon: 'Lua' },
  php: { name: 'PHP', icon: 'PHP' },
  r: { name: 'R', icon: 'R' },
  racket: { name: 'Racket', icon: 'Racket' },
  fortran: { name: 'Fortran', icon: 'Fortran' },
  pascal: { name: 'Pascal', icon: 'Pascal' },
  delphi: { name: 'Delphi', icon: 'Pascal' },
  zig: { name: 'Zig', icon: 'Zig' },
  powershell: { name: 'PowerShell', icon: 'PowerShell' },
  pwsh: { name: 'PowerShell', icon: 'PowerShell' },
  ps: { name: 'PowerShell', icon: 'PowerShell' },
  ps1: { name: 'PowerShell', icon: 'PowerShell' },
  yaml: { name: 'YAML', icon: 'YAML' },
  yml: { name: 'YAML', icon: 'YAML' },
  toml: { name: 'TOML', icon: 'TOML' },
  latex: { name: 'LaTeX', icon: 'LaTeX' },
  tex: { name: 'TeX', icon: 'LaTeX' },
  sql: { name: 'SQL', icon: 'SQLite' },
  sqlite: { name: 'SQLite', icon: 'SQLite' },
  tailwind: { name: 'Tailwind', icon: 'Tailwind' },
  vite: { name: 'Vite', icon: 'Vite' },
  bash: { name: 'Bash', icon: 'Terminal' },
  sh: { name: 'Shell', icon: 'Terminal' },
  shell: { name: 'Shell', icon: 'Terminal' },
  shellsession: { name: 'Shell', icon: 'Terminal' },
  zsh: { name: 'Zsh', icon: 'Terminal' },
  console: { name: 'Console', icon: 'Terminal' },
  bat: { name: 'Batch', icon: 'Terminal' },
  cmd: { name: 'Batch', icon: 'Terminal' },
  dosbatch: { name: 'Batch', icon: 'Terminal' },

  // —— 无专属图标，仅友好名（走 fallback 生成器） ——
  javascript: { name: 'JavaScript' },
  js: { name: 'JavaScript' },
  rust: { name: 'Rust' },
  rs: { name: 'Rust' },
  go: { name: 'Go' },
  golang: { name: 'Go' },
  java: { name: 'Java' },
  kotlin: { name: 'Kotlin' },
  kt: { name: 'Kotlin' },
  swift: { name: 'Swift' },
  ruby: { name: 'Ruby' },
  rb: { name: 'Ruby' },
  dart: { name: 'Dart' },
  scala: { name: 'Scala' },
  perl: { name: 'Perl' },
  haskell: { name: 'Haskell' },
  elixir: { name: 'Elixir' },
  erlang: { name: 'Erlang' },
  clojure: { name: 'Clojure' },
  groovy: { name: 'Groovy' },
  julia: { name: 'Julia' },
  matlab: { name: 'MATLAB' },
  solidity: { name: 'Solidity' },
  objectivec: { name: 'Objective-C' },
  html: { name: 'HTML' },
  xml: { name: 'XML' },
  svg: { name: 'SVG' },
  css: { name: 'CSS' },
  scss: { name: 'SCSS' },
  sass: { name: 'Sass' },
  less: { name: 'Less' },
  json: { name: 'JSON' },
  json5: { name: 'JSON5' },
  markdown: { name: 'Markdown' },
  md: { name: 'Markdown' },
  dockerfile: { name: 'Dockerfile' },
  docker: { name: 'Dockerfile' },
  makefile: { name: 'Makefile' },
  nginx: { name: 'nginx' },
  graphql: { name: 'GraphQL' },
  diff: { name: 'Diff' },
  ini: { name: 'INI' },
  text: { name: 'Text' },
  plaintext: { name: 'Text' },
}

/** 稳定地把字符串散列为色相（0..359），用于 fallback 占位的底色。 */
function hashHue(s: string): number {
  let h = 0
  for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) >>> 0
  return h % 360
}

/** 取显示名的字母数字部分，最多 4 个字符的大写缩写。 */
function abbrev(name: string): string {
  const letters = name.replace(/[^a-zA-Z0-9]/g, '')
  return (letters || name).slice(0, 4).toUpperCase()
}

export interface ResolvedLang {
  /** 友好显示名。 */
  name: string
  /** 命中的图标 URL；未命中为 null（此时用 fallbackIconSvg）。 */
  iconUrl: string | null
  /** fallback 占位用的缩写（最多 4 个大写字符）。 */
  abbr: string
  /** fallback 占位底色的色相。 */
  hue: number
}

/** 把 highlight.js 语言标识解析为显示名 + 图标信息。 */
export function resolveLang(lang: string): ResolvedLang {
  const key = (lang || '').trim().toLowerCase()
  const entry = LANG_TABLE[key]
  const name = entry?.name ?? (key ? key[0].toUpperCase() + key.slice(1) : 'Text')
  const iconUrl = entry?.icon ? (ICONS[entry.icon] ?? null) : null
  return { name, iconUrl, abbr: abbrev(name), hue: hashHue(name) }
}

/**
 * 生成缺省语言图标：圆角矩形底 + 右下角缩写。
 * 返回内联 SVG 字符串（内容自生成、无外部输入，可安全 innerHTML）。
 */
export function fallbackIconSvg(abbr: string, hue: number): string {
  const fontSize = abbr.length >= 4 ? 7 : abbr.length === 3 ? 8 : 9
  return (
    '<svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg" width="100%" height="100%">' +
    `<rect x="1" y="1" width="22" height="22" rx="5" fill="hsl(${hue},52%,45%)"/>` +
    `<text x="21.5" y="21" text-anchor="end" font-family="ui-monospace,monospace"` +
    ` font-weight="700" font-size="${fontSize}" fill="#fff">${abbr}</text>` +
    '</svg>'
  )
}
