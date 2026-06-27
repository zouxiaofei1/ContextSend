<script setup lang="ts">
/**
 * 可复用的快捷键录制输入框。
 */
import { computed, ref } from 'vue'

const props = defineProps<{
  /** 当前快捷键（Tauri accelerator 语法）；空串表示未设置。 */
  modelValue: string
  /** 未设置时的占位文案。 */
  placeholder?: string
  /** 录制态提示文案。 */
  recordingHint?: string
}>()
const emit = defineEmits<{ 'update:modelValue': [string] }>()

/** 是否处于录制态（监听键盘）。 */
const recording = ref(false)

/** 将 KeyboardEvent.code 映射为 Tauri 主键 token；不支持的键返回 null。 */
function mainKeyToken(code: string): string | null {
  let m: RegExpMatchArray | null
  if ((m = code.match(/^Key([A-Z])$/))) return m[1]
  if ((m = code.match(/^Digit([0-9])$/))) return m[1]
  if ((m = code.match(/^F([0-9]{1,2})$/))) return `F${m[1]}`
  switch (code) {
    case 'ArrowUp':
      return 'Up'
    case 'ArrowDown':
      return 'Down'
    case 'ArrowLeft':
      return 'Left'
    case 'ArrowRight':
      return 'Right'
    case 'Space':
      return 'Space'
    case 'Enter':
      return 'Enter'
    case 'Backquote':
      return '`'
    case 'Minus':
      return '-'
    case 'Equal':
      return '='
    default:
      return null
  }
}

/** 由事件构造 accelerator；要求至少一个修饰键 + 一个主键，否则返回 null。 */
function eventToAccelerator(e: KeyboardEvent): string | null {
  const parts: string[] = []
  if (e.ctrlKey || e.metaKey) parts.push('CmdOrCtrl')
  if (e.altKey) parts.push('Alt')
  if (e.shiftKey) parts.push('Shift')
  const key = mainKeyToken(e.code)
  if (!key || parts.length === 0) return null
  parts.push(key)
  return parts.join('+')
}

function onKeydown(e: KeyboardEvent): void {
  if (!recording.value) return
  e.preventDefault()
  e.stopPropagation()
  // Esc 取消录制，不改动当前值。
  if (e.code === 'Escape') {
    recording.value = false
    return
  }
  const acc = eventToAccelerator(e)
  if (!acc) return // 仅按下修饰键 / 不支持的主键：继续等待有效组合。
  recording.value = false
  emit('update:modelValue', acc)
}

/** 将 accelerator 转为人类可读展示（`CmdOrCtrl` → `Ctrl`，`+` → ` + `）。 */
const display = computed(() =>
  props.modelValue
    ? props.modelValue
        .replace(/CmdOrCtrl/g, 'Ctrl')
        .split('+')
        .join(' + ')
    : '',
)

function startRecording(): void {
  recording.value = true
}

function clear(): void {
  recording.value = false
  if (props.modelValue !== '') emit('update:modelValue', '')
}
</script>

<template>
  <div class="shortcut-input">
    <button
      type="button"
      class="shortcut-input__field"
      :class="{ 'shortcut-input__field--recording': recording }"
      @click="startRecording"
      @keydown="onKeydown"
      @blur="recording = false"
    >
      <span v-if="recording" class="muted">{{ recordingHint ?? '按下快捷键…' }}</span>
      <span v-else-if="display">{{ display }}</span>
      <span v-else class="muted">{{ placeholder ?? '未设置' }}</span>
    </button>
    <button
      v-if="modelValue && !recording"
      type="button"
      class="shortcut-input__clear"
      aria-label="clear"
      @click="clear"
    >
      ✕
    </button>
  </div>
</template>

<style scoped>
.shortcut-input {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.shortcut-input__field {
  min-width: 140px;
  padding: 4px 10px;
  font: inherit;
  text-align: center;
  color: var(--text);
  background: var(--surface-2, rgba(127, 127, 127, 0.1));
  border: 1px solid var(--border, rgba(127, 127, 127, 0.3));
  border-radius: 6px;
  cursor: pointer;
}

.shortcut-input__field--recording {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 30%, transparent);
}

.shortcut-input__clear {
  padding: 0 4px;
  font-size: 0.8rem;
  line-height: 1;
  color: var(--text-muted, #888);
  background: transparent;
  border: none;
  cursor: pointer;
}

.shortcut-input__clear:hover {
  color: var(--text);
}
</style>
