<script setup lang="ts">
import { useSettingsStore } from '../stores/settings'
import GeneralSettings from './settings/GeneralSettings.vue'
import AdvancedSettings from './settings/AdvancedSettings.vue'
import AboutSection from './settings/AboutSection.vue'

const settings = useSettingsStore()
</script>

<template>
  <div class="panel">
    <GeneralSettings />
    <AdvancedSettings v-if="settings.showAdvanced" />
    <AboutSection />
  </div>
</template>

<style scoped>
.panel {
  flex: 1;
  padding: 1.5rem 2rem;
  overflow-y: auto;

  width: 100%;
  align-self: center;
}
</style>
