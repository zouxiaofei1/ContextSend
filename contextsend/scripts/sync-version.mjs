/**
 * sync-version.mjs
 * 从 version.json 读取版本号，同步到 package.json、Cargo.toml、tauri.conf.json。
 */

import { readFileSync, writeFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, '..');

// 1. 读取版本号来源
const versionJsonPath = resolve(root, 'version.json');
const { version } = JSON.parse(readFileSync(versionJsonPath, 'utf-8'));

if (!version || typeof version !== 'string') {
  console.error('❌ version.json 中缺少有效的 version 字段');
  process.exit(1);
}

console.log(`📌 目标版本: ${version}`);

// 2. 同步 package.json
{
  const path = resolve(root, 'package.json');
  const pkg = JSON.parse(readFileSync(path, 'utf-8'));
  const old = pkg.version;
  pkg.version = version;
  writeFileSync(path, JSON.stringify(pkg, null, '  ') + '\n', 'utf-8');
  console.log(`✅ package.json: ${old} → ${version}`);
}

// 3. 同步 Cargo.toml（[workspace.package] 下的 version）
{
  const path = resolve(root, 'Cargo.toml');
  let content = readFileSync(path, 'utf-8');
  const re = /^version\s*=\s*"[^"]*"\s*$/m;
  let matched = false;

  // 只在 [workspace.package] 区块内替换
  const sections = content.split(/(?=^\[)/m);
  const updated = sections.map((section) => {
    if (/^\[workspace\.package\]/m.test(section)) {
      return section.replace(re, (match) => {
        matched = true;
        const oldVersion = match.match(/"([^"]*)"/)[1];
        console.log(`✅ Cargo.toml (workspace): ${oldVersion} → ${version}`);
        return `version = "${version}"`;
      });
    }
    return section;
  });

  if (matched) {
    writeFileSync(path, updated.join(''), 'utf-8');
  } else {
    console.warn('⚠️  未在 Cargo.toml [workspace.package] 中找到 version 字段');
  }
}

// 4. 同步 tauri.conf.json
{
  const path = resolve(root, 'src-tauri', 'tauri.conf.json');
  const conf = JSON.parse(readFileSync(path, 'utf-8'));
  const old = conf.version;
  conf.version = version;
  writeFileSync(path, JSON.stringify(conf, null, '  ') + '\n', 'utf-8');
  console.log(`✅ tauri.conf.json: ${old} → ${version}`);
}

console.log('🎉 版本同步完成！');
