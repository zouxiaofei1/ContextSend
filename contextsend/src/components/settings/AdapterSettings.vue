<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { ADAPTER_LOGOS } from '../../constants'
import type { AdapterInfo } from '../../stores/app'
import SettingsSection from './SettingsSection.vue'

defineProps<{ adapters: AdapterInfo[] }>()
const emit = defineEmits<{ open: [name: string] }>()

const { t } = useI18n()

/** 适配器 logo：按名字小写查表，未收录返回空（模板回退到首字母）。 */
function logoFor(name: string): string | undefined {
  return ADAPTER_LOGOS[name.toLowerCase()]
}
</script>

<template>
  <SettingsSection :title="t('settings.adapters.title')">
    <div class="adapter-grid">
      <button
        v-for="a in adapters"
        :key="a.name"
        class="adapter-card"
        :class="{ 'adapter-card--offline': !a.installed }"
        :title="a.installed ? a.name : t('settings.adapters.notInstalled')"
        @click="emit('open', a.name)"
      >
        <img
          v-if="logoFor(a.name)"
          class="adapter-card__logo"
          :src="logoFor(a.name)"
          :alt="a.name"
        />
        <span v-else class="adapter-card__logo adapter-card__logo--text">{{
          a.name.charAt(0)
        }}</span>
        <span class="adapter-card__name">{{ a.name }}</span>
        <span v-if="!a.installed" class="adapter-card__badge">{{
          t('settings.adapters.notInstalled')
        }}</span>
      </button>
    </div>
  </SettingsSection>
</template>

<style scoped>
.adapter-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(96px, 1fr));
  gap: 0.85rem;
}

.adapter-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.5rem;
  padding: 0.9rem 0.5rem;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: transparent;
  cursor: pointer;
  transition:
    background 0.12s ease,
    border-color 0.12s ease,
    transform 0.12s ease;
}

.adapter-card:hover {
  background: var(--bg-secondary);
  border-color: var(--accent);
}

.adapter-card:active {
  transform: scale(0.97);
}

.adapter-card__logo {
  width: 48px;
  height: 48px;
  border-radius: 12px;
  object-fit: contain;
}

.adapter-card__logo--text {
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 1.5rem;
  font-weight: 600;
  color: var(--text-secondary);
  background: var(--bg-secondary);
}

.adapter-card__name {
  font-size: 0.85rem;
  color: var(--text-primary);
}

/* 未探测到安装：logo 置灰、整体降透明度。 */
.adapter-card--offline .adapter-card__logo {
  filter: grayscale(1);
  opacity: 0.45;
}

.adapter-card--offline .adapter-card__name {
  color: var(--text-secondary);
}

.adapter-card__badge {
  font-size: 0.68rem;
  color: var(--text-secondary);
}
</style>
