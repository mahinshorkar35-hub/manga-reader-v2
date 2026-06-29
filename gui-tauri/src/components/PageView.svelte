<script>
  /**
   * PageView — GPU-accelerated manga page viewer using <canvas>.
   *
   * Images are loaded from the Rust engine via IPC and rendered
   * onto a 2D canvas context with smooth transitions.
   */
  import { onMount, afterUpdate, createEventDispatcher } from 'svelte';
  import { get } from 'svelte/store';
  import { invoke } from '@tauri-apps/api/core';
  import {
    currentChapter,
    currentPageIndex,
    pageFitMode,
    doublePageMode,
    readingDirection,
    isLoadingPage,
    pageImageData,
    totalPages,
    showPageNumbers,
  } from '../lib/stores/index.js';

  const dispatch = createEventDispatcher();

  export let seriesId = '';
  export let chapterId = '';

  let canvasEl;
  let ctx = null;
  let canvasWidth = 0;
  let canvasHeight = 0;
  let containerEl;
  let img = null;

  $: if (canvasEl && ctx === null) {
    ctx = canvasEl.getContext('2d', { alpha: false, desynchronized: true });
  }

  // Handle resize
  function handleResize() {
    if (!containerEl) return;
    const rect = containerEl.getBoundingClientRect();
    canvasWidth = rect.width;
    canvasHeight = rect.height;
    if (canvasEl) {
      canvasEl.width = canvasWidth;
      canvasEl.height = canvasHeight;
    }
    renderImage();
  }

  // Load page image from Rust engine
  async function loadPageImage(pageIndex) {
    if (!seriesId || !chapterId) return;

    isLoadingPage.set(true);
    try {
      const data = await invoke('get_page_image', {
        seriesId,
        chapterId,
        pageIndex,
      });
      pageImageData.set(data);
      return data;
    } catch (err) {
      console.error('Failed to load page:', err);
      return null;
    } finally {
      isLoadingPage.set(false);
    }
  }

  // Render the loaded image onto the canvas
  function renderImage() {
    if (!ctx || !img || canvasWidth === 0 || canvasHeight === 0) return;

    ctx.clearRect(0, 0, canvasWidth, canvasHeight);

    // Fill background
    ctx.fillStyle = '#1a1a2e';
    ctx.fillRect(0, 0, canvasWidth, canvasHeight);

    let fitMode = get(pageFitMode);

    const imgAspect = img.width / img.height;
    const canvasAspect = canvasWidth / canvasHeight;

    let drawW, drawH, drawX, drawY;

    if (fitMode === 'fit-width') {
      drawW = canvasWidth;
      drawH = canvasWidth / imgAspect;
      drawX = 0;
      drawY = (canvasHeight - drawH) / 2;
    } else if (fitMode === 'fit-height') {
      drawH = canvasHeight;
      drawW = canvasHeight * imgAspect;
      drawX = (canvasWidth - drawW) / 2;
      drawY = 0;
    } else {
      // original size
      drawW = img.width;
      drawH = img.height;
      drawX = (canvasWidth - drawW) / 2;
      drawY = (canvasHeight - drawH) / 2;
    }

    // Use image smoothing for high-quality scaling
    ctx.imageSmoothingEnabled = true;
    ctx.imageSmoothingQuality = 'high';

    ctx.drawImage(img, drawX, drawY, drawW, drawH);

    // Draw page number overlay
    const showPageNums = get(showPageNumbers);
    if (showPageNums) {
      const pg = $currentPageIndex + 1;
      const total = $totalPages;
      ctx.fillStyle = 'rgba(0, 0, 0, 0.5)';
      ctx.roundRect?.(canvasWidth - 90, canvasHeight - 36, 80, 28, 6);
      ctx.fill();
      ctx.fillStyle = '#ffffff';
      ctx.font = '12px system-ui, sans-serif';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText(`${pg} / ${total}`, canvasWidth - 50, canvasHeight - 22);
    }
  }

  // Load and render when page changes
  $: if ($currentPageIndex !== undefined && seriesId && chapterId) {
    loadAndRender($currentPageIndex);
  }

  async function loadAndRender(pageIndex) {
    const data = await loadPageImage(pageIndex);
    if (!data) return;

    const existingImg = new Image();
    existingImg.onload = () => {
      img = existingImg;
      renderImage();
    };
    existingImg.src = data;
  }

  // Keyboard navigation
  function handleKeydown(e) {
    const dir = get(readingDirection);

    const isRtl = dir === 'rtl';
    const nextKeys = isRtl ? ['ArrowLeft'] : ['ArrowRight'];
    const prevKeys = isRtl ? ['ArrowRight'] : ['ArrowLeft'];

    if (nextKeys.includes(e.key) || e.key === ' ' || e.key === 'PageDown') {
      e.preventDefault();
      dispatch('nextPage');
    } else if (prevKeys.includes(e.key) || e.key === 'PageUp') {
      e.preventDefault();
      dispatch('prevPage');
    } else if (e.key === 'Home') {
      e.preventDefault();
      dispatch('firstPage');
    } else if (e.key === 'End') {
      e.preventDefault();
      dispatch('lastPage');
    }
  }

  onMount(() => {
    handleResize();
    window.addEventListener('resize', handleResize);
    window.addEventListener('keydown', handleKeydown);

    return () => {
      window.removeEventListener('resize', handleResize);
      window.removeEventListener('keydown', handleKeydown);
    };
  });

  afterUpdate(() => {
    if (canvasEl && !ctx) {
      ctx = canvasEl.getContext('2d', { alpha: false, desynchronized: true });
    }
  });

  // Click to navigate
  function handleCanvasClick(e) {
    if (!canvasEl) return;
    const rect = canvasEl.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const centerX = rect.width / 2;

    const dir = get(readingDirection);
    const isRtl = dir === 'rtl';

    if (isRtl) {
      if (x < centerX) {
        dispatch('nextPage');
      } else {
        dispatch('prevPage');
      }
    } else {
      if (x > centerX) {
        dispatch('nextPage');
      } else {
        dispatch('prevPage');
      }
    }
  }
</script>

<div
  class="page-view-container"
  bind:this={containerEl}
  role="region"
  aria-label="Manga page viewer"
>
  {#if $isLoadingPage}
    <div class="loading-overlay">
      <div class="spinner"></div>
      <span>Loading page...</span>
    </div>
  {/if}

  <canvas
    bind:this={canvasEl}
    class="page-canvas"
    on:click={handleCanvasClick}
    role="img"
    aria-label="Manga page {($currentPageIndex || 0) + 1} of {$totalPages}"
  ></canvas>
</div>

<style>
  .page-view-container {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: var(--color-bg-primary);
  }

  .page-canvas {
    display: block;
    width: 100%;
    height: 100%;
    cursor: pointer;
  }

  .loading-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    background: var(--color-overlay);
    z-index: 10;
    color: var(--color-text-primary);
    font-size: var(--text-sm);
  }

  .spinner {
    width: 32px;
    height: 32px;
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
