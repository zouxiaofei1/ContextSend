<script setup lang="ts">
import { useAppStore } from '../stores/app'
import { useI18n } from 'vue-i18n'
import iconReceive from '../assets/icon-receive.svg?raw'
import iconDevices from '../assets/icon-devices.svg?raw'
import iconSettings from '../assets/icon-settings.svg?raw'

defineProps<{
  activeTab: string
}>()

const emit = defineEmits<{
  select: [tab: string]
}>()

const app = useAppStore()
const { t } = useI18n()

const navItems = [
  { id: 'receive', icon: iconReceive, label: t('sidebar.receive') },
  { id: 'devices', icon: iconDevices, label: t('sidebar.devices') },
  { id: 'settings', icon: iconSettings, label: t('sidebar.settings') },
]
</script>

<template>
  <aside class="sidebar">
    <!-- Logo 区 -->
    <div class="sidebar__logo">
      <span class="sidebar__logo-icon">📤</span>
      <span class="sidebar__logo-text">{{ t('app.title') }}</span>
    </div>

    <!-- 导航项 -->
    <nav class="sidebar__nav">
      <button
        v-for="item in navItems"
        :key="item.id"
        class="sidebar__nav-item"
        :class="{ 'sidebar__nav-item--active': activeTab === item.id }"
        @click="emit('select', item.id)"
      >
        <span class="sidebar__nav-icon" v-html="item.icon"></span>
        <span class="sidebar__nav-label">{{ item.label }}</span>
      </button>
    </nav>

    <!-- 底部本机信息 -->
    <div class="sidebar__footer" v-if="app.identity">
      <span class="dot online" />
      <span class="muted">{{ app.identity.name }}</span>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  width: 220px;
  min-width: 220px;
  height: 100vh;
  background: var(--bg-secondary);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  user-select: none;
  -webkit-user-select: none;
}

.sidebar__logo {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 1.25rem 1rem;
  border-bottom: 1px solid var(--border);
}

.sidebar__logo-icon {
  font-size: 1.5rem;
}

.sidebar__logo-text {
  font-size: 1.1rem;
  font-weight: 700;
  color: var(--text-primary);
}

.sidebar__nav {
  flex: 1;
  padding: 0.75rem 0.5rem;
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
}

.sidebar__nav-item {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  padding: 0.55rem 0.75rem;
  border-radius: 8px;
  background: transparent;
  color: var(--text-secondary);
  font-size: 0.9rem;
  border: none;
  cursor: pointer;
  transition: background 0.12s, color 0.12s;
  text-align: left;
  width: 100%;
}

.sidebar__nav-item:hover {
  background: var(--bg-tertiary);
  color: var(--text-primary);
}

.sidebar__nav-item--active {
  background: var(--accent) !important;
  color: #fff !important;
}

.sidebar__nav-icon {
  width: 1.5rem;
  height: 1.5rem;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.sidebar__nav-icon :deep(svg) {
  width: 1.25rem;
  height: 1.25rem;
  display: block;
}

.sidebar__nav-icon :deep(svg path) {
  fill: currentColor;
}

.sidebar__footer {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  border-top: 1px solid var(--border);
  font-size: 0.8rem;
}
</style>
