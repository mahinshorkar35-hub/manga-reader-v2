<script>
  /**
   * Library Page — Browse, filter, and search manga series.
   */
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import MangaCard from '../components/MangaCard.svelte';
  import SearchBar from '../components/SearchBar.svelte';
  import ThemeToggle from '../components/ThemeToggle.svelte';
  import {
    library,
    libraryStats,
    seriesFilter,
    searchQuery,
    navigateTo,
    showToast,
  } from '../lib/stores/index.js';

  let isLoading = false;
  let sortBy = 'title'; // 'title' | 'recent' | 'progress'
  let viewMode = 'grid'; // 'grid' | 'list'

  onMount(async () => {
    await loadLibrary();
  });

  async function loadLibrary() {
    isLoading = true;
    try {
      const data = await invoke('get_library');
      library.set(data);

      const stats = await invoke('get_library_stats');
      libraryStats.set({
        totalSeries: stats.total_series,
        totalChapters: stats.total_chapters,
        totalPages: stats.total_pages,
        readingCount: stats.reading_count,
        completedCount: stats.completed_count,
        planToReadCount: stats.plan_to_read_count,
      });
    } catch (err) {
      showToast(`Failed to load library: ${err}`, 'error');
    } finally {
      isLoading = false;
    }
  }

  async function handleScanDirectory() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select Manga Directory',
      });
      if (!selected) return;

      isLoading = true;
      const series = await invoke('scan_manga_directory', { path: selected });
      showToast(`Found ${series.length} series`, 'success');
      await loadLibrary();
    } catch (err) {
      showToast(`Scan failed: ${err}`, 'error');
    } finally {
      isLoading = false;
    }
  }

  function handleSelectSeries(event) {
    const { series } = event.detail;
    navigateTo('reader', { seriesId: series.id, seriesTitle: series.title });
  }

  function setFilter(filter) {
    seriesFilter.set(filter);
  }

  // Derived: sorted & filtered library
  import { derived } from 'svelte/store';
  import { filteredLibrary } from '../lib/stores/index.js';

  const sortedLibrary = derived(
    [filteredLibrary],
    ([$filteredLibrary]) => {
      const sorted = [...$filteredLibrary];
      if (sortBy === 'title') {
        sorted.sort((a, b) => a.title.localeCompare(b.title));
      } else if (sortBy === 'progress') {
        sorted.sort((a, b) => {
          const order = { Reading: 0, Completed: 1, 'Plan to Read': 2, Dropped: 3 };
          return (order[a.status] || 99) - (order[b.status] || 99);
        });
      }
      return sorted;
    },
  );

  const filters = [
    { id: 'all', label: 'All' },
    { id: 'reading', label: 'Reading' },
    { id: 'completed', label: 'Completed' },
    { id: 'plan-to-read', label: 'Plan to Read' },
    { id: 'dropped', label: 'Dropped' },
  ];
</script>

<div class="library-page">
  <!-- Header -->
  <header class="page-header">
    <div class="header-left">
      <h1 class="page-title">Library</h1>
      <span class="series-count">{$library.length} series</span>
    </div>
    <div class="header-right">
      <SearchBar />
      <div class="sort-group">
        <select class="sort-select" bind:value={sortBy} aria-label="Sort by">
          <option value="title">Sort by Title</option>
          <option value="progress">Sort by Progress</option>
        </select>
      </div>
      <button class="btn btn-primary" on:click={handleScanDirectory}>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
          <polyline points="17 8 12 3 7 8" />
          <line x1="12" y1="3" x2="12" y2="15" />
        </svg>
        Add Folder
      </button>
      <ThemeToggle />
    </div>
  </header>

  <!-- Filter tabs -->
  <nav class="filter-tabs" aria-label="Filter by status">
    {#each filters as filter}
      <button
        class="filter-tab"
        class:active={$seriesFilter === filter.id}
        on:click={() => setFilter(filter.id)}
      >
        {filter.label}
        {#if filter.id === 'all'}
          <span class="filter-count">{$library.length}</span>
        {:else if filter.id === 'reading'}
          <span class="filter-count">{$libraryStats.readingCount}</span>
        {:else if filter.id === 'completed'}
          <span class="filter-count">{$libraryStats.completedCount}</span>
        {:else if filter.id === 'plan-to-read'}
          <span class="filter-count">{$libraryStats.planToReadCount}</span>
        {/if}
      </button>
    {/each}
  </nav>

  <!-- Content -->
  <div class="library-content">
    {#if isLoading}
      <div class="loading-state">
        <div class="spinner"></div>
        <p>Loading library...</p>
      </div>
    {:else if $sortedLibrary.length === 0}
      <div class="empty-state">
        {#if $searchQuery}
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <circle cx="11" cy="11" r="8" />
            <line x1="21" y1="21" x2="16.65" y2="16.65" />
          </svg>
          <h2>No results found</h2>
          <p>Try a different search term or clear filters.</p>
        {:else}
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20" />
            <path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z" />
          </svg>
          <h2>Your library is empty</h2>
          <p>Click "Add Folder" to scan your manga collection.</p>
          <button class="btn btn-primary" on:click={handleScanDirectory}>
            Add Manga Folder
          </button>
        {/if}
      </div>
    {:else}
      <div class="manga-grid">
        {#each $sortedLibrary as series (series.id)}
          <MangaCard {series} on:select={handleSelectSeries} />
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .library-page {
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
    gap: var(--space-4);
    flex-wrap: wrap;
  }

  .header-left {
    display: flex;
    align-items: baseline;
    gap: var(--space-3);
  }

  .page-title {
    font-size: var(--text-2xl);
    font-weight: 700;
    color: var(--color-text-primary);
  }

  .series-count {
    font-size: var(--text-sm);
    color: var(--color-text-tertiary);
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    flex-wrap: wrap;
  }

  .sort-select {
    padding: var(--space-1) var(--space-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg-secondary);
    color: var(--color-text-primary);
    font-family: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .sort-select:focus {
    outline: none;
    border-color: var(--color-accent);
  }

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

  .btn-primary {
    background: var(--color-accent);
    color: white;
  }

  .btn-primary:hover {
    background: var(--color-accent-hover);
  }

  /* Filter tabs */
  .filter-tabs {
    display: flex;
    gap: var(--space-1);
    padding: var(--space-2) var(--space-6);
    border-bottom: 1px solid var(--color-border);
    background: var(--color-bg-primary);
  }

  .filter-tab {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border: none;
    background: none;
    color: var(--color-text-secondary);
    font-family: inherit;
    font-size: var(--text-sm);
    font-weight: 500;
    cursor: pointer;
    border-radius: var(--radius-sm);
    transition: all var(--transition-fast);
  }

  .filter-tab:hover {
    color: var(--color-text-primary);
    background: var(--color-bg-hover);
  }

  .filter-tab.active {
    color: var(--color-accent);
    background: rgba(124, 58, 237, 0.1);
  }

  .filter-count {
    font-size: var(--text-xs);
    padding: 1px var(--space-2);
    border-radius: var(--radius-full);
    background: var(--color-bg-tertiary);
  }

  .filter-tab.active .filter-count {
    background: var(--color-accent);
    color: white;
  }

  /* Content area */
  .library-content {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-6);
  }

  .manga-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: var(--space-5);
  }

  /* Loading & Empty states */
  .loading-state,
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-4);
    padding: var(--space-12);
    color: var(--color-text-secondary);
    text-align: center;
  }

  .loading-state svg,
  .empty-state svg {
    color: var(--color-text-tertiary);
  }

  .empty-state h2 {
    font-size: var(--text-xl);
    color: var(--color-text-primary);
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
