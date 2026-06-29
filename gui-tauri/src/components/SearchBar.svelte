<script>
  import { createEventDispatcher } from 'svelte';
  import { searchQuery } from '../lib/stores/index.js';

  const dispatch = createEventDispatcher();
  let inputEl;

  function handleInput() {
    searchQuery.set(inputEl.value);
    dispatch('search', { query: inputEl.value });
  }

  function handleClear() {
    inputEl.value = '';
    searchQuery.set('');
    dispatch('clear');
    inputEl.focus();
  }
</script>

<div class="search-bar" role="search">
  <svg
    class="search-icon"
    width="16"
    height="16"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="2.5"
    stroke-linecap="round"
    stroke-linejoin="round"
  >
    <circle cx="11" cy="11" r="8" />
    <line x1="21" y1="21" x2="16.65" y2="16.65" />
  </svg>

  <input
    bind:this={inputEl}
    type="search"
    class="search-input"
    placeholder="Search manga by title, author, or genre..."
    on:input={handleInput}
    aria-label="Search manga series"
  />

  {#if inputEl?.value}
    <button class="clear-btn" on:click={handleClear} aria-label="Clear search">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
        <line x1="18" y1="6" x2="6" y2="18" />
        <line x1="6" y1="6" x2="18" y2="18" />
      </svg>
    </button>
  {/if}
</div>

<style>
  .search-bar {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: var(--color-bg-secondary);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    transition: border-color var(--transition-fast), box-shadow var(--transition-fast);
  }

  .search-bar:focus-within {
    border-color: var(--color-accent);
    box-shadow: 0 0 0 3px rgba(124, 58, 237, 0.15);
  }

  .search-icon {
    color: var(--color-text-tertiary);
    flex-shrink: 0;
  }

  .search-input {
    flex: 1;
    border: none;
    background: none;
    outline: none;
    font-family: inherit;
    font-size: var(--text-sm);
    color: var(--color-text-primary);
  }

  .search-input::placeholder {
    color: var(--color-text-tertiary);
  }

  .clear-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border: none;
    background: none;
    color: var(--color-text-tertiary);
    cursor: pointer;
    border-radius: var(--radius-sm);
    padding: 0;
    transition: color var(--transition-fast), background var(--transition-fast);
  }

  .clear-btn:hover {
    color: var(--color-text-primary);
    background: var(--color-bg-hover);
  }
</style>
