<script setup lang="ts">
import { useI18n } from 'vue-i18n'

defineEmits<{
  back: []
}>()

const { t } = useI18n()

const cards = [
  { titleKey: 'device.troubleshootingLanTitle', descKey: 'device.troubleshootingLanDesc' },
  { titleKey: 'device.troubleshootingApTitle', descKey: 'device.troubleshootingApDesc' },
  { titleKey: 'device.troubleshootingFirewallTitle', descKey: 'device.troubleshootingFirewallDesc' },
]
</script>

<template>
  <div class="panel">
    <div class="header">
      <button class="ghost small back-btn" @click="$emit('back')">
        ← {{ t('device.troubleshootingBack') }}
      </button>
      <h3>{{ t('device.troubleshootingTitle') }}</h3>
    </div>

    <div class="cards">
      <div v-for="card in cards" :key="card.titleKey" class="card">
        <h4>{{ t(card.titleKey) }}</h4>
        <p class="muted">{{ t(card.descKey) }}</p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.panel {
  flex: 1;
  padding: 1.5rem;
  overflow-y: auto;
  user-select: none;
}

.header {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  margin-bottom: 1.25rem;
}

.header h3 {
  margin: 0;
}

.back-btn {
  flex: none;
}

.cards {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 1rem 1.25rem;
}

.card h4 {
  margin: 0 0 0.5rem;
  font-size: 0.95rem;
  color: var(--text-primary);
}

.card p {
  margin: 0;
  font-size: 0.85rem;
  line-height: 1.6;
}
</style>
