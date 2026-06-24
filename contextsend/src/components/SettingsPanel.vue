<script setup lang="ts">
import { ref } from 'vue'
import { useAppStore } from '../stores/app'
import { useSettingsStore, ACCENT_COLORS } from '../stores/settings'
import { SUPPORTED_LOCALES } from '../i18n'
import type { Locale } from '../i18n'
import { useI18n } from 'vue-i18n'

const app = useAppStore()
const settings = useSettingsStore()
const { t, locale } = useI18n()

const renameText = ref('')

async function onRename(): Promise<void> {
  const name = renameText.value.trim()
  if (!name) return
  await app.renameSelf(name)
  renameText.value = ''
  app.status = t('common.renameSuccess')
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
          <button
            v-for="color in ACCENT_COLORS"
            :key="color.hex"
            class="color-swatch"
            :class="{ active: settings.accentColor === color.hex }"
            :style="{ background: color.hex }"
            :title="color.name"
            @click="onColorSelect(color.hex)"
          />
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
            :placeholder="t('settings.renamePlaceholder')"
            class="rename-input"
          />
          <button class="small" @click="onRename">
            {{ t('settings.renameButton') }}
          </button>
        </div>
      </div>
    </section>

    <!-- 关于 -->
    <section v-if="app.info" class="settings-section">
      <h2>{{ t('settings.about') }}</h2>

      <div class="setting-row">
        <div class="setting-row__label muted">{{ t('settings.version') }}</div>
        <div class="setting-row__control">{{ app.info.version }}</div>
      </div>

      <div class="setting-row">
        <div class="setting-row__label muted">{{ t('settings.platform') }}</div>
        <div class="setting-row__control">{{ app.info.platform }}</div>
      </div>

      <div class="setting-row">
        <div class="setting-row__label muted">{{ t('settings.adapters') }}</div>
        <div class="setting-row__control">{{ app.info.adapters.join('、') }}</div>
      </div>
    </section>

    <!-- 本机身份 -->
    <section v-if="app.identity" class="settings-section">
      <h2>{{ t('common.myDevice') }}</h2>
      <div class="setting-row">
        <div class="setting-row__label muted">{{ t('common.name') }}</div>
        <div class="setting-row__control">
          <strong>{{ app.identity.name }}</strong>
          <span class="muted" style="margin-left: 0.5rem">
            ({{ app.identity.uuid.slice(0, 8) }})
          </span>
        </div>
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

.rename-control {
  display: flex;
  gap: 0.5rem;
}

.rename-input {
  width: 160px;
}

/* 颜色色块容器 */
.setting-row__control .color-swatch + .color-swatch {
  margin-left: 0;
}
</style>
