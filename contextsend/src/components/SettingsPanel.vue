<script setup lang="ts">
import { ref, watch } from 'vue'
import { useAppStore } from '../stores/app'
import { useToastStore } from '../stores/toast'
import { useSettingsStore, ACCENT_COLORS } from '../stores/settings'
import { SUPPORTED_LOCALES } from '../i18n'
import type { Locale } from '../i18n'
import { useI18n } from 'vue-i18n'
import { generateRandomName } from '../utils/nameGenerator'
import appIcon from '../assets/app-icon.png'

const app = useAppStore()
const toast = useToastStore()
const settings = useSettingsStore()
const { t, locale } = useI18n()

const GITHUB_URL = 'https://github.com/zouxiaofei1/ContextSend'

const renameText = ref(app.identity?.name ?? '')

// 当从后端加载完身份信息后，回填输入框
watch(
  () => app.identity?.name,
  (name) => {
    if (name) renameText.value = name
  },
)

async function onRenameBlur(): Promise<void> {
  const name = renameText.value.trim()
  if (!name || name === app.identity?.name) {
    renameText.value = app.identity?.name ?? ''
    return
  }
  void applyRename(name)
}

async function applyRename(name: string): Promise<void> {
  await app.renameSelf(name)
  toast.success(t('common.renameSuccess'))
}

function onRandomName(): void {
  renameText.value = generateRandomName()
}

function onThemeToggle(): void {
  settings.toggleTheme()
}

function onColorSelect(hex: string): void {
  settings.setAccentColor(hex)
}

function onLocaleChange(e: Event): void {
  const target = e.target as HTMLSelectElement
  const loc = target.value as Locale
  locale.value = loc
  settings.setLocale(loc)
}

function openGitHub(): void {
  window.open(GITHUB_URL, '_blank')
}

const localeOptions = SUPPORTED_LOCALES.map((l) => ({
  value: l,
  label: l === 'zh-CN' ? '简体中文' : 'English',
}))
</script>

<template>
  <div class="panel">
    <!-- 通用设置 -->
    <section class="settings-section">
      <h2>{{ t('settings.general') }}</h2>

      <!-- 主题：亮/暗 -->
      <div class="setting-row">
        <div class="setting-row__label">
          <span>{{ t('settings.theme.label') }}</span>
        </div>
        <div class="setting-row__control">
          <span class="muted setting-option" :class="{ active: settings.theme === 'dark' }">
            {{ t('settings.theme.dark') }}
          </span>
          <label class="toggle">
            <input
              type="checkbox"
              :checked="settings.theme === 'light'"
              @change="onThemeToggle()"
            />
            <span class="toggle__slider" />
          </label>
          <span class="muted setting-option" :class="{ active: settings.theme === 'light' }">
            {{ t('settings.theme.light') }}
          </span>
        </div>
      </div>

      <!-- 主题色选择 -->
      <div class="setting-row">
        <div class="setting-row__label">
          <span>{{ t('settings.accentColor') }}</span>
        </div>
        <div class="setting-row__control">
          <select
            class="accent-color-select"
            :value="settings.accentColor"
            @change="onColorSelect(($event.target as HTMLSelectElement).value)"
          >
            <option
              v-for="color in ACCENT_COLORS"
              :key="color.hex"
              :value="color.hex"
            >
              {{ color.name }}
            </option>
          </select>
        </div>
      </div>

      <!-- 语言切换 -->
      <div class="setting-row">
        <div class="setting-row__label">
          <span>{{ t('settings.language') }}</span>
        </div>
        <div class="setting-row__control">
          <select
            class="locale-select"
            :value="settings.locale"
            @change="onLocaleChange"
          >
            <option
              v-for="opt in localeOptions"
              :key="opt.value"
              :value="opt.value"
            >
              {{ opt.label }}
            </option>
          </select>
        </div>
      </div>

      <!-- 关闭时最小化到托盘 -->
      <div class="setting-row">
        <div class="setting-row__label">
          <span>{{ t('settings.minimizeToTray') }}</span>
        </div>
        <div class="setting-row__control">
          <label class="toggle">
            <input
              type="checkbox"
              :checked="settings.minimizeToTray"
              @change="settings.toggleMinimizeToTray()"
            />
            <span class="toggle__slider" />
          </label>
        </div>
      </div>

      <!-- 开机自启 -->
      <div class="setting-row">
        <div class="setting-row__label">
          <span>{{ t('settings.autoStart') }}</span>
        </div>
        <div class="setting-row__control">
          <label class="toggle">
            <input
              type="checkbox"
              :checked="settings.autoStart"
              @change="settings.toggleAutoStart()"
            />
            <span class="toggle__slider" />
          </label>
        </div>
      </div>

      <!-- 本机改名 -->
      <div class="setting-row">
        <div class="setting-row__label">
          <span>{{ t('settings.rename') }}</span>
        </div>
        <div class="setting-row__control rename-control">
          <input
            v-model="renameText"
            class="rename-input"
            @blur="onRenameBlur"
            @keyup.enter="($event.target as HTMLInputElement).blur()"
          />
          <button
            class="ghost small random-name-btn"
            :title="t('settings.randomName')"
            @click="onRandomName"
          >
            🎲
          </button>
        </div>
      </div>
    </section>

    <!-- 关于 -->
    <section v-if="app.info" class="about-section">
      <img
        :src="appIcon"
        alt="ContextSend"
        class="about-icon"
      />
      <h1 class="about-title">ContextSend</h1>
      <div class="about-meta">
        <a class="about-link" @click.prevent="openGitHub">GitHub</a>
        <span class="about-version">Ver {{ app.info.version }} ({{ app.info.platform }})</span>
        <span class="about-license">MIT</span>
      </div>
    </section>
  </div>
</template>

<style scoped>
.panel {
  flex: 1;
  padding: 1.5rem 2rem;
  overflow-y: auto;

  width: 100%;
  align-self: center;
}

.settings-section {
  margin-bottom: 2rem;
}

.settings-section h2 {
  margin: 0 0 1rem;
  font-size: 1.05rem;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.setting-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.7rem 0;
  border-bottom: 1px solid var(--border);
}

.setting-row:last-child {
  border-bottom: none;
}

.setting-row__label {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  font-size: 0.9rem;
}

.setting-row__control {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.9rem;
}

.setting-option {
  font-size: 0.8rem;
  transition: color 0.15s;
}

.setting-option.active {
  color: var(--text-primary);
  font-weight: 600;
}

.locale-select {
  width: auto;
  min-width: 120px;
}

.accent-color-select {
  width: auto;
  min-width: 160px;
}

.rename-control {
  display: flex;
  gap: 0.5rem;
}

.rename-input {
  width: 200px;
}

.random-name-btn {
  font-size: 1.1rem;
  line-height: 1;
  padding: 0.15rem 0.4rem;
}

/* ===== 关于区域 ===== */

.about-section {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.8rem;
  padding: 2rem 0 1rem;
  margin-top: 1.5rem;
  border-top: 1px solid var(--border);
}

.about-icon {
  width: 96px;
  height: 96px;
  border-radius: 22px;
  user-select: none;
  -webkit-user-drag: none;
}

.about-title {
  margin: 0;
  font-size: 1.6rem;
  font-weight: 700;
  color: var(--text-primary);
  letter-spacing: -0.01em;
}

.about-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  max-width: 360px;
  margin-top: 0.4rem;
  font-size: 0.8rem;
  color: var(--text-secondary);
}

.about-link {
  color: var(--accent);
  text-decoration: none;
  cursor: pointer;
}

.about-link:hover {
  text-decoration: underline;
}

.about-version {
  flex: 1;
  text-align: center;
}

.about-license {
  font-weight: 500;
}
</style>
