<script setup lang="ts">
import { onMounted } from 'vue'
import { useAppStore } from './stores/app'

const app = useAppStore()

onMounted(() => {
  void app.loadAppInfo()
})
</script>

<template>
  <main class="app">
    <header class="app__header">
      <h1>ContextSend</h1>
      <p class="app__subtitle">局域网内分享本地 Chat AI 上下文</p>
    </header>

    <section class="card">
      <h2>运行信息</h2>
      <p v-if="app.loading">加载中…</p>
      <p v-else-if="app.error" class="error">读取失败：{{ app.error }}</p>
      <ul v-else-if="app.info">
        <li>版本：{{ app.info.version }}</li>
        <li>平台：{{ app.info.platform }}</li>
        <li>内置适配器：{{ app.info.adapters.join('、') }}</li>
      </ul>
    </section>

    <section class="card card--muted">
      <h2>设备列表</h2>
      <p>局域网设备发现将在 Phase 1（Network layer）实现。</p>
    </section>
  </main>
</template>

<style scoped>
.app {
  max-width: 720px;
  margin: 0 auto;
  padding: 2rem 1.5rem;
}

.app__header h1 {
  margin: 0;
  font-size: 1.8rem;
}

.app__subtitle {
  margin: 0.25rem 0 1.5rem;
  color: #9aa0aa;
}

.card {
  background: #24272e;
  border: 1px solid #30343c;
  border-radius: 10px;
  padding: 1rem 1.25rem;
  margin-bottom: 1rem;
}

.card h2 {
  margin: 0 0 0.5rem;
  font-size: 1.05rem;
}

.card ul {
  margin: 0;
  padding-left: 1.1rem;
}

.card--muted {
  color: #9aa0aa;
}

.error {
  color: #ff7a7a;
}
</style>
