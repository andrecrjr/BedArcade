import { defineConfig } from 'wxt';

export default defineConfig({
  manifest: {
    name: '__MSG_extensionName__',
    description: '__MSG_extensionDescription__',
    default_locale: 'en',
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
