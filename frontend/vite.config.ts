import { defineConfig } from 'vite'
import { resolve } from 'node:path'

export default defineConfig({
  base: '/frontend',
  build: {
    minify: false,
    modulePreload: {
      polyfill: false
    },
    rollupOptions: {
      input: {
        main: resolve(__dirname, "index.html")
      },
      output: {
        format: "esm",
        manualChunks: {
          lit: ["lit"],
          webdav: ["webdav"],
        }
      }
    },
  },
})
