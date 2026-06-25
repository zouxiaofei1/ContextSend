<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useI18n } from 'vue-i18n'
import appIcon from '../assets/app-icon.png'

const { t } = useI18n()
const appWindow = getCurrentWindow()

const isMaximized = ref(false)
let unlisten: (() => void) | undefined

async function syncMaximized(): Promise<void> {
  isMaximized.value = await appWindow.isMaximized()
}

onMounted(async () => {
  await syncMaximized()
  // 窗口尺寸变化（含双击拖拽区最大化/还原）后同步按钮图标
  unlisten = await appWindow.onResized(() => void syncMaximized())
})

onUnmounted(() => unlisten?.())

function minimize(): void {
  void appWindow.minimize()
}

function toggleMaximize(): void {
  void appWindow.toggleMaximize()
}

function close(): void {
  void appWindow.close()
}
</script>

<template>
  <div class="titlebar" data-tauri-drag-region>
    <!-- 左侧：图标 + 标题（整块可拖动） -->
    <div class="titlebar__brand" data-tauri-drag-region>
      <span class="titlebar__logo">📤</span>
      <span class="titlebar__title">{{ t('app.title') }}</span>
    </div>

    <!-- 中间弹性占位，扩大可拖动区域 -->
    <div class="titlebar__spacer" data-tauri-drag-region></div>

    <!-- 右侧：窗口控制按钮 -->
    <div class="titlebar__controls">
      <button class="titlebar__btn" :title="t('titlebar.minimize')" @click="minimize">
        <svg width="11" height="11" viewBox="0 0 11 11" aria-hidden="true">
          <rect x="1" y="5" width="9" height="1" fill="currentColor" />
        </svg>
      </button>

      <button class="titlebar__btn" :title="t('titlebar.maximize')" @click="toggleMaximize">
        <svg v-if="!isMaximized" width="11" height="11" viewBox="0 0 11 11" aria-hidden="true">
          <rect x="1.5" y="1.5" width="8" height="8" fill="none" stroke="currentColor" stroke-width="1" />
        </svg>
        <svg v-else width="11" height="11" viewBox="0 0 11 11" aria-hidden="true">
          <rect x="1.5" y="3" width="6" height="6" fill="none" stroke="currentColor" stroke-width="1" />
          <path d="M3.5 3V1.5H9.5V7.5H8" fill="none" stroke="currentColor" stroke-width="1" />
        </svg>
      </button>

      <button class="titlebar__btn titlebar__btn--close" :title="t('titlebar.close')" @click="close">
        <svg width="11" height="11" viewBox="0 0 11 11" aria-hidden="true">
          <path d="M1.5 1.5L9.5 9.5M9.5 1.5L1.5 9.5" stroke="currentColor" stroke-width="1.1" />
        </svg>
      </button>
    </div>
  </div>
</template>

<style scoped>
.titlebar {
  height: 36px;
  flex-shrink: 0;
  display: flex;
  align-items: stretch;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border);
  user-select: none;
  -webkit-user-select: none;
}

.titlebar__brand {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0 0.75rem;
}

.titlebar__logo {
  font-size: 1rem;
  line-height: 1;
}

.titlebar__title {
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--text-secondary);
  white-space: nowrap;
}

.titlebar__spacer {
  flex: 1;
}

.titlebar__controls {
  display: flex;
  align-items: stretch;
}

.titlebar__btn {
  width: 44px;
  height: 100%;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: none;
  border-radius: 0;
  padding: 0;
  color: var(--text-secondary);
  cursor: pointer;
  transition: background 0.12s, color 0.12s;
}

.titlebar__btn:hover {
  background: var(--bg-tertiary);
  color: var(--text-primary);
}

.titlebar__btn:active {
  opacity: 0.8;
}

.titlebar__btn--close:hover {
  background: var(--danger);
  color: #fff;
}
</style>
