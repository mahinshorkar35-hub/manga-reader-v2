<script>
  import { createEventDispatcher } from 'svelte';

  export let series = {
    id: '',
    title: '',
    author: '',
    coverPath: '',
    totalChapters: 0,
    genre: [],
    status: 'Plan to Read',
  };

  const dispatch = createEventDispatcher();

  const statusColors = {
    Reading: 'var(--color-accent)',
    Completed: 'var(--color-success)',
    'Plan to Read': 'var(--color-info)',
    Dropped: 'var(--color-danger)',
  };

  function getStatusColor(status) {
    return statusColors[status] || 'var(--color-text-tertiary)';
  }

  function handleClick() {
    dispatch('select', { series });
  }

  function handleContextMenu(e) {
    e.preventDefault();
    dispatch('contextmenu', { series, x: e.clientX, y: e.clientY });
  }
</script>

<button
  class="manga-card"
  on:click={handleClick}
  on:contextmenu={handleContextMenu}
  title={series.title}
>
  <div class="cover">
    {#if series.coverPath}
      <img src={series.coverPath} alt={series.title} loading="lazy" />
    {:else}
      <div class="cover-placeholder">
        <span class="cover-initials">
          {series.title.slice(0, 2).toUpperCase()}
        </span>
      </div>
    {/if}
    <div class="status-badge" style="background: {getStatusColor(series.status)}">
      {series.status}
    </div>
  </div>

  <div class="info">
    <h3 class="title truncate">{series.title}</h3>
    <p class="author truncate">{series.author}</p>
    <div class="meta">
      <span class="chapters">{series.totalChapters} ch.</span>
      {#if series.genre.length > 0}
        <span class="genre-badge">{series.genre[0]}</span>
      {/if}
    </div>
  </div>
</button>

<style>
  .manga-card {
    display: flex;
    flex-direction: column;
    background: var(--color-bg-secondary);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    overflow: hidden;
    cursor: pointer;
    transition: transform var(--transition-fast), box-shadow var(--transition-fast),
      border-color var(--transition-fast);
    text-align: left;
    font-family: inherit;
    color: inherit;
    padding: 0;
    width: 100%;
  }

  .manga-card:hover {
    transform: translateY(-2px);
    box-shadow: var(--shadow-lg);
    border-color: var(--color-accent);
  }

  .manga-card:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }

  .cover {
    position: relative;
    width: 100%;
    aspect-ratio: 3 / 4;
    overflow: hidden;
    background: var(--color-bg-tertiary);
  }

  .cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    transition: transform var(--transition-slow);
  }

  .manga-card:hover .cover img {
    transform: scale(1.05);
  }

  .cover-placeholder {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(135deg, var(--color-bg-tertiary), var(--color-bg-elevated));
  }

  .cover-initials {
    font-size: var(--text-3xl);
    font-weight: 700;
    color: var(--color-accent-light);
    user-select: none;
  }

  .status-badge {
    position: absolute;
    top: var(--space-2);
    right: var(--space-2);
    padding: 2px var(--space-2);
    border-radius: var(--radius-full);
    font-size: var(--text-xs);
    font-weight: 600;
    color: white;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .info {
    padding: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .title {
    font-size: var(--text-sm);
    font-weight: 600;
    line-height: 1.3;
    color: var(--color-text-primary);
  }

  .author {
    font-size: var(--text-xs);
    color: var(--color-text-secondary);
  }

  .meta {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-top: var(--space-1);
  }

  .chapters {
    font-size: var(--text-xs);
    color: var(--color-text-tertiary);
    font-weight: 500;
  }

  .genre-badge {
    font-size: 0.65rem;
    padding: 1px var(--space-2);
    border-radius: var(--radius-full);
    background: var(--color-accent);
    color: white;
    font-weight: 500;
  }
</style>
