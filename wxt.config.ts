import { defineConfig } from 'wxt';

export default defineConfig({
  manifest: {
    name: 'Gamepad Navigator',
    description: 'Navigate the web with a gamepad.',
    icons: {
      16: '/icons/icon-16.png',
      32: '/icons/icon-32.png',
      48: '/icons/icon-48.png',
      128: '/icons/icon-128.png',
    },
    permissions: ['storage', 'declarativeNetRequestWithHostAccess', 'scripting', 'favicon'],
    host_permissions: ['http://*/*', 'https://*/*'],
  },
});
