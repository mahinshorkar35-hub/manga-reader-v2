<script>
  /**
   * Settings Page — Application configuration.
   */
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import ThemeToggle from '../components/ThemeToggle.svelte';
  import { settings, theme, showToast } from '../lib/stores/index.js';

  let isSaving = false;
  let isLoaded = false;

  onMount(async () => {
    try {
      const data = await invoke('get_settings');
      settings.set({
        theme: data.theme,
        readingDirection: data.reading_direction,
        pageFitMode: data.page_fit_mode,
        showPageNumbers: data.show_page_numbers,
        doublePageMode: data.double_page_mode,
        backgroundColor: data.background_color,
        mangaRootPath: data.manga_root_path,
        ttsEnabled: data.tts_enabled,
        ttsVoice: data.tts_voice,
        ttsSpeed: data.tts_speed,
        visionBackend: data.vision_backend,
        geminiApiKey: data.gemini_api_key,
        openaiApiKey: data.openai_api_key,
        elevenlabsApiKey: data.elevenlabs_api_key,
      });
      theme.init(data.theme);
      isLoaded = true;
    } catch (err) {
      showToast(`Failed to load settings: ${err}`, 'error');
      isLoaded = true;
    }
  });

  async function handleSave() {
    isSaving = true;
    try {
      const s = $settings;
      await invoke('save_settings', {
        newSettings: {
          theme: s.theme,
          reading_direction: s.readingDirection,
          page_fit_mode: s.pageFitMode,
          show_page_numbers: s.showPageNumbers,
          double_page_mode: s.doublePageMode,
          background_color: s.backgroundColor,
          manga_root_path: s.mangaRootPath,
          tts_enabled: s.ttsEnabled,
          tts_voice: s.ttsVoice,
          tts_speed: s.ttsSpeed,
          vision_backend: s.visionBackend,
          gemini_api_key: s.geminiApiKey,
          openai_api_key: s.openaiApiKey,
          elevenlabs_api_key: s.elevenlabsApiKey,
        },
      });
      theme.init(s.theme);
      showToast('Settings saved successfully', 'success');
    } catch (err) {
      showToast(`Failed to save settings: ${err}`, 'error');
    } finally {
      isSaving = false;
    }
  }

  async function handleReset() {
    try {
      const data = await invoke('reset_settings');
      settings.set({
        theme: data.theme,
        readingDirection: data.reading_direction,
        pageFitMode: data.page_fit_mode,
        showPageNumbers: data.show_page_numbers,
        doublePageMode: data.double_page_mode,
        backgroundColor: data.background_color,
        mangaRootPath: data.manga_root_path,
        ttsEnabled: data.tts_enabled,
        ttsVoice: data.tts_voice,
        ttsSpeed: data.tts_speed,
        visionBackend: data.vision_backend,
        geminiApiKey: data.gemini_api_key,
        openaiApiKey: data.openai_api_key,
        elevenlabsApiKey: data.elevenlabs_api_key,
      });
      theme.init('dark');
      showToast('Settings reset to defaults', 'info');
    } catch (err) {
      showToast(`Failed to reset settings: ${err}`, 'error');
    }
  }

  async function browseMangaRoot() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select Manga Root Directory',
      });
      if (selected) {
        settings.update((s) => ({ ...s, mangaRootPath: selected }));
      }
    } catch (err) {
      showToast(`Browse failed: ${err}`, 'error');
    }
  }
</script>

<div class="settings-page">
  <header class="page-header">
    <h1 class="page-title">Settings</h1>
    <ThemeToggle />
  </header>

  {#if !isLoaded}
    <div class="loading-state">
      <div class="spinner"></div>
      <p>Loading settings...</p>
    </div>
  {:else}
    <div class="settings-content">
      <!-- ============ Appearance ============ -->
      <section class="settings-section">
        <h2 class="section-title">Appearance</h2>

        <div class="setting-row">
          <div class="setting-info">
            <span class="setting-label">Theme</span>
            <p class="setting-desc">Choose between dark and light appearance</p>
          </div>
          <div class="setting-control">
            <button
              class="toggle-group"
              role="radiogroup"
              aria-label="Theme selection"
            >
              <button
                class="toggle-option"
                class:active={$settings.theme === 'dark'}
                on:click={() => {
                  settings.update((s) => ({ ...s, theme: 'dark' }));
                  theme.init('dark');
                }}
                role="radio"
                aria-checked={$settings.theme === 'dark'}
              >
                Dark
              </button>
              <button
                class="toggle-option"
                class:active={$settings.theme === 'light'}
                on:click={() => {
                  settings.update((s) => ({ ...s, theme: 'light' }));
                  theme.init('light');
                }}
                role="radio"
                aria-checked={$settings.theme === 'light'}
              >
                Light
              </button>
            </button>
          </div>
        </div>
      </section>

      <!-- ============ Reading ============ -->
      <section class="settings-section">
        <h2 class="section-title">Reading</h2>

        <div class="setting-row">
          <div class="setting-info">
            <span class="setting-label">Reading Direction</span>
            <p class="setting-desc">Left-to-right (Western) or right-to-left (Manga)</p>
          </div>
          <div class="setting-control">
            <div class="toggle-group" role="radiogroup">
              <button
                class="toggle-option"
                class:active={$settings.readingDirection === 'ltr'}
                on:click={() => settings.update((s) => ({ ...s, readingDirection: 'ltr' }))}
                role="radio"
                aria-checked={$settings.readingDirection === 'ltr'}
              >
                LTR
              </button>
              <button
                class="toggle-option"
                class:active={$settings.readingDirection === 'rtl'}
                on:click={() => settings.update((s) => ({ ...s, readingDirection: 'rtl' }))}
                role="radio"
                aria-checked={$settings.readingDirection === 'rtl'}
              >
                RTL
              </button>
            </div>
          </div>
        </div>

        <div class="setting-row">
          <div class="setting-info">
            <span class="setting-label">Page Fit Mode</span>
            <p class="setting-desc">How pages fit in the viewer</p>
          </div>
          <div class="setting-control">
            <select
              class="setting-select"
              value={$settings.pageFitMode}
              on:change={(e) => settings.update((s) => ({ ...s, pageFitMode: e.target.value }))}
            >
              <option value="fit-width">Fit Width</option>
              <option value="fit-height">Fit Height</option>
              <option value="original">Original Size</option>
            </select>
          </div>
        </div>

        <div class="setting-row">
          <div class="setting-info">
            <span class="setting-label">Show Page Numbers</span>
            <p class="setting-desc">Display page number overlay during reading</p>
          </div>
          <div class="setting-control">
            <label class="switch">
              <input
                type="checkbox"
                checked={$settings.showPageNumbers}
                on:change={(e) => settings.update((s) => ({ ...s, showPageNumbers: e.target.checked }))}
              />
              <span class="switch-slider"></span>
            </label>
          </div>
        </div>

        <div class="setting-row">
          <div class="setting-info">
            <span class="setting-label">Double Page Mode</span>
            <p class="setting-desc">Display two pages side by side</p>
          </div>
          <div class="setting-control">
            <label class="switch">
              <input
                type="checkbox"
                checked={$settings.doublePageMode}
                on:change={(e) => settings.update((s) => ({ ...s, doublePageMode: e.target.checked }))}
              />
              <span class="switch-slider"></span>
            </label>
          </div>
        </div>
      </section>

      <!-- ============ Library ============ -->
      <section class="settings-section">
        <h2 class="section-title">Library</h2>

        <div class="setting-row">
          <div class="setting-info">
            <span class="setting-label">Manga Root Directory</span>
            <p class="setting-desc">Default folder for manga scanning</p>
          </div>
          <div class="setting-control">
            <div class="path-input-group">
              <input
                type="text"
                class="setting-input path-input"
                value={$settings.mangaRootPath}
                on:change={(e) => settings.update((s) => ({ ...s, mangaRootPath: e.target.value }))}
                placeholder="No directory set"
                readonly
              />
              <button class="btn btn-secondary" on:click={browseMangaRoot}>
                Browse
              </button>
            </div>
          </div>
        </div>
      </section>

      <!-- ============ Text-to-Speech ============ -->
      <section class="settings-section">
        <h2 class="section-title">Text-to-Speech</h2>

        <div class="setting-row">
          <div class="setting-info">
            <span class="setting-label">Enable TTS</span>
            <p class="setting-desc">Generate speech for manga dialogue</p>
          </div>
          <div class="setting-control">
            <label class="switch">
              <input
                type="checkbox"
                checked={$settings.ttsEnabled}
                on:change={(e) => settings.update((s) => ({ ...s, ttsEnabled: e.target.checked }))}
              />
              <span class="switch-slider"></span>
            </label>
          </div>
        </div>

        {#if $settings.ttsEnabled}
          <div class="setting-row">
            <div class="setting-info">
              <label class="setting-label" for="tts-voice">TTS Voice</label>
              <p class="setting-desc">Voice for speech generation (Kokoro)</p>
            </div>
            <div class="setting-control">
              <select
                id="tts-voice"
                class="setting-select"
                value={$settings.ttsVoice}
                on:change={(e) => settings.update((s) => ({ ...s, ttsVoice: e.target.value }))}
              >
                <option value="af_heart">af_heart (Female, Heart)</option>
                <option value="af_bella">af_bella (Female, Bella)</option>
                <option value="af_nicole">af_nicole (Female, Nicole)</option>
                <option value="af_sarah">af_sarah (Female, Sarah)</option>
                <option value="am_adam">am_adam (Male, Adam)</option>
                <option value="am_michael">am_michael (Male, Michael)</option>
              </select>
            </div>
          </div>

          <div class="setting-row">
            <div class="setting-info">
              <label class="setting-label" for="tts-speed">TTS Speed</label>
              <p class="setting-desc">Speech rate multiplier</p>
            </div>
            <div class="setting-control">
              <div class="slider-group">
                <input
                  id="tts-speed"
                  type="range"
                  min="0.5"
                  max="2.0"
                  step="0.1"
                  value={$settings.ttsSpeed}
                  on:input={(e) => settings.update((s) => ({ ...s, ttsSpeed: parseFloat(e.target.value) }))}
                  class="setting-slider"
                />
                <span class="slider-value">{$settings.ttsSpeed}x</span>
              </div>
            </div>
          </div>

          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-label">Vision Backend</span>
              <p class="setting-desc">AI provider for manga panel analysis</p>
            </div>
            <div class="setting-control">
              <select
                class="setting-select"
                value={$settings.visionBackend}
                on:change={(e) => settings.update((s) => ({ ...s, visionBackend: e.target.value }))}
              >
                <option value="gemini">Gemini</option>
                <option value="openai">OpenAI</option>
              </select>
            </div>
          </div>

          <div class="setting-row">
            <div class="setting-info">
              <label class="setting-label" for="gemini-key">Gemini API Key</label>
              <p class="setting-desc">Required for Gemini vision backend</p>
            </div>
            <div class="setting-control">
              <input
                id="gemini-key"
                type="password"
                class="setting-input"
                value={$settings.geminiApiKey}
                on:input={(e) => settings.update((s) => ({ ...s, geminiApiKey: e.target.value }))}
                placeholder="Enter your Gemini API key"
              />
            </div>
          </div>

          <div class="setting-row">
            <div class="setting-info">
              <label class="setting-label" for="openai-key">OpenAI API Key</label>
              <p class="setting-desc">Required for OpenAI vision backend</p>
            </div>
            <div class="setting-control">
              <input
                id="openai-key"
                type="password"
                class="setting-input"
                value={$settings.openaiApiKey}
                on:input={(e) => settings.update((s) => ({ ...s, openaiApiKey: e.target.value }))}
                placeholder="Enter your OpenAI API key"
              />
            </div>
          </div>

          <div class="setting-row">
            <div class="setting-info">
              <label class="setting-label" for="elevenlabs-key">ElevenLabs API Key</label>
              <p class="setting-desc">Required for ElevenLabs TTS</p>
            </div>
            <div class="setting-control">
              <input
                id="elevenlabs-key"
                type="password"
                class="setting-input"
                value={$settings.elevenlabsApiKey}
                on:input={(e) => settings.update((s) => ({ ...s, elevenlabsApiKey: e.target.value }))}
                placeholder="Enter your ElevenLabs API key"
              />
            </div>
          </div>
        {/if}
      </section>

      <!-- ============ Actions ============ -->
      <section class="settings-section actions-section">
        <div class="action-buttons">
          <button class="btn btn-primary" on:click={handleSave} disabled={isSaving}>
            {isSaving ? 'Saving...' : 'Save Settings'}
          </button>
          <button class="btn btn-danger" on:click={handleReset}>
            Reset to Defaults
          </button>
        </div>
      </section>
    </div>
  {/if}
</div>

<style>
  .settings-page {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .page-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-4) var(--space-6);
    border-bottom: 1px solid var(--color-border);
  }

  .page-title {
    font-size: var(--text-2xl);
    font-weight: 700;
  }

  .settings-content {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-6);
    max-width: 800px;
  }

  .settings-section {
    margin-bottom: var(--space-8);
  }

  .section-title {
    font-size: var(--text-lg);
    font-weight: 600;
    color: var(--color-accent);
    margin-bottom: var(--space-4);
    padding-bottom: var(--space-2);
    border-bottom: 1px solid var(--color-border);
  }

  .setting-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3) 0;
    gap: var(--space-6);
  }

  .setting-row + .setting-row {
    border-top: 1px solid var(--color-border-light);
  }

  .setting-info {
    flex: 1;
    min-width: 0;
  }

  .setting-label {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--color-text-primary);
  }

  .setting-desc {
    font-size: var(--text-xs);
    color: var(--color-text-tertiary);
    margin-top: 2px;
  }

  .setting-control {
    flex-shrink: 0;
    display: flex;
    align-items: center;
  }

  /* Toggle group */
  .toggle-group {
    display: flex;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    overflow: hidden;
  }

  .toggle-option {
    padding: var(--space-1) var(--space-3);
    border: none;
    background: var(--color-bg-tertiary);
    color: var(--color-text-secondary);
    font-family: inherit;
    font-size: var(--text-sm);
    font-weight: 500;
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .toggle-option:hover {
    background: var(--color-bg-hover);
  }

  .toggle-option.active {
    background: var(--color-accent);
    color: white;
  }

  /* Select */
  .setting-select {
    padding: var(--space-1) var(--space-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg-tertiary);
    color: var(--color-text-primary);
    font-family: inherit;
    font-size: var(--text-sm);
    min-width: 140px;
  }

  .setting-select:focus {
    outline: none;
    border-color: var(--color-accent);
  }

  /* Input */
  .setting-input {
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg-tertiary);
    color: var(--color-text-primary);
    font-family: inherit;
    font-size: var(--text-sm);
    width: 100%;
    min-width: 200px;
  }

  .setting-input:focus {
    outline: none;
    border-color: var(--color-accent);
  }

  .path-input-group {
    display: flex;
    gap: var(--space-2);
    align-items: center;
  }

  .path-input {
    flex: 1;
    min-width: 240px;
    cursor: pointer;
  }

  /* Switch */
  .switch {
    position: relative;
    display: inline-block;
    width: 44px;
    height: 24px;
    cursor: pointer;
  }

  .switch input {
    opacity: 0;
    width: 0;
    height: 0;
  }

  .switch-slider {
    position: absolute;
    inset: 0;
    background: var(--color-bg-tertiary);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-full);
    transition: all var(--transition-fast);
  }

  .switch-slider::before {
    content: '';
    position: absolute;
    top: 2px;
    left: 2px;
    width: 18px;
    height: 18px;
    background: var(--color-text-secondary);
    border-radius: 50%;
    transition: all var(--transition-fast);
  }

  .switch input:checked + .switch-slider {
    background: var(--color-accent);
    border-color: var(--color-accent);
  }

  .switch input:checked + .switch-slider::before {
    transform: translateX(20px);
    background: white;
  }

  .switch input:focus-visible + .switch-slider {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }

  /* Slider */
  .slider-group {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .setting-slider {
    width: 120px;
    accent-color: var(--color-accent);
  }

  .slider-value {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--color-text-primary);
    min-width: 36px;
  }

  /* Buttons */
  .btn {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4);
    border: none;
    border-radius: var(--radius-md);
    font-family: inherit;
    font-size: var(--text-sm);
    font-weight: 600;
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-primary {
    background: var(--color-accent);
    color: white;
  }

  .btn-primary:hover:not(:disabled) {
    background: var(--color-accent-hover);
  }

  .btn-secondary {
    background: var(--color-bg-tertiary);
    color: var(--color-text-primary);
    border: 1px solid var(--color-border);
  }

  .btn-secondary:hover {
    background: var(--color-bg-hover);
  }

  .btn-danger {
    background: transparent;
    color: var(--color-danger);
    border: 1px solid var(--color-danger);
  }

  .btn-danger:hover {
    background: var(--color-danger);
    color: white;
  }

  .actions-section {
    padding-top: var(--space-4);
  }

  .action-buttons {
    display: flex;
    gap: var(--space-3);
  }

  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-4);
    padding: var(--space-12);
    color: var(--color-text-secondary);
  }

  .spinner {
    width: 40px;
    height: 40px;
    border: 3px solid var(--color-border);
    border-top-color: var(--color-accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
