<script setup lang="ts">
import { computed, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useSettingsStore } from '../../stores/settings'
import { LANGUAGE_PREFERENCES, LOCALE_NATIVE_NAME } from '../../i18n'
import type { LangPreference } from '../../i18n'

const settings = useSettingsStore()
const { t } = useI18n()

const emit = defineEmits<{ back: [] }>()

function onKeydown(e: KeyboardEvent): void {
  if (e.key === 'Escape') {
    emit('back')
  }
}

onMounted(() => document.addEventListener('keydown', onKeydown))
onUnmounted(() => document.removeEventListener('keydown', onKeydown))

/** 语言列表：跟随系统显示本地化文案，其余显示母语名称。 */
const options = computed(() =>
  LANGUAGE_PREFERENCES.map((value) => ({
    value,
    label: value === 'system' ? t('settings.languageFollowSystem') : LOCALE_NATIVE_NAME[value],
  })),
)

function select(value: LangPreference): void {
  settings.setLangPreference(value)
}
</script>

<template>
  <div class="language-page">
    <header class="language-page__header">
      <button class="back-btn" :title="t('common.back')" @click="$emit('back')">
        <svg viewBox="0 0 24 24" width="22" height="22" aria-hidden="true">
          <path
            d="M15 6l-6 6 6 6"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      </button>
      <h2 class="language-page__title">{{ t('settings.language') }}</h2>
    </header>

    <ul class="language-list">
      <li
        v-for="opt in options"
        :key="opt.value"
        class="language-item"
        :class="{ 'language-item--active': settings.langPreference === opt.value }"
        role="button"
        tabindex="0"
        @click="select(opt.value)"
        @keyup.enter="select(opt.value)"
      >
        <span class="language-item__label">{{ opt.label }}</span>
        <svg
          v-if="settings.langPreference === opt.value"
          class="language-item__check"
          viewBox="0 0 24 24"
          width="20"
          height="20"
          aria-hidden="true"
        >
          <path
            d="M5 13l4 4L19 7"
            fill="none"
            stroke="currentColor"
            stroke-width="2.2"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.language-page {
  display: flex;
  flex-direction: column;
}

.language-page__header {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  margin-bottom: 1.25rem;
}

.back-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  padding: 0;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--text-primary);
  cursor: pointer;
  transition: background 0.12s ease;
}

.back-btn:hover {
  background: var(--bg-secondary);
}

.language-page__title {
  margin: 0;
  font-size: 1.25rem;
  font-weight: 600;
  color: var(--text-primary);
}

.language-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
}

.language-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.85rem 0.75rem;
  border-radius: 8px;
  cursor: pointer;
  color: var(--text-primary);
  transition: background 0.12s ease;
}

.language-item:hover {
  background: var(--bg-secondary);
}

.language-item--active .language-item__label {
  color: var(--accent);
  font-weight: 600;
}

.language-item__label {
  font-size: 1rem;
}

.language-item__check {
  color: var(--accent);
  flex-shrink: 0;
}
</style>
