import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// Tauri 期望固定端口与可预测的 dev server，详见 tauri.conf.json 的 devUrl。
const host = process.env.TAURI_DEV_HOST

export default defineConfig({
  plugins: [vue()],
  // 避免 Vite 清屏遮挡 Rust 编译输出
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 由 cargo 监听 Rust 改动，Vite 忽略 src-tauri
      ignored: ['**/src-tauri/**'],
    },
  },
})
