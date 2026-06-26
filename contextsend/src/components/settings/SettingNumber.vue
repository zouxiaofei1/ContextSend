<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  modelValue: number
  min?: number
  max?: number
  placeholder?: string
  /** 当值等于该值时输入框显示为空（如端口 0、最大条数 -1）。 */
  emptyValue?: number
  /** 数值后缀单位文案（如「秒」）。 */
  unit?: string
}>()
const emit = defineEmits<{ 'update:modelValue': [number] }>()

const display = computed(() =>
  props.emptyValue !== undefined && props.modelValue === props.emptyValue ? '' : props.modelValue,
)
</script>

<template>
  <input
    type="number"
    class="number-input"
    :value="display"
    :placeholder="placeholder"
    :min="min"
    :max="max"
    @change="emit('update:modelValue', Number(($event.target as HTMLInputElement).value))"
  />
  <span v-if="unit" class="muted unit">{{ unit }}</span>
</template>

<style scoped>
.number-input {
  width: 100px;
  text-align: right;
}

.unit {
  font-size: 0.8rem;
  white-space: nowrap;
}
</style>
