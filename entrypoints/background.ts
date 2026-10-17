import { isTabActionMessage } from '../lib/tab-actions';

export default defineBackground(() => {
  chrome.runtime.onInstalled.addListener(({ reason }) => {
    if (reason === 'install') {
      void chrome.tabs.create({ url: chrome.runtime.getURL('newtab.html') }).catch(() => {});
    }
  });

  chrome.runtime.onMessage.addListener((message: unknown, sender) => {
    if (!isTabActionMessage(message)) return;

    const isNewTabPage = sender.url === chrome.runtime.getURL('newtab.html');
    const tabId = sender.tab?.id ?? (isNewTabPage ? message.tabId : undefined);
    if (typeof tabId !== 'number') return;

    if (message.action === 'back') {
      void chrome.tabs.goBack(tabId).catch(() => {});
      return;
    }

    void (async () => {
      const windowId = sender.tab?.windowId ?? (await chrome.tabs.get(tabId)).windowId;
      const tabs = await chrome.tabs.query({ windowId });
      const ordered = tabs
        .filter((tab) => typeof tab.index === 'number')
        .toSorted((a, b) => a.index - b.index);
      const current = ordered.findIndex((tab) => tab.id === tabId);
      if (current < 0 || ordered.length < 2) return;

      const step = message.action === 'next-tab' ? 1 : -1;
      const target = ordered[(current + step + ordered.length) % ordered.length];
      if (typeof target?.id === 'number') await chrome.tabs.update(target.id, { active: true });
    })().catch(() => {});
  });
});
