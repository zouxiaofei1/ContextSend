<script setup lang="ts">
defineProps<{ label: string; clickable?: boolean }>()
const emit = defineEmits<{ click: [] }>()
</script>

<template>
  <div
    class="setting-row"
    :class="{ 'setting-row--clickable': clickable }"
    :role="clickable ? 'button' : undefined"
    :tabindex="clickable ? 0 : undefined"
    @click="clickable && emit('click')"
    @keyup.enter="clickable && emit('click')"
  >
    <div class="setting-row__label">
      <slot name="label">
        <span>{{ label }}</span>
      </slot>
    </div>
    <div class="setting-row__control">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.setting-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.7rem 0;
  border-bottom: 1px solid var(--border);
}

.setting-row--clickable {
  cursor: pointer;
  margin: 0 -0.5rem;
  padding: 0.7rem 0.5rem;
  border-radius: 8px;
  transition: background 0.12s ease;
}

.setting-row--clickable:hover {
  background: var(--bg-secondary);
}

.setting-row:last-child {
  border-bottom: none;
}

.setting-row__label {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  font-size: 0.9rem;
}

.setting-row__control {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.9rem;
}
</style>
