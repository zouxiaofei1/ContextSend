<script setup lang="ts">
import { ref } from 'vue'
import { useAppStore } from '../stores/app'
import { useI18n } from 'vue-i18n'
import { TAB_RECEIVE, TAB_DEVICES, TAB_SETTINGS } from '../constants'
import iconReceive from '../assets/icon-receive.svg?raw'
import iconDevices from '../assets/icon-devices.svg?raw'
import iconSettings from '../assets/icon-settings.svg?raw'
import appIcon from '../assets/app-icon.png'

const props = defineProps<{
  activeTab: string
  compact?: boolean
}>()

const emit = defineEmits<{
  select: [tab: string]
  toggle: []
}>()

const app = useAppStore()
const { t } = useI18n()

const navItems = [
  { id: TAB_DEVICES, icon: iconDevices, label: t('sidebar.devices') },
  { id: TAB_RECEIVE, icon: iconReceive, label: t('sidebar.receive') },
  { id: TAB_SETTINGS, icon: iconSettings, label: t('sidebar.settings') },
]

// 键盘上下键导航
const navRefs = ref<(HTMLElement | null)[]>([])
const focusedIndex = ref(navItems.findIndex((item) => item.id === props.activeTab))

function onNavFocus(index: number) {
  focusedIndex.value = index
}

function onNavKeydown(e: KeyboardEvent) {
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    const next = (focusedIndex.value + 1) % navItems.length
    focusedIndex.value = next
    navRefs.value[next]?.focus()
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    const prev = (focusedIndex.value - 1 + navItems.length) % navItems.length
    focusedIndex.value = prev
    navRefs.value[prev]?.focus()
  }
}
</script>

<template>
  <aside
    class="sidebar"
    :class="{ 'sidebar--compact': compact }"
    @dblclick="emit('toggle')"
  >
    <!-- Logo 区 -->
    <div class="sidebar__logo">
      <img class="sidebar__logo-icon" :src="appIcon" alt="" />
      <span
        class="sidebar__logo-text sidebar__text"
        :class="{ 'sidebar__text--hidden': compact }"
        >{{ t('app.title') }}</span
      >
    </div>

    <!-- 导航项 -->
    <nav class="sidebar__nav" @keydown="onNavKeydown">
      <button
        v-for="(item, index) in navItems"
        :key="item.id"
        :ref="
          (el) => {
            if (el) navRefs[index] = el as HTMLElement
          }
        "
        class="sidebar__nav-item"
        :class="{ 'sidebar__nav-item--active': activeTab === item.id }"
        @click="emit('select', item.id)"
        @dblclick.stop
        @focus="onNavFocus(index)"
      >
        <span class="sidebar__nav-icon" v-html="item.icon"></span>
        <span
          class="sidebar__nav-label sidebar__text"
          :class="{ 'sidebar__text--hidden': compact }"
          >{{ item.label }}</span
        >
      </button>
    </nav>

    <!-- 底部本机信息 -->
    <div class="sidebar__footer" v-if="app.identity">
      <span class="sidebar__me-badge">
        <span class="dot online" />
        <span class="sidebar__me-label sidebar__text" :class="{ 'sidebar__text--hidden': compact }">{{
          t('common.myDevice')
        }}</span>
      </span>
      <span class="sidebar__me-name sidebar__text" :class="{ 'sidebar__text--hidden': compact }">{{
        app.identity.name
      }}</span>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  width: 187px;
  min-width: 187px;
  height: 100%;
  background: var(--bg-secondary);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  user-select: none;
  -webkit-user-select: none;
  transition:
    width 0.12s ease,
    min-width 0.12s ease;
}

.sidebar__logo {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 1.25rem 1rem;
  border-bottom: 1px solid var(--border);
  transition:
    gap 0.12s ease,
    padding 0.12s ease;
}

.sidebar__logo-icon {
  width: 1.6rem;
  height: 1.6rem;
  object-fit: contain;
  flex-shrink: 0;
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
  transition: padding 0.12s ease;
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
  transition:
    background 0.12s,
    color 0.12s,
    gap 0.12s ease,
    padding 0.12s ease;
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

/* ---- 文字过渡 ---- */
.sidebar__text {
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
  transition:
    opacity 0.1s ease,
    max-width 0.12s ease;
}

.sidebar__text--hidden {
  opacity: 0;
  max-width: 0;
}

/* ---- 紧凑模式（仅图标） ---- */
.sidebar--compact {
  width: 56px;
  min-width: 56px;
}

.sidebar--compact .sidebar__logo {
  justify-content: center;
  padding: 1rem 0.5rem;
  gap: 0;
}

.sidebar--compact .sidebar__nav {
  padding: 0.5rem 0.25rem;
}

.sidebar--compact .sidebar__nav-item {
  justify-content: center;
  padding: 0.55rem 0;
  gap: 0;
}

.sidebar--compact .sidebar__footer {
  justify-content: center;
  padding: 0.75rem 0.25rem;
}

.sidebar--compact .sidebar__me-badge {
  padding: 0;
  gap: 0;
  border: none;
  background: transparent;
}

.sidebar__footer {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  border-top: 1px solid var(--border);
  font-size: 0.8rem;
  min-width: 0;
  transition:
    gap 0.12s ease,
    padding 0.12s ease;
}

.sidebar__me-badge {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.2rem 0.45rem;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg-tertiary);
  flex-shrink: 0;
  transition:
    gap 0.12s ease,
    padding 0.12s ease,
    border-color 0.12s ease,
    background 0.12s ease;
}

.sidebar__me-label {
  color: var(--text-secondary);
}

.sidebar__me-name {
  color: var(--text-secondary);
  min-width: 0;
}
</style>
