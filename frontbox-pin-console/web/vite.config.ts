import { defineConfig } from 'vite'
import solid from 'vite-plugin-solid'

export default defineConfig({
  plugins: [solid()],
  server: {
    // in dev, the page is served by vite but the data still comes from the running machine
    proxy: {
      '/ws': { target: 'ws://localhost:3000', ws: true },
    },
  },
})
