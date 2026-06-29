/**
 * Manga Reader v2 — Svelte Stores
 *
 * Central state management for the application.
 * All data comes from the Rust engine via Tauri IPC.
 */

import { writable, derived } from 'svelte/store';

// ---------------------------------------------------------------
// Theme store
// ---------------------------------------------------------------
function createThemeStore() {
  const { subscribe, set, update } = writable('dark');

  return {
    subscribe,
    set(value) {
      if (typeof document !== 'undefined') {
        document.documentElement.setAttribute('data-theme', value);
      }
      set(value);
    },
    toggle() {
      update((current) => {
        const next = current === 'dark' ? 'light' : 'dark';
        if (typeof document !== 'undefined') {
          document.documentElement.setAttribute('data-theme', next);
        }
        return next;
      });
    },
    init(theme) {
      if (typeof document !== 'undefined') {
        document.documentElement.setAttribute('data-theme', theme);
      }
      set(theme);
    },
  };
}

export const theme = createThemeStore();

// ---------------------------------------------------------------
// Navigation store
// ---------------------------------------------------------------
export const currentRoute = writable('library'); // 'library' | 'reader' | 'settings'

// Parameters passed alongside route navigation
export const routeParams = writable({});

// Navigation helper
export function navigateTo(route, params = {}) {
  routeParams.set(params);
  currentRoute.set(route);
}

// ---------------------------------------------------------------
// Library store
// ---------------------------------------------------------------
export const library = writable([]);
export const libraryStats = writable({
  totalSeries: 0,
  totalChapters: 0,
  totalPages: 0,
  readingCount: 0,
  completedCount: 0,
  planToReadCount: 0,
});

export const seriesFilter = writable('all'); // 'all' | 'reading' | 'completed' | 'plan-to-read' | 'dropped'
export const searchQuery = writable('');

// Derived: filtered library based on status filter and search query
export const filteredLibrary = derived(
  [library, seriesFilter, searchQuery],
  ([$library, $seriesFilter, $searchQuery]) => {
    let result = $library;

    // Status filter
    if ($seriesFilter !== 'all') {
      const statusMap = {
        reading: 'Reading',
        completed: 'Completed',
        'plan-to-read': 'Plan to Read',
        dropped: 'Dropped',
      };
      const targetStatus = statusMap[$seriesFilter];
      if (targetStatus) {
        result = result.filter((s) => s.status === targetStatus);
      }
    }

    // Search filter
    if ($searchQuery.trim()) {
      const q = $searchQuery.toLowerCase();
      result = result.filter(
        (s) =>
          s.title.toLowerCase().includes(q) ||
          s.author.toLowerCase().includes(q) ||
          s.genre.some((g) => g.toLowerCase().includes(q)),
      );
    }

    return result;
  },
);

// ---------------------------------------------------------------
// Reader store
// ---------------------------------------------------------------
export const currentSeries = writable(null);
export const currentChapter = writable(null);
export const chapters = writable([]);
export const currentPageIndex = writable(0);
export const pageFitMode = writable('fit-width'); // 'fit-width' | 'fit-height' | 'original'
export const doublePageMode = writable(false);
export const showPageNumbers = writable(true);
export const readingDirection = writable('ltr'); // 'ltr' | 'rtl'
export const isLoadingPage = writable(false);
export const pageImageData = writable('');

// Derived: total pages in current chapter
export const totalPages = derived(currentChapter, ($chapter) => {
  return $chapter?.pageCount ?? 0;
});

// Derived: progress percentage
export const readingProgress = derived(
  [currentPageIndex, totalPages],
  ([$currentPageIndex, $totalPages]) => {
    if ($totalPages === 0) return 0;
    return Math.round((($currentPageIndex + 1) / $totalPages) * 100);
  },
);

// ---------------------------------------------------------------
// Settings store
// ---------------------------------------------------------------
export const settings = writable({
  theme: 'dark',
  readingDirection: 'ltr',
  pageFitMode: 'fit-width',
  showPageNumbers: true,
  doublePageMode: false,
  backgroundColor: '#1a1a2e',
  mangaRootPath: '',
  ttsEnabled: false,
  ttsVoice: 'af_heart',
  ttsSpeed: 1.0,
  visionBackend: 'gemini',
  geminiApiKey: '',
  openaiApiKey: '',
  elevenlabsApiKey: '',
});

// ---------------------------------------------------------------
// UI state
// ---------------------------------------------------------------
export const isSidebarOpen = writable(true);
export const toastMessage = writable(null);

// Toast helper — auto-dismiss after 3 seconds
export function showToast(message, type = 'info', duration = 3000) {
  toastMessage.set({ message, type });
  setTimeout(() => {
    toastMessage.set(null);
  }, duration);
}
