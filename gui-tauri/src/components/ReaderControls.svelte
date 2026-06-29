<script>
  import { createEventDispatcher } from 'svelte';
  import {
    currentPageIndex,
    totalPages,
    readingProgress,
    pageFitMode,
    doublePageMode,
    showPageNumbers,
    readingDirection,
    isLoadingPage,
  } from '../lib/stores/index.js';

  const dispatch = createEventDispatcher();

  function goPrev() {
    dispatch('prevPage');
  }

  function goNext() {
    dispatch('nextPage');
  }

  function goFirst() {
    dispatch('firstPage');
  }

  function goLast() {
    dispatch('lastPage');
  }

  function toggleDoublePage() {
    doublePageMode.update((v) => !v);
  }

  function cycleFitMode() {
    const modes = ['fit-width', 'fit-height', 'original'];
    let current = 'fit-width';
    pageFitMode.subscribe((v) => (current = v))();
    const idx = (modes.indexOf(current) + 1) % modes.length;
    pageFitMode.set(modes[idx]);
  }

  function togglePageNumbers() {
    showPageNumbers.update((v) => !v);
  }

  function toggleDirection() {
    readingDirection.update((v) => (v === 'ltr' ? 'rtl' : 'ltr'));
  }

  function goToPage(e) {
    const val = parseInt(e.target.value, 10);
    if (!isNaN(val) && val >= 1 && val <= $totalPages) {
      currentPageIndex.set(val - 1);
    }
  }
</script>

<div class="reader-controls">
  <!-- Top bar: progress & page jump -->
  <div class="progress-bar-container">
    <div class="progress-track">
      <div class="progress-fill" style="width: {$readingProgress}%"></div>
    </div>
    <span class="progress-text">{$readingProgress}%</span>
  </div>

  <!-- Main controls -->
  <div class="controls-row">
    <div class="nav-group">
      <button class="ctrl-btn" on:click={goFirst} disabled={$currentPageIndex === 0} title="First page">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
          <polyline points="11 17 6 12 11 7" />
          <polyline points="18 17 13 12 18 7" />
        </svg>
      </button>
      <button class="ctrl-btn" on:click={goPrev} disabled={$currentPageIndex === 0} title="Previous page">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
          <polyline points="15 18 9 12 15 6" />
        </svg>
      </button>

      <div class="page-jump">
        <input
          type="number"
          class="page-input"
          min="1"
          max={$totalPages}
          value={$currentPageIndex + 1}
          on:change={goToPage}
          disabled={$isLoadingPage}
          aria-label="Go to page"
        />
        <span class="page-separator">/</span>
        <span class="page-total">{$totalPages}</span>
      </div>

      <button class="ctrl-btn" on:click={goNext} disabled={$currentPageIndex >= $totalPages - 1} title="Next page">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
          <polyline points="9 18 15 12 9 6" />
        </svg>
      </button>
      <button class="ctrl-btn" on:click={goLast} disabled={$currentPageIndex >= $totalPages - 1} title="Last page">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
          <polyline points="13 17 18 12 13 7" />
          <polyline points="6 17 11 12 6 7" />
        </svg>
      </button>
    </div>

    <div class="separator"></div>

    <div class="view-group">
      <button
        class="ctrl-btn"
        on:click={cycleFitMode}
        title="Cycle fit mode: {$pageFitMode === 'fit-width' ? 'Fit Width' : $pageFitMode === 'fit-height' ? 'Fit Height' : 'Original'}"
      >
        {#if $pageFitMode === 'fit-width'}
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
            <path d="M8 3H5a2 2 0 0 0-2 2v3m18 0V5a2 2 0 0 0-2-2h-3m0 18h3a2 2 0 0 0 2-2v-3M3 16v3a2 2 0 0 0 2 2h3" />
          </svg>
        {:else if $pageFitMode === 'fit-height'}
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
            <path d="M8 3H5a2 2 0 0 0-2 2v3m18 0V5a2 2 0 0 0-2-2h-3m0 18h3a2 2 0 0 0 2-2v-3M3 16v3a2 2 0 0 0 2 2h3" />
          </svg>
        {:else}
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
            <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
          </svg>
        {/if}
      </button>

      <button class="ctrl-btn" on:click={toggleDoublePage} class:active={$doublePageMode} title="Double page mode">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
          <rect x="2" y="3" width="9" height="18" rx="1" />
          <rect x="13" y="3" width="9" height="18" rx="1" />
        </svg>
      </button>

      <button class="ctrl-btn" on:click={toggleDirection} title="Reading direction: {$readingDirection === 'ltr' ? 'Left to Right' : 'Right to Left'}">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
          {#if $readingDirection === 'ltr'}
            <line x1="3" y1="12" x2="21" y2="12" />
            <polyline points="17 8 21 12 17 16" />
          {:else}
            <line x1="21" y1="12" x2="3" y2="12" />
            <polyline points="7 8 3 12 7 16" />
          {/if}
        </svg>
      </button>

      <button class="ctrl-btn" on:click={togglePageNumbers} class:active={$showPageNumbers} title="Toggle page numbers">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
          <rect x="4" y="4" width="16" height="16" rx="2" />
          <line x1="9" y1="9" x2="15" y2="9" />
          <line x1="9" y1="13" x2="15" y2="13" />
          <line x1="9" y1="17" x2="12" y2="17" />
        </svg>
      </button>

      <button class="ctrl-btn close-btn" on:click={() => dispatch('close')} title="Close reader">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
          <line x1="18" y1="6" x2="6" y2="18" />
          <line x1="6" y1="6" x2="18" y2="18" />
        </svg>
      </button>
    </div>
  </div>
</div>

<style>
  .reader-controls {
    display: flex;
    flex-direction: column;
    gap: 0;
    background: var(--color-bg-secondary);
    border-top: 1px solid var(--color-border);
    padding: var(--space-2) var(--space-4);
    user-select: none;
  }

  /* Progress bar */
  .progress-bar-container {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin-bottom: var(--space-2);
  }

  .progress-track {
    flex: 1;
    height: 4px;
    background: var(--color-bg-tertiary);
    border-radius: var(--radius-full);
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: var(--color-accent);
    border-radius: var(--radius-full);
    transition: width var(--transition-normal);
  }

  .progress-text {
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--color-text-secondary);
    min-width: 36px;
    text-align: right;
  }

  /* Control row */
  .controls-row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .nav-group,
  .view-group {
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }

  .separator {
    width: 1px;
    height: 24px;
    background: var(--color-border);
    flex-shrink: 0;
  }

  .ctrl-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--color-text-secondary);
    cursor: pointer;
    transition: all var(--transition-fast);
    padding: 0;
  }

  .ctrl-btn:hover:not(:disabled) {
    background: var(--color-bg-hover);
    color: var(--color-text-primary);
  }

  .ctrl-btn:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .ctrl-btn:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }

  .ctrl-btn.active {
    background: var(--color-accent);
    color: white;
    border-color: var(--color-accent);
  }

  .close-btn {
    margin-left: auto;
  }

  .close-btn:hover {
    color: var(--color-danger) !important;
  }

  /* Page jump */
  .page-jump {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    font-size: var(--text-sm);
    color: var(--color-text-secondary);
  }

  .page-input {
    width: 48px;
    padding: var(--space-1);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-bg-tertiary);
    color: var(--color-text-primary);
    font-family: inherit;
    font-size: var(--text-sm);
    text-align: center;
    -moz-appearance: textfield;
  }

  .page-input::-webkit-inner-spin-button,
  .page-input::-webkit-outer-spin-button {
    -webkit-appearance: none;
    margin: 0;
  }

  .page-input:focus {
    outline: none;
    border-color: var(--color-accent);
  }

  .page-separator {
    color: var(--color-text-tertiary);
  }

  .page-total {
    font-weight: 600;
    color: var(--color-text-primary);
  }
</style>
