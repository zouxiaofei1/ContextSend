import { ref, onMounted, onUnmounted } from 'vue'

/** 竖屏切换阈值：窗口宽度低于此值时切换为底部 tab 布局 */
const BREAKPOINT = 500

const isPortrait = ref(false)

function update() {
  isPortrait.value = window.innerWidth < BREAKPOINT
}

export function useLayout() {
  onMounted(() => {
    update()
    window.addEventListener('resize', update)
  })

  onUnmounted(() => {
    window.removeEventListener('resize', update)
  })

  return { isPortrait }
}
