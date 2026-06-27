<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAppStore } from '../stores/app'
import type { AdapterInfo } from '../stores/app'
import SplitButton from './SplitButton.vue'

const app = useAppStore()
const { t } = useI18n()

/** 下拉项类型（与 SplitButton 内部结构一致）。 */
interface SplitOption {
  value: string
  label: string
}

const peerName = computed(() => app.incoming?.peerName ?? '')

/** 已探测到（installed）的适配器，作为「接受」的可选导入目标。 */
const adapters = ref<AdapterInfo[]>([])
onMounted(async () => {
  try {
    adapters.value = (await app.listAdapters()).filter((a) => a.installed)
  } catch {
    adapters.value = []
  }
})

/** 下拉项：收件箱（默认）+ 每个可用适配器。value 空串表示收件箱。 */
const options = computed<SplitOption[]>(() => [
  { value: '', label: t('device.acceptToInbox') },
  ...adapters.value.map((a) => ({
    value: a.name,
    label: t('device.acceptToApp', { app: a.name }),
  })),
])

/** 主按钮：直接接受 → 落收件箱。 */
function onAccept(): void {
  void app.acceptIncoming()
}

/** 下拉选择：空串 → 收件箱；否则导入到该适配器。 */
function onSelect(value: string): void {
  void app.acceptIncoming(value || undefined)
}

function onReject(): void {
  void app.rejectIncoming()
}
</script>

<template>
  <div class="incoming">
    <div class="incoming-body">
      <h1 class="peer">「{{ peerName }}」</h1>
      <p class="subtitle">{{ t('device.receiveSubtitle') }}</p>

      <!-- 详细预览占位（暂不实现） -->
      <button class="detail" disabled>{{ t('device.receiveDetail') }}…</button>
    </div>

    <div class="actions">
      <button class="ghost" @click="onReject">{{ t('device.reject') }}</button>
      <SplitButton
        :label="t('device.acceptBtn')"
        :options="options"
        @main="onAccept"
        @select="onSelect"
      />
    </div>
  </div>
</template>

<style scoped>
.incoming {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 2rem;
  padding: 2rem;
  background: var(--bg-primary);
}

.incoming-body {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.6rem;
  text-align: center;
}

.peer {
  margin: 0;
  font-size: 1.6rem;
  font-weight: 600;
  color: var(--text-primary);
}

.subtitle {
  margin: 0;
  color: var(--text-secondary);
}

.detail {
  margin-top: 1.5rem;
  background: transparent;
  color: var(--text-secondary);
  border: 1px solid var(--border);
  cursor: not-allowed;
  opacity: 0.6;
}

.actions {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}
</style>
