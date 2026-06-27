<script setup lang="ts">
import { useToastStore, type ToastType } from '../stores/toast'

const toast = useToastStore()

/** 各类型的图标字符（无额外依赖，配合类型色条已足够辨识）。 */
const ICONS: Record<ToastType, string> = {
  success: '✓',
  error: '✕',
  info: 'i',
  warning: '!',
}
</script>

<template>
  <TransitionGroup tag="div" name="toast" class="toast-host">
    <div
      v-for="t in toast.toasts"
      :key="t.id"
      class="toast"
      :class="`toast--${t.type}`"
      role="status"
      @click="toast.dismiss(t.id)"
    >
      <span class="toast__icon" :class="`toast__icon--${t.type}`">{{ ICONS[t.type] }}</span>
      <span class="toast__msg">{{ t.message }}</span>
      <button class="toast__close" @click.stop="toast.dismiss(t.id)" aria-label="close">✕</button>
    </div>
  </TransitionGroup>
</template>

<style scoped>
.toast-host {
  position: fixed;
  /* 避开 36px 自定义标题栏及其上方窗口控制按钮，落在「右端较上方」 */
  top: 44px;
  right: 16px;
  z-index: 1000;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  /* 容器不拦截点击，仅 toast 卡片本身可交互 */
  pointer-events: none;
  /* 固定宽度：防止 toast 离开时容器收缩导致文字重新换行 */
  width: min(360px, calc(100vw - 32px));
  max-width: min(360px, calc(100vw - 32px));
}

.toast {
  pointer-events: auto;
  display: flex;
  align-items: center;
  gap: 0.55rem;
  padding: 0.6rem 0.7rem 0.6rem 0.75rem;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  /* 左侧类型色条 */
  border-left: 3px solid var(--accent);
  border-radius: 8px;
  box-shadow: 0 6px 20px var(--shadow);
  font-size: 0.85rem;
  color: var(--text-primary);
  cursor: pointer;
}

.toast--success {
  border-left-color: var(--success);
}
.toast--error {
  border-left-color: var(--danger);
}
.toast--warning {
  border-left-color: var(--warning);
}
.toast--info {
  border-left-color: var(--accent);
}

.toast__icon {
  flex-shrink: 0;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 0.7rem;
  font-weight: 700;
  color: #fff;
  line-height: 1;
}
.toast__icon--success {
  background: var(--success);
}
.toast__icon--error {
  background: var(--danger);
}
.toast__icon--warning {
  background: var(--warning);
}
.toast__icon--info {
  background: var(--accent);
}

.toast__msg {
  flex: 1;
  min-width: 0;
  line-height: 1.35;
  word-break: break-word;
}

.toast__close {
  flex-shrink: 0;
  background: transparent;
  border: none;
  color: var(--text-secondary);
  font-size: 0.75rem;
  padding: 0.1rem 0.2rem;
  line-height: 1;
  border-radius: 4px;
  opacity: 0.6;
  transition:
    opacity 0.15s,
    color 0.15s;
}
.toast__close:hover {
  background: transparent;
  opacity: 1;
  color: var(--text-primary);
}

/* ===== 动画（150ms，进入滑入+淡入+轻微放大；离开淡出滑出+塌缩补位） ===== */
.toast-enter-active,
.toast-leave-active {
  transition: all 0.15s cubic-bezier(0.16, 1, 0.3, 1);
}
.toast-enter-from {
  opacity: 0;
  transform: translateX(16px) scale(0.96);
}
.toast-leave-to {
  opacity: 0;
  transform: translateX(8px) scale(0.98);
}
.toast-leave-active {
  position: absolute;
  left: 0;
  right: 0;
}
.toast-move {
  transition: transform 0.15s cubic-bezier(0.16, 1, 0.3, 1);
}
</style>
