<script setup lang="ts" generic="T extends string">
defineProps<{
  modelValue: T
  options: ReadonlyArray<{ value: T; label: string }>
  /** 下拉框最小宽度（CSS 值，默认 120px）。 */
  minWidth?: string
}>()
const emit = defineEmits<{ 'update:modelValue': [T] }>()
</script>

<template>
  <select
    class="setting-select"
    :style="{ minWidth: minWidth ?? '120px' }"
    :value="modelValue"
    @change="emit('update:modelValue', ($event.target as HTMLSelectElement).value as T)"
  >
    <option v-for="opt in options" :key="opt.value" :value="opt.value">
      {{ opt.label }}
    </option>
  </select>
</template>

<style scoped>
.setting-select {
  width: auto;
}
</style>
