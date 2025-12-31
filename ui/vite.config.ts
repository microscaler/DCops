import { defineConfig } from 'vite';
import solid from 'vite-plugin-solid';
import path from 'path';

export default defineConfig({
  plugins: [solid()],
  base: '/',
  publicDir: 'public',
  build: {
    outDir: 'dist',
    assetsDir: 'assets',
    copyPublicDir: true,
    rollupOptions: {
      output: {
        manualChunks: (id) => {
          if (id.includes('mermaid')) {
            return 'mermaid';
          }
          if (id.includes('katex')) {
            return 'katex';
          }
          if (id.includes('marked')) {
            return 'marked';
          }
          if (id.includes('minisearch')) {
            return 'minisearch';
          }
          if (id.includes('/pages/')) {
            const match = id.match(/\/pages\/([^/]+)/);
            return match ? `page-${match[1]}` : 'pages';
          }
          if (id.includes('/data/content/')) {
            return 'content-data';
          }
        },
      },
    },
    chunkSizeWarningLimit: 2000,
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  server: {
    port: 3003,
    open: true,
  },
});

