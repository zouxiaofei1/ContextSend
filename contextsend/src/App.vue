<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useAppStore } from './stores/app'
import { useLayout } from './composables/useLayout'
import AppSidebar from './components/AppSidebar.vue'
import BottomTabBar from './components/BottomTabBar.vue'
import ReceivePanel from './components/ReceivePanel.vue'
import DevicePanel from './components/DevicePanel.vue'
import SettingsPanel from './components/SettingsPanel.vue'

const app = useAppStore()
const { isPortrait } = useLayout()

const activeTab = ref<string>('receive')

/** 面板懒加载映射 */
const panelMap: Record<string, unknown> = {
  receive: ReceivePanel,
  devices: DevicePanel,
  settings: SettingsPanel,
}

onMounted(() => {
  // 首帧已挂载，立即显示窗口（窗口配置为初始隐藏，避免冷启动白屏）。
  void getCurrentWindow().show()
  void app.init()
})

function onSelectTab(tab: string): void {
  activeTab.value = tab
}
</script>

<template>
  <!-- 横屏布局：左侧边栏 + 右侧内容 -->
  <div v-if="!isPortrait" class="app-layout">
    <AppSidebar :active-tab="activeTab" @select="onSelectTab" />
    <main class="app-main">
      <component :is="panelMap[activeTab]" />
    </main>
  </div>

  <!-- 竖屏布局：内容在上 + 底部 Tab 栏 -->
  <div v-else class="app-layout app-layout--portrait">
    <main class="app-main">
      <component :is="panelMap[activeTab]" />
    </main>
    <BottomTabBar :active-tab="activeTab" @select="onSelectTab" />
  </div>
</template>

<style scoped>
.app-layout {
  display: flex;
  height: 100vh;
  width: 100vw;
}

.app-layout--portrait {
  flex-direction: column;
}

.app-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--bg-primary);
}
</style>
