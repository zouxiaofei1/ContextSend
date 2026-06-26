<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAppStore } from '../../stores/app'
import { useToastStore } from '../../stores/toast'
import { useSettingsStore } from '../../stores/settings'
import { THEMES } from '../../constants'
import { SUPPORTED_LOCALES } from '../../i18n'
import type { Locale } from '../../i18n'
import { generateRandomName } from '../../utils/nameGenerator'
import { validateName } from '../../utils/nameValidation'
import SettingsSection from './SettingsSection.vue'
import SettingRow from './SettingRow.vue'
import SettingToggle from './SettingToggle.vue'
import SettingSelect from './SettingSelect.vue'

const app = useAppStore()
const toast = useToastStore()
const settings = useSettingsStore()
const { t, locale } = useI18n()

const renameText = ref(app.identity?.name ?? '')

// 当从后端加载完身份信息后，回填输入框
watch(
  () => app.identity?.name,
  (name) => {
    if (name) renameText.value = name
  },
)

async function applyRename(name: string): Promise<void> {
  const trimmed = name.trim()
  if (!trimmed || trimmed === app.identity?.name) {
    renameText.value = app.identity?.name ?? ''
    return
  }
  const result = validateName(trimmed)
  if (!result.valid) {
    if (result.error === 'empty') {
      toast.error(t('settings.nameErrorEmpty'))
    } else if (result.error === 'tooLong') {
      toast.error(t('settings.nameErrorTooLong', { max: 32 }))
    }
    renameText.value = app.identity?.name ?? ''
    return
  }
  try {
    await app.renameSelf(trimmed)
    toast.success(t('common.renameSuccess'))
  } catch (e) {
    toast.error(t('settings.nameErrorBackend', { error: String(e) }))
    renameText.value = app.identity?.name ?? ''
  }
}

async function onRenameBlur(): Promise<void> {
  await applyRename(renameText.value)
}

function onRandomName(): void {
  const name = generateRandomName()
  renameText.value = name
  void applyRename(name)
}

function onLocaleChange(loc: Locale): void {
  locale.value = loc
  settings.setLocale(loc)
}

const themeOptions = computed(() =>
  THEMES.map((th) => ({
    value: th.id,
    label: th.nameEn,
  })),
)

const localeOptions = SUPPORTED_LOCALES.map((l) => ({
  value: l,
  label: l === 'zh-CN' ? '简体中文' : 'English',
}))
</script>

<template>
  <SettingsSection :title="t('settings.general')">
    <!-- 主题（命名主题；深浅跟随系统） -->
    <SettingRow :label="t('settings.theme.label')">
      <template #label>
        <span>{{ t('settings.theme.label') }}</span>
        <span class="muted theme-hint">{{ t('settings.theme.followSystem') }}</span>
      </template>
      <SettingSelect
        :model-value="settings.themeId"
        :options="themeOptions"
        min-width="160px"
        @update:model-value="settings.setThemeId($event)"
      />
    </SettingRow>

    <!-- 语言切换 -->
    <SettingRow :label="t('settings.language')">
      <SettingSelect
        :model-value="settings.locale"
        :options="localeOptions"
        @update:model-value="onLocaleChange($event)"
      />
    </SettingRow>

    <!-- 关闭时最小化到托盘 -->
    <SettingRow :label="t('settings.minimizeToTray')">
      <SettingToggle
        :model-value="settings.minimizeToTray"
        @update:model-value="settings.toggleMinimizeToTray()"
      />
    </SettingRow>

    <!-- 开机自启 -->
    <SettingRow :label="t('settings.autoStart')">
      <SettingToggle
        :model-value="settings.autoStart"
        @update:model-value="settings.toggleAutoStart()"
      />
    </SettingRow>

    <!-- 本机改名 -->
    <SettingRow :label="t('settings.rename')">
      <div class="rename-control">
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
    </SettingRow>

    <!-- 高级设置开关 -->
    <SettingRow :label="t('settings.advanced.label')">
      <SettingToggle
        :model-value="settings.showAdvanced"
        @update:model-value="settings.toggleShowAdvanced()"
      />
    </SettingRow>
  </SettingsSection>
</template>

<style scoped>
.theme-hint {
  font-size: 0.75rem;
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
</style>
