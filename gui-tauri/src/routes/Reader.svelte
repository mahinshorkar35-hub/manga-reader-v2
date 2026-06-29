<script>
  /**
   * Reader Page — Full-screen manga reader with canvas-based GPU rendering.
   */
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import PageView from '../components/PageView.svelte';
  import ReaderControls from '../components/ReaderControls.svelte';
  import {
    currentSeries,
    currentChapter,
    chapters,
    currentPageIndex,
    routeParams,
    navigateTo,
    showToast,
  } from '../lib/stores/index.js';

  let chapterListOpen = false;
  let seriesId = '';
  let chapterId = '';

  // Unsubscribe from routeParams to get initial values
  const unsubParams = routeParams.subscribe((params) => {
    if (params.seriesId) {
      seriesId = params.seriesId;
    }
    if (params.chapterId) {
      chapterId = params.chapterId;
    }
  });

  onMount(async () => {
    if (!seriesId) {
      showToast('No series selected', 'error');
      navigateTo('library');
      return;
    }

    try {
      // Load series info
      const seriesData = await invoke('get_series', { seriesId });
      if (seriesData) {
        // Map Rust snake_case fields to camelCase for JS
        currentSeries.set({
          id: seriesData.id,
          title: seriesData.title,
          author: seriesData.author,
          coverPath: seriesData.cover_path,
          totalChapters: seriesData.total_chapters,
          genre: seriesData.genre,
          status: seriesData.status,
        });
      }

      // Load chapters
      const chapterData = await invoke('get_chapters', { seriesId });
      chapters.set(
        chapterData.map((ch) => ({
          id: ch.id,
          seriesId: ch.series_id,
          number: ch.number,
          title: ch.title,
          pageCount: ch.page_count,
          pages: ch.pages.map((p) => ({
            index: p.index,
            path: p.path,
            width: p.width,
            height: p.height,
          })),
        })),
      );

      // Load first chapter if none specified
      if (!chapterId && chapterData.length > 0) {
        chapterId = chapterData[0].id;
      }
      if (chapterId) {
        await loadChapter(chapterId);
      }
    } catch (err) {
      showToast(`Failed to load reader: ${err}`, 'error');
    }
  });

  onDestroy(() => {
    unsubParams();
  });

  async function loadChapter(chId) {
    try {
      const ch = await invoke('get_chapter', { chapterId: chId });
      if (ch) {
        currentChapter.set({
          id: ch.id,
          seriesId: ch.series_id,
          number: ch.number,
          title: ch.title,
          pageCount: ch.page_count,
          pages: ch.pages.map((p) => ({
            index: p.index,
            path: p.path,
            width: p.width,
            height: p.height,
          })),
        });
        chapterId = chId;
        currentPageIndex.set(0);
      }
    } catch (err) {
      showToast(`Failed to load chapter: ${err}`, 'error');
    }
  }

  function handleNextPage() {
    currentPageIndex.update((n) => {
      const max = $currentChapter?.pageCount ?? 1;
      return Math.min(n + 1, max - 1);
    });
  }

  function handlePrevPage() {
    currentPageIndex.update((n) => Math.max(n - 1, 0));
  }

  function handleFirstPage() {
    currentPageIndex.set(0);
  }

  function handleLastPage() {
    const max = $currentChapter?.pageCount ?? 1;
    currentPageIndex.set(max - 1);
  }

  function handleClose() {
    // Mark series as reading
    if (seriesId) {
      invoke('update_series_status', {
        seriesId,
        status: 'Reading',
      }).catch(() => {});
    }
    navigateTo('library');
  }

  function selectChapter(event) {
    const chId = event.detail?.chapterId || event.target?.value;
    if (chId) {
      loadChapter(chId);
      chapterListOpen = false;
    }
  }
</script>

<div class="reader-page">
  <!-- Chapter selector header -->
  <header class="reader-header">
    <div class="reader-header-left">
      <button class="back-btn" on:click={handleClose} title="Back to library">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
          <polyline points="15 18 9 12 15 6" />
        </svg>
      </button>
      <div class="series-info">
        <span class="series-title">{$currentSeries?.title || 'Loading...'}</span>
        {#if $currentChapter}
          <span class="chapter-title">
            Ch. {$currentChapter.number}{$currentChapter.title ? ` — ${$currentChapter.title}` : ''}
          </span>
        {/if}
      </div>
    </div>

    <div class="reader-header-right">
      <div class="chapter-selector">
        <button
          class="chapter-toggle"
          on:click={() => (chapterListOpen = !chapterListOpen)}
          aria-haspopup="listbox"
          aria-expanded={chapterListOpen}
        >
          {#if $currentChapter}
            Chapter {$currentChapter.number}
          {:else}
            Select Chapter
          {/if}
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
            <polyline points="6 9 12 15 18 9" />
          </svg>
        </button>

        {#if chapterListOpen}
          <div class="chapter-dropdown" role="listbox" aria-label="Chapter list">
            {#each $chapters as ch (ch.id)}
              <button
                class="chapter-option"
                class:active={ch.id === chapterId}
                on:click={() => selectChapter({ detail: { chapterId: ch.id } })}
                role="option"
                aria-selected={ch.id === chapterId}
              >
                <span class="ch-number">Ch. {ch.number}</span>
                <span class="ch-title">{ch.title || `Chapter ${ch.number}`}</span>
                <span class="ch-pages">{ch.pageCount} pages</span>
              </button>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  </header>

  <!-- Page viewer -->
  <main class="reader-main">
    {#if $currentChapter}
      <PageView
        {seriesId}
        chapterId={$currentChapter.id}
        on:nextPage={handleNextPage}
        on:prevPage={handlePrevPage}
        on:firstPage={handleFirstPage}
        on:lastPage={handleLastPage}
      />
    {:else}
      <div class="no-chapter">
        <p>No chapter loaded. Select a chapter from the dropdown.</p>
      </div>
    {/if}
  </main>

  <!-- Controls bar -->
  <ReaderControls
    on:nextPage={handleNextPage}
    on:prevPage={handlePrevPage}
    on:firstPage={handleFirstPage}
    on:lastPage={handleLastPage}
    on:close={handleClose}
  />
</div>

<style>
  .reader-page {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--color-bg-primary);
  }

  .reader-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-2) var(--space-4);
    border-bottom: 1px solid var(--color-border);
    background: var(--color-bg-secondary);
    z-index: 10;
  }

  .reader-header-left {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .reader-header-right {
    display: flex;
    align-items: center;
  }

  .back-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border: none;
    background: none;
    color: var(--color-text-secondary);
    cursor: pointer;
    border-radius: var(--radius-sm);
    transition: all var(--transition-fast);
  }

  .back-btn:hover {
    background: var(--color-bg-hover);
    color: var(--color-text-primary);
  }

  .series-info {
    display: flex;
    flex-direction: column;
    gap: 0;
  }

  .series-title {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--color-text-primary);
  }

  .chapter-title {
    font-size: var(--text-xs);
    color: var(--color-text-tertiary);
  }

  /* Chapter selector */
  .chapter-selector {
    position: relative;
  }

  .chapter-toggle {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg-tertiary);
    color: var(--color-text-primary);
    font-family: inherit;
    font-size: var(--text-sm);
    font-weight: 500;
    cursor: pointer;
    transition: border-color var(--transition-fast);
  }

  .chapter-toggle:hover {
    border-color: var(--color-accent);
  }

  .chapter-dropdown {
    position: absolute;
    top: 100%;
    right: 0;
    margin-top: var(--space-1);
    min-width: 280px;
    max-height: 360px;
    overflow-y: auto;
    background: var(--color-bg-elevated);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-lg);
    z-index: 100;
  }

  .chapter-option {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    padding: var(--space-2) var(--space-3);
    border: none;
    background: none;
    color: var(--color-text-primary);
    font-family: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
    text-align: left;
    transition: background var(--transition-fast);
  }

  .chapter-option:hover {
    background: var(--color-bg-hover);
  }

  .chapter-option.active {
    background: rgba(124, 58, 237, 0.15);
    color: var(--color-accent);
  }

  .ch-number {
    font-weight: 600;
    min-width: 56px;
    color: var(--color-accent);
  }

  .ch-title {
    flex: 1;
    color: var(--color-text-secondary);
  }

  .ch-pages {
    font-size: var(--text-xs);
    color: var(--color-text-tertiary);
  }

  /* Main viewer */
  .reader-main {
    flex: 1;
    overflow: hidden;
    position: relative;
  }

  .no-chapter {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--color-text-tertiary);
    font-size: var(--text-lg);
  }
</style>
