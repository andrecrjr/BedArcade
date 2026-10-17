import { defineConfig } from 'wxt';

export default defineConfig({
  manifest: {
    name: 'Gamepad Navigator',
    description: 'Navigate the web with a gamepad.',
    permissions: ['storage'],
  },
});
