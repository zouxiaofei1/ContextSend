<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useAppStore } from './stores/app'
import AppSidebar from './components/AppSidebar.vue'
import ReceivePanel from './components/ReceivePanel.vue'
import DevicePanel from './components/DevicePanel.vue'
import SettingsPanel from './components/SettingsPanel.vue'

const app = useAppStore()

const activeTab = ref<string>('receive')

/** 面板懒加载映射 */
const panelMap: Record<string, unknown> = {
  receive: ReceivePanel,
  devices: DevicePanel,
  settings: SettingsPanel,
}

onMounted(() => {
  void app.init()
})

function onSelectTab(tab: string): void {
  activeTab.value = tab
}
</script>

<template>
  <div class="app-layout">
    <AppSidebar :active-tab="activeTab" @select="onSelectTab" />
    <main class="app-main">
      <component :is="panelMap[activeTab]" />
    </main>
  </div>
</template>

<style scoped>
.app-layout {
  display: flex;
  height: 100vh;
  width: 100vw;
}

.app-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--bg-primary);
}
</style>
