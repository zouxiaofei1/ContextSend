<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useI18n } from 'vue-i18n'

defineProps<{
  /** 竖屏布局下无侧边栏，拖动区需让开整条顶部 */
  portrait?: boolean
}>()

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
  <!-- 悬浮控制条：左侧弹性区可拖动窗口，右侧为窗口控制按钮，整体靠右 -->
  <div class="titlebar" :class="{ 'titlebar--portrait': portrait }">
    <div class="titlebar__drag" data-tauri-drag-region></div>

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
  position: fixed;
  top: 0;
  /* 让开左侧 220px 侧边栏，使侧边栏完整顶到顶部、不被拖动区覆盖 */
  left: 220px;
  right: 0;
  height: 36px;
  z-index: 100;
  display: flex;
  align-items: stretch;
  pointer-events: none;
  user-select: none;
  -webkit-user-select: none;
}

/* 竖屏无侧边栏：拖动区让开整条顶部 */
.titlebar--portrait {
  left: 0;
}

/* 可拖动区域占据除按钮外的整条标题栏 */
.titlebar__drag {
  flex: 1;
  pointer-events: auto;
}

.titlebar__controls {
  display: flex;
  align-items: stretch;
  pointer-events: auto;
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
