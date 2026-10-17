export type TabAction = 'back' | 'prev-tab' | 'next-tab';

export interface TabActionMessage {
  action: TabAction;
  tabId?: number;
}

export function isTabActionMessage(value: unknown): value is TabActionMessage {
  if (typeof value !== 'object' || value === null || !('action' in value)) return false;
  const message = value as Record<string, unknown>;
  return (
    (message.action === 'back' || message.action === 'prev-tab' || message.action === 'next-tab') &&
    (message.tabId === undefined || typeof message.tabId === 'number')
  );
}

export function sendTabAction(action: TabAction, tabId?: number) {
  void chrome.runtime.sendMessage({ action, tabId }).catch(() => {});
}
