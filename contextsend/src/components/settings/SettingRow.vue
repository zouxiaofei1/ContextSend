<script setup lang="ts">
defineProps<{ label: string }>()
</script>

<template>
  <div class="setting-row">
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
