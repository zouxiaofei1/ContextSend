<script setup lang="ts">
import { useNavItems } from '../composables/useNavItems'

defineProps<{
  activeTab: string
}>()

const emit = defineEmits<{
  select: [tab: string]
}>()

const { navItems } = useNavItems()
</script>

<template>
  <nav class="bottom-bar">
    <button
      v-for="item in navItems"
      :key="item.id"
      class="bottom-bar__item"
      :class="{ 'bottom-bar__item--active': activeTab === item.id }"
      @click="emit('select', item.id)"
    >
      <span class="bottom-bar__icon" v-html="item.icon"></span>
      <span class="bottom-bar__label">{{ item.label }}</span>
    </button>
  </nav>
</template>

<style scoped>
.bottom-bar {
  display: flex;
  align-items: center;
  justify-content: space-around;
  height: 56px;
  background: var(--bg-secondary);
  border-top: 1px solid var(--border);
  flex-shrink: 0;
  user-select: none;
  -webkit-user-select: none;
}

.bottom-bar__item {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 2px;
  padding: 0.25rem 1rem;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-secondary);
  font-size: 0.7rem;
  cursor: pointer;
  transition:
    background 0.12s,
    color 0.12s;
  min-width: 64px;
}

.bottom-bar__item:hover {
  color: var(--text-primary);
}

.bottom-bar__item--active {
  color: var(--accent) !important;
}

.bottom-bar__icon {
  width: 1.5rem;
  height: 1.5rem;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.bottom-bar__icon :deep(svg) {
  width: 1.25rem;
  height: 1.25rem;
  display: block;
}

.bottom-bar__icon :deep(svg path) {
  fill: currentColor;
}

.bottom-bar__label {
  line-height: 1;
}
</style>
