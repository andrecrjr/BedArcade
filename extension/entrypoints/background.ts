import { isTabActionMessage } from '../lib/tab-actions';
import {
  ONBOARDING_COMPLETED_STORAGE_KEY,
  onboardingCompletionAfterInstall,
} from '../lib/onboarding';
import {
  buildTvModeRules,
  isSaveTvModeProfileMessage,
  loadTvModeProfiles,
  normalizeTvModeProfile,
  persistTvModeProfile,
  TV_MODE_RULE_ID_END,
  TV_MODE_RULE_ID_START,
  TV_MODE_SCRIPT_ID,
} from '../lib/tv-mode';

async function reconcileTvMode() {
  const profiles = await loadTvModeProfiles();
  const rules = buildTvModeRules(profiles);
  const currentRules = await chrome.declarativeNetRequest.getDynamicRules();
  const ownedRuleIds = currentRules
    .filter((rule) => rule.id >= TV_MODE_RULE_ID_START && rule.id <= TV_MODE_RULE_ID_END)
    .map((rule) => rule.id);

  await chrome.declarativeNetRequest.updateDynamicRules({
    removeRuleIds: ownedRuleIds,
    addRules: rules,
  });

  const registeredScripts = await chrome.scripting.getRegisteredContentScripts({
    ids: [TV_MODE_SCRIPT_ID],
  });
  if (registeredScripts.length) {
    await chrome.scripting.unregisterContentScripts({ ids: [TV_MODE_SCRIPT_ID] });
  }

  const userAgentHosts = profiles
    .filter((profile) => profile.enabled && !profile.tvUrl)
    .map((profile) => profile.hostname);
  const matches = userAgentHosts.map((hostname) => `*://${hostname}/*`);
  if (matches.length) {
    await chrome.scripting.registerContentScripts([
      {
        id: TV_MODE_SCRIPT_ID,
        matches,
        js: ['tv-user-agent.js'],
        runAt: 'document_start',
        world: 'MAIN',
        persistAcrossSessions: true,
      },
    ]);
  }
}

async function refreshAllTabs() {
  const tabs = await chrome.tabs.query({});
  const extensionUrl = chrome.runtime.getURL('');
  await Promise.all(
    tabs.map(async (tab) => {
      const url = tab.url ?? '';
      if (
        typeof tab.id !== 'number' ||
        (!/^https?:\/\//.test(url) && !url.startsWith(extensionUrl))
      )
        return;
      try {
        await chrome.tabs.reload(tab.id);
      } catch {}
    }),
  );
}

async function initializeOnboarding(reason: string) {
  const stored = await chrome.storage.local.get(ONBOARDING_COMPLETED_STORAGE_KEY);
  const completion = onboardingCompletionAfterInstall(
    reason,
    stored[ONBOARDING_COMPLETED_STORAGE_KEY],
  );
  if (completion !== null) {
    await chrome.storage.local.set({ [ONBOARDING_COMPLETED_STORAGE_KEY]: completion });
  }
  if (reason === 'install') {
    await chrome.tabs.create({ url: chrome.runtime.getURL('newtab.html') });
  }
}

export default defineBackground(() => {
  const tabSwitchQueues = new Map<number, Promise<void>>();
  let tvModeQueue = Promise.resolve();

  chrome.runtime.onInstalled.addListener(({ reason }) => {
    void refreshAllTabs().catch(() => {});
    void initializeOnboarding(reason).catch(() => {
      if (reason === 'install') {
        void chrome.tabs.create({ url: chrome.runtime.getURL('newtab.html') }).catch(() => {});
      }
    });
    void queueTvModeReconciliation().catch(() => {});
  });

  chrome.runtime.onStartup.addListener(() => {
    void refreshAllTabs().catch(() => {});
  });

  void queueTvModeReconciliation().catch(() => {});

  chrome.runtime.onMessage.addListener((message: unknown, sender, sendResponse) => {
    if (!isSaveTvModeProfileMessage(message)) return;
    if (sender.url !== chrome.runtime.getURL('newtab.html')) return;

    void (async () => {
      const profile = normalizeTvModeProfile(message.profile);
      if (!profile) return { ok: false, error: 'Enter a valid same-host TV landing URL.' };

      await persistTvModeProfile(profile);
      await queueTvModeReconciliation();
      return { ok: true };
    })()
      .then(sendResponse)
      .catch(() => sendResponse({ ok: false, error: 'Could not save TV mode settings.' }));

    return true;
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
      if (message.action === 'new-tab') {
        await chrome.tabs.create({ windowId, url: chrome.runtime.getURL('newtab.html') });
        return;
      }

      const previous = tabSwitchQueues.get(windowId) ?? Promise.resolve();
      const next = previous
        .then(async () => {
          const tabs = await chrome.tabs.query({ windowId });
          const ordered = tabs
            .filter((tab) => typeof tab.index === 'number')
            .toSorted((a, b) => a.index - b.index);
          const current = ordered.findIndex((tab) => tab.active);
          if (current < 0 || ordered.length < 2) return;

          const step = message.action === 'next-tab' ? 1 : -1;
          const target = ordered[(current + step + ordered.length) % ordered.length];
          if (typeof target?.id === 'number') await chrome.tabs.update(target.id, { active: true });
        })
        .catch(() => {});

      tabSwitchQueues.set(windowId, next);
      void next.then(() => {
        if (tabSwitchQueues.get(windowId) === next) tabSwitchQueues.delete(windowId);
      });
    })().catch(() => {});
  });

  function queueTvModeReconciliation() {
    const next = tvModeQueue.then(reconcileTvMode);
    tvModeQueue = next.catch(() => {});
    return next;
  }
});
