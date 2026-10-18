import { translate, type AppLocale } from './i18n';

function dispatchSelection(select: HTMLSelectElement) {
  select.dispatchEvent(new Event('input', { bubbles: true, composed: true }));
  select.dispatchEvent(new Event('change', { bubbles: true, composed: true }));
}

export function createSelectPicker(select: HTMLSelectElement, locale: AppLocale) {
  const label =
    select.labels?.[0]?.textContent?.trim() ||
    select.getAttribute('aria-label')?.trim() ||
    translate(locale, 'selectPickerTitle');
  const dialog = document.createElement('dialog');
  const heading = document.createElement('h2');
  const hint = document.createElement('p');
  const list = document.createElement('div');
  const enabledButtons: HTMLButtonElement[] = [];

  dialog.dataset.gamepadNavigator = 'select-picker';
  dialog.setAttribute('aria-label', label);
  Object.assign(dialog.style, {
    position: 'fixed',
    inset: '50% auto auto 50%',
    transform: 'translate(-50%, -50%)',
    margin: '0',
    width: 'min(440px, calc(100vw - 32px))',
    maxHeight: 'min(80vh, 560px)',
    overflow: 'hidden',
    boxSizing: 'border-box',
    border: '1px solid rgba(164,184,210,.35)',
    borderRadius: '14px',
    padding: '20px',
    background: '#101b2a',
    color: '#f5f8fc',
    boxShadow: '0 16px 48px rgba(0,0,0,.55)',
    font: '14px system-ui,sans-serif',
    zIndex: '2147483647',
  });
  heading.textContent = label;
  Object.assign(heading.style, { margin: '0 0 6px', fontSize: '18px' });
  hint.textContent = translate(
    locale,
    select.multiple ? 'selectMultiplePickerHint' : 'selectPickerHint',
  );
  Object.assign(hint.style, { margin: '0 0 14px', color: '#aebdd0', fontSize: '12px' });
  list.setAttribute('role', 'listbox');
  list.setAttribute('aria-label', label);
  if (select.multiple) list.setAttribute('aria-multiselectable', 'true');
  Object.assign(list.style, {
    display: 'grid',
    gap: '6px',
    maxHeight: 'min(55vh, 400px)',
    overflowY: 'auto',
  });

  let previousGroup: HTMLOptGroupElement | null = null;
  [...select.options].forEach((option, index) => {
    if (option.hidden) return;
    const group = option.parentElement instanceof HTMLOptGroupElement ? option.parentElement : null;
    if (group && group !== previousGroup && group.label) {
      const groupLabel = document.createElement('div');
      groupLabel.textContent = group.label;
      Object.assign(groupLabel.style, {
        padding: '8px 4px 2px',
        color: '#aebdd0',
        fontSize: '12px',
        fontWeight: '600',
      });
      list.append(groupLabel);
    }
    previousGroup = group;

    const button = document.createElement('button');
    button.type = 'button';
    button.dataset.optionIndex = String(index);
    button.setAttribute('role', 'option');
    button.setAttribute('aria-selected', String(option.selected));
    button.textContent = option.label;
    button.disabled = option.disabled || Boolean(group?.disabled);
    Object.assign(button.style, {
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'space-between',
      width: '100%',
      minHeight: '44px',
      border: '1px solid rgba(164,184,210,.22)',
      borderRadius: '8px',
      padding: '8px 12px',
      background: option.selected ? 'rgba(99,230,190,.18)' : 'rgba(137,158,184,.1)',
      color: option.disabled || group?.disabled ? '#8793a2' : '#f5f8fc',
      font: 'inherit',
      textAlign: 'left',
      cursor: option.disabled || group?.disabled ? 'default' : 'pointer',
    });
    button.addEventListener('click', () => {
      if (select.multiple) {
        option.selected = !option.selected;
        button.setAttribute('aria-selected', String(option.selected));
        button.style.background = option.selected ? 'rgba(99,230,190,.18)' : 'rgba(137,158,184,.1)';
        dispatchSelection(select);
        return;
      }

      const changed = select.selectedIndex !== index;
      select.selectedIndex = index;
      if (changed) dispatchSelection(select);
      dialog.close('select');
    });
    if (!button.disabled) enabledButtons.push(button);
    list.append(button);
  });

  dialog.append(heading, hint, list);
  return enabledButtons.length ? dialog : null;
}

export function showSelectPicker(select: HTMLSelectElement, locale: AppLocale) {
  const dialog = createSelectPicker(select, locale);
  if (!dialog) return null;

  const selectedOption = dialog.querySelector<HTMLButtonElement>(
    `[data-option-index="${select.selectedIndex}"]`,
  );
  const firstEnabledOption = dialog.querySelector<HTMLButtonElement>('button:not(:disabled)');
  const initialOption =
    selectedOption && !selectedOption.disabled ? selectedOption : firstEnabledOption;
  if (!initialOption) return null;

  document.body.append(dialog);
  dialog.showModal();
  initialOption.focus({ preventScroll: true });
  return { dialog, initialOption };
}
