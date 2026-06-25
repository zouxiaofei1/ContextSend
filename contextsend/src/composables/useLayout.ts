import { ref, onMounted, onUnmounted } from 'vue'

/** 竖屏切换阈值：窗口宽度低于此值时切换为底部 tab 布局 */
const PORTRAIT_BREAKPOINT = 500
/** 侧边栏折叠阈值：宽度低于此值时侧边栏仅显示图标 */
const COMPACT_BREAKPOINT = 720

const isPortrait = ref(false)
const isCompact = ref(false)

function update() {
  isPortrait.value = window.innerWidth < PORTRAIT_BREAKPOINT
  isCompact.value = window.innerWidth < COMPACT_BREAKPOINT
}

export function useLayout() {
  onMounted(() => {
    update()
    window.addEventListener('resize', update)
  })

  onUnmounted(() => {
    window.removeEventListener('resize', update)
  })

  return { isPortrait, isCompact }
}
