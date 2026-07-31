const menuToggle = document.querySelector('.menu-toggle');
const siteNav = document.querySelector('.site-nav');

if (menuToggle && siteNav) {
  const closeMenu = () => {
    menuToggle.setAttribute('aria-expanded', 'false');
    siteNav.classList.remove('is-open');
  };

  menuToggle.addEventListener('click', () => {
    const willOpen = menuToggle.getAttribute('aria-expanded') !== 'true';
    menuToggle.setAttribute('aria-expanded', String(willOpen));
    siteNav.classList.toggle('is-open', willOpen);
  });

  siteNav.addEventListener('click', (event) => {
    if (event.target instanceof HTMLAnchorElement) {
      closeMenu();
    }
  });

  document.addEventListener('keydown', (event) => {
    if (event.key === 'Escape') {
      closeMenu();
      menuToggle.focus();
    }
  });
}

const tabs = Array.from(document.querySelectorAll('[data-command-tab]'));
const panels = Array.from(document.querySelectorAll('[data-command-panel]'));

const activateTab = (selectedTab) => {
  const selectedName = selectedTab.dataset.commandTab;

  tabs.forEach((tab) => {
    const isSelected = tab === selectedTab;
    tab.classList.toggle('is-active', isSelected);
    tab.setAttribute('aria-selected', String(isSelected));
    tab.tabIndex = isSelected ? 0 : -1;
  });

  panels.forEach((panel) => {
    panel.hidden = panel.dataset.commandPanel !== selectedName;
  });
};

tabs.forEach((tab, index) => {
  tab.addEventListener('click', () => activateTab(tab));
  tab.addEventListener('keydown', (event) => {
    if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) {
      return;
    }

    event.preventDefault();
    let nextIndex = index;

    if (event.key === 'ArrowLeft') nextIndex = (index - 1 + tabs.length) % tabs.length;
    if (event.key === 'ArrowRight') nextIndex = (index + 1) % tabs.length;
    if (event.key === 'Home') nextIndex = 0;
    if (event.key === 'End') nextIndex = tabs.length - 1;

    activateTab(tabs[nextIndex]);
    tabs[nextIndex].focus();
  });
});

const copyStatus = document.querySelector('.copy-status');

document.querySelectorAll('[data-copy-command]').forEach((button) => {
  button.addEventListener('click', async () => {
    const panelName = button.dataset.copyCommand;
    const command = document.querySelector(`[data-command-panel="${panelName}"] [data-command]`)?.textContent?.trim();

    if (!command) return;

    try {
      await navigator.clipboard.writeText(command);
      if (copyStatus) copyStatus.textContent = '已複製安裝指令。';
      button.textContent = '已複製';
      window.setTimeout(() => {
        button.textContent = '複製指令';
      }, 1800);
    } catch {
      if (copyStatus) copyStatus.textContent = '瀏覽器未允許自動複製，請手動選取指令。';
    }
  });
});
