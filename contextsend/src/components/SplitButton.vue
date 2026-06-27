<script setup lang="ts">
import { ref, onBeforeUnmount } from 'vue'

/** 下拉项：value 回传给 `@select`，label 为展示文案。 */
interface SplitOption {
  value: string
  label: string
}

const props = defineProps<{
  /** 主按钮文案。点击主体等价于选中第一个 option（由父级处理）。 */
  label: string
  /** 下拉可选项。 */
  options: SplitOption[]
}>()

const emit = defineEmits<{
  /** 点击主按钮（无 value）或选中某下拉项（带 value）。 */
  (e: 'main'): void
  (e: 'select', value: string): void
}>()

const open = ref(false)

function toggle(): void {
  open.value = !open.value
}

function onMain(): void {
  open.value = false
  emit('main')
}

function onSelect(value: string): void {
  open.value = false
  emit('select', value)
}

/** 点击组件外部时收起菜单。 */
const root = ref<HTMLElement | null>(null)
function onDocClick(e: MouseEvent): void {
  if (open.value && root.value && !root.value.contains(e.target as Node)) {
    open.value = false
  }
}
document.addEventListener('click', onDocClick)
onBeforeUnmount(() => document.removeEventListener('click', onDocClick))
</script>

<template>
  <div ref="root" class="split">
    <div class="split-row">
      <button class="split-main" @click="onMain">{{ props.label }}</button>
      <button class="split-caret" :aria-expanded="open" @click="toggle">▾</button>
    </div>
    <ul v-if="open" class="split-menu">
      <li v-for="opt in props.options" :key="opt.value">
        <button class="split-item" @click="onSelect(opt.value)">{{ opt.label }}</button>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.split {
  position: relative;
  display: inline-block;
}

.split-row {
  display: flex;
}

.split-main {
  border-top-right-radius: 0;
  border-bottom-right-radius: 0;
}

.split-caret {
  border-top-left-radius: 0;
  border-bottom-left-radius: 0;
  padding: 0.45rem 0.55rem;
  border-left: 1px solid rgba(255, 255, 255, 0.25);
}

.split-menu {
  position: absolute;
  right: 0;
  bottom: calc(100% + 0.35rem);
  margin: 0;
  padding: 0.25rem;
  list-style: none;
  min-width: 100%;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  box-shadow: 0 6px 20px rgba(0, 0, 0, 0.25);
  z-index: 10;
}

.split-item {
  display: block;
  width: 100%;
  text-align: left;
  white-space: nowrap;
  background: transparent;
  color: var(--text-primary);
  border-radius: 5px;
}

.split-item:hover {
  background: var(--accent);
  color: #fff;
}
</style>
