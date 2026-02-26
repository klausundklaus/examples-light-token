import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import { nodePolyfills } from 'vite-plugin-node-polyfills';

export default defineConfig({
  plugins: [react(), nodePolyfills({ include: ['buffer'] })],
  test: {
    environment: 'happy-dom',
    globals: true,
    include: ['src/**/integration/**/*.test.ts'],
    testTimeout: 30_000,
    sequence: { concurrent: false },
    fileParallelism: false,
  },
});
