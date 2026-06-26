<script setup lang="ts">
defineProps<{ title: string }>()
</script>

<template>
  <section class="settings-section">
    <h2>{{ title }}</h2>
    <slot />
  </section>
</template>

<style scoped>
.settings-section {
  margin-bottom: 2rem;
}

.settings-section h2 {
  margin: 0 0 1rem;
  font-size: 1.05rem;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
</style>
