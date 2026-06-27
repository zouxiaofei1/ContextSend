<script setup lang="ts" generic="T extends string">
import { ref, computed, onBeforeUnmount, nextTick } from 'vue'

const props = defineProps<{
  modelValue: T
  options: ReadonlyArray<{ value: T; label: string }>
  minWidth?: string
}>()

const emit = defineEmits<{ 'update:modelValue': [T] }>()

// ---- state ----
const open = ref(false)
const activeIndex = ref(-1)

// ---- refs ----
const triggerRef = ref<HTMLButtonElement | null>(null)
const menuRef = ref<HTMLDivElement | null>(null)
const optionRefs = ref<HTMLElement[]>([])

// ---- computed ----
const selectedLabel = computed(
  () => props.options.find((o) => o.value === props.modelValue)?.label ?? props.modelValue,
)

/** dropdown 面板的 fixed 坐标 */
interface PanelRect {
  top: number
  left: number
  minWidth: number
  openUpward: boolean
}
const panelRect = ref<PanelRect>({ top: 0, left: 0, minWidth: 120, openUpward: false })

// ---- open / close ----
function openDropdown(): void {
  if (open.value) return
  updatePanelRect()
  activeIndex.value = props.options.findIndex((o) => o.value === props.modelValue)
  open.value = true
  nextTick(() => focusActiveOption())
}

function closeDropdown(): void {
  open.value = false
  activeIndex.value = -1
  triggerRef.value?.focus()
}

function toggle(): void {
  open.value ? closeDropdown() : openDropdown()
}

// ---- selection ----
function select(value: T): void {
  emit('update:modelValue', value)
  closeDropdown()
}

// ---- keyboard ----
function onTriggerKeydown(e: KeyboardEvent): void {
  if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
    e.preventDefault()
    openDropdown()
    if (e.key === 'ArrowUp') {
      activeIndex.value = props.options.length - 1
      nextTick(() => focusActiveOption())
    }
  } else if (e.key === 'Enter' || e.key === ' ') {
    e.preventDefault()
    toggle()
  }
}

function onMenuKeydown(e: KeyboardEvent): void {
  switch (e.key) {
    case 'ArrowDown':
      e.preventDefault()
      activeIndex.value = (activeIndex.value + 1) % props.options.length
      nextTick(() => focusActiveOption())
      break
    case 'ArrowUp':
      e.preventDefault()
      activeIndex.value =
        (activeIndex.value - 1 + props.options.length) % props.options.length
      nextTick(() => focusActiveOption())
      break
    case 'Enter':
      e.preventDefault()
      if (activeIndex.value >= 0 && activeIndex.value < props.options.length) {
        select(props.options[activeIndex.value].value)
      }
      break
    case 'Escape':
      e.preventDefault()
      closeDropdown()
      break
  }
}

function focusActiveOption(): void {
  const el = optionRefs.value[activeIndex.value]
  el?.focus()
}

// ---- position ----
function updatePanelRect(): void {
  const trigger = triggerRef.value
  if (!trigger) return
  const rect = trigger.getBoundingClientRect()
  const gap = 4
  const estPanelH = Math.min(props.options.length, 10) * 36 + 12
  const spaceBelow = window.innerHeight - rect.bottom - gap
  const openUpward = spaceBelow < estPanelH && rect.top > estPanelH + gap

  panelRect.value = {
    top: openUpward ? rect.top - gap : rect.bottom + gap,
    left: rect.left,
    minWidth: rect.width,
    openUpward,
  }
}

// ---- click outside ----
function onDocClick(e: MouseEvent): void {
  if (!open.value) return
  const target = e.target as Node
  if (triggerRef.value?.contains(target)) return
  if (menuRef.value?.contains(target)) return
  closeDropdown()
}
document.addEventListener('click', onDocClick)
onBeforeUnmount(() => document.removeEventListener('click', onDocClick))

// ---- resize / scroll reposition ----
function onReposition(): void {
  if (open.value) updatePanelRect()
}
window.addEventListener('resize', onReposition)
window.addEventListener('scroll', onReposition, true)
onBeforeUnmount(() => {
  window.removeEventListener('resize', onReposition)
  window.removeEventListener('scroll', onReposition, true)
})

// ---- option refs ----
function setOptionRef(el: unknown, i: number): void {
  if (el) optionRefs.value[i] = el as HTMLElement
}

// ---- Transition: JS-driven animation (direction-aware) ----
function onEnter(el: Element, done: () => void): void {
  const htmlEl = el as HTMLElement
  const dir = panelRect.value.openUpward ? '4px' : '-4px'
  // 起点
  htmlEl.style.opacity = '0'
  htmlEl.style.transform = `translateY(${dir})`
  htmlEl.style.transition = 'transform 0.15s ease-out, opacity 0.15s ease-out'
  // 强制回流
  void htmlEl.offsetHeight
  // 终点
  htmlEl.style.opacity = '1'
  htmlEl.style.transform = 'translateY(0)'
  htmlEl.addEventListener('transitionend', done, { once: true })
}

function onLeave(el: Element, done: () => void): void {
  const htmlEl = el as HTMLElement
  const dir = panelRect.value.openUpward ? '4px' : '-4px'
  htmlEl.style.transition = 'transform 0.12s ease-in, opacity 0.12s ease-in'
  htmlEl.style.opacity = '0'
  htmlEl.style.transform = `translateY(${dir})`
  htmlEl.addEventListener('transitionend', done, { once: true })
}

function onAfterLeave(): void {
  // 清理 inline styles（防止残留）
}
</script>

<template>
  <div class="combobox" :style="{ minWidth: props.minWidth ?? '120px' }">
    <!-- 触发器 -->
    <button
      ref="triggerRef"
      class="combobox-trigger"
      type="button"
      :aria-expanded="open"
      :aria-haspopup="'listbox'"
      @click="toggle"
      @keydown="onTriggerKeydown"
    >
      <span class="combobox-label">{{ selectedLabel }}</span>
      <svg
        class="combobox-chevron"
        :class="{ open }"
        viewBox="0 0 24 24"
        width="16"
        height="16"
        aria-hidden="true"
      >
        <path
          d="M6 9l6 6 6-6"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </button>

    <!-- 下拉面板（Teleport 到 body 避免滚动裁剪） -->
    <Teleport to="body">
      <Transition
        :css="false"
        @enter="onEnter"
        @leave="onLeave"
        @after-leave="onAfterLeave"
      >
        <div
          v-if="open"
          ref="menuRef"
          class="combobox-menu"
          role="listbox"
          :style="{
            position: 'fixed',
            top: panelRect.openUpward ? 'auto' : panelRect.top + 'px',
            bottom: panelRect.openUpward ? (window.innerHeight - panelRect.top) + 'px' : 'auto',
            left: panelRect.left + 'px',
            minWidth: panelRect.minWidth + 'px',
          }"
          @keydown="onMenuKeydown"
        >
          <button
            v-for="(opt, i) in props.options"
            :key="opt.value"
            :ref="(el) => setOptionRef(el, i)"
            class="combobox-option"
            :class="{
              selected: opt.value === props.modelValue,
              active: i === activeIndex,
            }"
            type="button"
            role="option"
            :aria-selected="opt.value === props.modelValue"
            @click="select(opt.value)"
            @mouseenter="activeIndex = i"
          >
            <span class="combobox-option-label">{{ opt.label }}</span>
            <svg
              v-if="opt.value === props.modelValue"
              class="combobox-check"
              viewBox="0 0 24 24"
              width="16"
              height="16"
              aria-hidden="true"
            >
              <path
                d="M5 13l4 4L19 7"
                fill="none"
                stroke="currentColor"
                stroke-width="2.5"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          </button>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped>
/* ===== 容器 ===== */
.combobox {
  position: relative;
  display: inline-block;
}

/* ===== 触发器按钮 ===== */
.combobox-trigger {
  display: inline-flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  width: 100%;
  background: var(--bg-primary);
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text-primary);
  padding: 0.45rem 0.65rem;
  font-family: inherit;
  font-size: 0.9rem;
  line-height: 1.5;
  cursor: pointer;
  transition: border-color 0.15s;
  text-align: left;
}

.combobox-trigger:hover {
  border-color: var(--text-secondary);
}

.combobox-trigger:focus {
  outline: none;
  border-color: var(--accent);
}

.combobox-label {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* ===== Chevron ===== */
.combobox-chevron {
  flex-shrink: 0;
  color: var(--text-secondary);
  transition: transform 0.2s ease;
}

.combobox-chevron.open {
  transform: rotate(180deg);
}
</style>

<!-- 全局样式（Teleport 到 body，不能 scoped） -->
<style>
.combobox-menu {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  box-shadow: 0 8px 24px var(--shadow);
  padding: 0.25rem;
  z-index: 1000;
  max-height: 240px;
  overflow-y: auto;
}

/* ===== 选项按钮 ===== */
.combobox-option {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  width: 100%;
  text-align: left;
  white-space: nowrap;
  background: transparent;
  color: var(--text-primary);
  border: none;
  border-radius: 5px;
  padding: 0.4rem 0.6rem;
  font-family: inherit;
  font-size: 0.875rem;
  line-height: 1.5;
  cursor: pointer;
  transition: background 0.1s, color 0.1s;
}

.combobox-option:hover,
.combobox-option.active {
  background: var(--accent);
  color: #fff;
}

.combobox-option:focus {
  outline: none;
  background: var(--accent);
  color: #fff;
}

/* 选中态：未 hover/focus 时用 accent 色标记 */
.combobox-option.selected:not(:hover):not(.active):not(:focus) {
  color: var(--accent);
  font-weight: 500;
}

.combobox-option-label {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
}

.combobox-check {
  flex-shrink: 0;
  color: currentColor;
}
</style>
