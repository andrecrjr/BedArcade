export const ONBOARDING_COMPLETED_STORAGE_KEY = 'onboardingCompleted';

export function onboardingCompletionAfterInstall(reason: string, storedValue: unknown) {
  if (reason === 'install') return false;
  if (reason === 'update' && typeof storedValue !== 'boolean') return true;
  return null;
}

export async function loadOnboardingCompleted(): Promise<boolean> {
  try {
    const stored = await chrome.storage.local.get(ONBOARDING_COMPLETED_STORAGE_KEY);
    return stored[ONBOARDING_COMPLETED_STORAGE_KEY] === true;
  } catch {
    return true;
  }
}

export async function completeOnboarding(): Promise<void> {
  try {
    await chrome.storage.local.set({ [ONBOARDING_COMPLETED_STORAGE_KEY]: true });
  } catch {}
}
