import { ref, onMounted, onUnmounted } from 'vue'
import { PORTRAIT_BREAKPOINT, COMPACT_BREAKPOINT } from '../constants'

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
