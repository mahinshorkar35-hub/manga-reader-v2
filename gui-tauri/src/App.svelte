<script>
  /**
   * App.svelte — Root application component.
   * Provides sidebar navigation, page routing, and toast notifications.
   */
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import Library from './routes/Library.svelte';
  import Reader from './routes/Reader.svelte';
  import Settings from './routes/Settings.svelte';
  import ThemeToggle from './components/ThemeToggle.svelte';
  import {
    currentRoute,
    theme,
    settings,
    isSidebarOpen,
    toastMessage,
    navigateTo,
    showToast,
  } from './lib/stores/index.js';

  onMount(async () => {
    // Load settings from Rust engine on startup
    try {
      const data = await invoke('get_settings');
      if (data && data.theme) {
        settings.set({
          theme: data.theme,
          readingDirection: data.reading_direction || 'ltr',
          pageFitMode: data.page_fit_mode || 'fit-width',
          showPageNumbers: data.show_page_numbers ?? true,
          doublePageMode: data.double_page_mode ?? false,
          backgroundColor: data.background_color || '#1a1a2e',
          mangaRootPath: data.manga_root_path || '',
          ttsEnabled: data.tts_enabled ?? false,
          ttsVoice: data.tts_voice || 'af_heart',
          ttsSpeed: data.tts_speed || 1.0,
          visionBackend: data.vision_backend || 'gemini',
          geminiApiKey: data.gemini_api_key || '',
          openaiApiKey: data.openai_api_key || '',
          elevenlabsApiKey: data.elevenlabs_api_key || '',
        });
        theme.init(data.theme);
      }
    } catch (err) {
      console.warn('Could not load settings:', err);
    }
  });

  // Navigation items
  const navItems = [
    {
      id: 'library',
      label: 'Library',
      icon: `<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"/><path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"/></svg>`,
    },
    {
      id: 'settings',
      label: 'Settings',
      icon: `<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>`,
    },
  ];
</script>

<div class="app-layout">
  <!-- Sidebar (hidden in reader mode) -->
  {#if $currentRoute !== 'reader'}
    <aside class="sidebar" class:collapsed={!$isSidebarOpen}>
      <div class="sidebar-header">
        <div class="sidebar-logo">
          <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="var(--color-accent)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20" />
            <path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z" />
            <line x1="8" y1="7" x2="14" y2="7" />
            <line x1="8" y1="11" x2="14" y2="11" />
            <line x1="8" y1="15" x2="12" y2="15" />
          </svg>
          <span class="sidebar-brand">Manga Reader</span>
        </div>
        <ThemeToggle />
      </div>

      <nav class="sidebar-nav" aria-label="Main navigation">
        {#each navItems as item}
          <button
            class="nav-item"
            class:active={$currentRoute === item.id}
            on:click={() => navigateTo(item.id)}
            aria-current={$currentRoute === item.id ? 'page' : undefined}
          >
            <span class="nav-icon">{@html item.icon}</span>
            <span class="nav-label">{item.label}</span>
          </button>
        {/each}
      </nav>

      <div class="sidebar-footer">
        <span class="version">v2.0.0 — Tauri</span>
      </div>
    </aside>
  {/if}

  <!-- Main content area -->
  <main class="main-content">
    {#if $currentRoute === 'library'}
      <Library />
    {:else if $currentRoute === 'reader'}
      <Reader />
    {:else if $currentRoute === 'settings'}
      <Settings />
    {/if}
  </main>
</div>

<!-- Toast notifications -->
{#if $toastMessage}
  <div class="toast toast-{$toastMessage.type}" role="alert">
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
      {#if $toastMessage.type === 'success'}
        <polyline points="20 6 9 17 4 12" />
      {:else if $toastMessage.type === 'error'}
        <circle cx="12" cy="12" r="10" />
        <line x1="12" y1="8" x2="12" y2="12" />
        <line x1="12" y1="16" x2="12.01" y2="16" />
      {:else}
        <circle cx="12" cy="12" r="10" />
        <line x1="12" y1="16" x2="12" y2="12" />
        <line x1="12" y1="8" x2="12.01" y2="8" />
      {/if}
    </svg>
    <span>{$toastMessage.message}</span>
  </div>
{/if}

<style>
  .app-layout {
    display: flex;
    height: 100%;
    overflow: hidden;
  }

  /* ================================================================
     Sidebar
     ================================================================ */
  .sidebar {
    display: flex;
    flex-direction: column;
    width: 240px;
    min-width: 240px;
    background: var(--color-bg-secondary);
    border-right: 1px solid var(--color-border);
    transition: width var(--transition-normal), min-width var(--transition-normal);
    overflow: hidden;
  }

  .sidebar.collapsed {
    width: 0;
    min-width: 0;
    border-right: none;
  }

  .sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-4) var(--space-4);
    border-bottom: 1px solid var(--color-border);
  }

  .sidebar-logo {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .sidebar-brand {
    font-size: var(--text-lg);
    font-weight: 700;
    color: var(--color-text-primary);
    white-space: nowrap;
  }

  .sidebar-nav {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-3);
    overflow-y: auto;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    border: none;
    border-radius: var(--radius-md);
    background: none;
    color: var(--color-text-secondary);
    font-family: inherit;
    font-size: var(--text-sm);
    font-weight: 500;
    cursor: pointer;
    transition: all var(--transition-fast);
    text-align: left;
    width: 100%;
  }

  .nav-item:hover {
    background: var(--color-bg-hover);
    color: var(--color-text-primary);
  }

  .nav-item.active {
    background: rgba(124, 58, 237, 0.12);
    color: var(--color-accent);
  }

  .nav-item:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }

  .nav-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    flex-shrink: 0;
  }

  .nav-label {
    white-space: nowrap;
  }

  .sidebar-footer {
    padding: var(--space-3) var(--space-4);
    border-top: 1px solid var(--color-border);
  }

  .version {
    font-size: var(--text-xs);
    color: var(--color-text-tertiary);
  }

  /* ================================================================
     Main content
     ================================================================ */
  .main-content {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    background: var(--color-bg-primary);
  }

  /* ================================================================
     Toast notifications
     ================================================================ */
  .toast {
    position: fixed;
    bottom: var(--space-6);
    right: var(--space-6);
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-5);
    border-radius: var(--radius-md);
    background: var(--color-bg-elevated);
    border: 1px solid var(--color-border);
    box-shadow: var(--shadow-lg);
    color: var(--color-text-primary);
    font-size: var(--text-sm);
    font-weight: 500;
    z-index: 9999;
    animation: toastIn var(--transition-normal) ease forwards;
    max-width: 400px;
  }

  .toast-success {
    border-color: var(--color-success);
  }

  .toast-success svg {
    color: var(--color-success);
  }

  .toast-error {
    border-color: var(--color-danger);
  }

  .toast-error svg {
    color: var(--color-danger);
  }

  .toast-info {
    border-color: var(--color-info);
  }

  .toast-info svg {
    color: var(--color-info);
  }

  @keyframes toastIn {
    from {
      opacity: 0;
      transform: translateY(12px) scale(0.96);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }
</style>
