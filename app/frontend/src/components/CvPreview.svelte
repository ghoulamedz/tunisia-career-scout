<script lang="ts">
  import { onDestroy } from 'svelte';
  import * as pdfjs from 'pdfjs-dist';
  import {
    ChevronLeft,
    ChevronRight,
    ZoomIn,
    ZoomOut,
    RotateCcw,
    Download,
    Printer,
    Loader2,
    AlertCircle,
    FileText,
  } from '@lucide/svelte';

  // Configure PDF.js worker
  pdfjs.GlobalWorkerOptions.workerSrc = new URL('pdfjs-dist/build/pdf.worker.min.mjs', import.meta.url).toString();

  interface Props {
    pdfData: ArrayBuffer | null;
    loading?: boolean;
    error?: string | null;
  }

  let { pdfData, loading = false, error = null }: Props = $props();

  let pdfDoc = $state<any | null>(null);
  let currentPage = $state(1);
  let totalPages = $state(0);
  let scale = $state(1.0);
  let canvasRef = $state<HTMLCanvasElement | null>(null);
  let localLoading = $state(false);
  let localError = $state<string | null>(null);

  // Debounced render function
  let renderTimeout: ReturnType<typeof setTimeout> | null = null;

  async function loadPdf(data: ArrayBuffer) {
    localLoading = true;
    localError = null;
    try {
      pdfDoc = await pdfjs.getDocument({ data }).promise;
      totalPages = pdfDoc.numPages;
      currentPage = 1;
      scale = 1.0;
      await renderPage(currentPage);
    } catch (e) {
      console.error('PDF load error:', e);
      localError = 'Failed to load PDF preview';
    } finally {
      localLoading = false;
    }
  }

  async function renderPage(pageNum: number) {
    if (!pdfDoc || !canvasRef) return;
    
    try {
      const page = await pdfDoc.getPage(pageNum);
      const viewport = page.getViewport({ scale });
      
      const context = canvasRef.getContext('2d');
      if (!context) return;
      
      canvasRef.height = viewport.height;
      canvasRef.width = viewport.width;
      
      const renderContext = {
        canvasContext: context,
        viewport,
      };
      
      await page.render(renderContext).promise;
    } catch (e) {
      console.error('Page render error:', e);
      localError = 'Failed to render page';
    }
  }

  function debouncedRender(pageNum: number) {
    if (renderTimeout) clearTimeout(renderTimeout);
    renderTimeout = setTimeout(() => renderPage(pageNum), 50);
  }

  function goToPage(pageNum: number) {
    if (pageNum < 1 || pageNum > totalPages) return;
    currentPage = pageNum;
    debouncedRender(pageNum);
  }

  function nextPage() {
    goToPage(currentPage + 1);
  }

  function prevPage() {
    goToPage(currentPage - 1);
  }

  function zoomIn() {
    scale = Math.min(scale + 0.25, 3.0);
    debouncedRender(currentPage);
  }

  function zoomOut() {
    scale = Math.max(scale - 0.25, 0.5);
    debouncedRender(currentPage);
  }

  function resetZoom() {
    scale = 1.0;
    debouncedRender(currentPage);
  }

  function downloadPdf() {
    if (!pdfData) return;
    const blob = new Blob([pdfData], { type: 'application/pdf' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'cv-preview.pdf';
    a.click();
    URL.revokeObjectURL(url);
  }

  function printPdf() {
    if (!pdfData) return;
    const blob = new Blob([pdfData], { type: 'application/pdf' });
    const url = URL.createObjectURL(blob);
    const printWindow = window.open(url, '_blank');
    if (printWindow) {
      printWindow.onload = () => {
        printWindow.print();
        setTimeout(() => URL.revokeObjectURL(url), 1000);
      };
    }
  }

  // Watch for pdfData changes
  $effect(() => {
    if (pdfData) {
      loadPdf(pdfData);
    } else {
      pdfDoc = null;
      totalPages = 0;
      currentPage = 1;
    }
  });

  // Sync props to local state
  $effect(() => {
    localLoading = loading;
  });

  $effect(() => {
    localError = error;
  });

  onDestroy(() => {
    if (renderTimeout) clearTimeout(renderTimeout);
    if (pdfDoc) {
      pdfDoc.destroy();
    }
  });
</script>

<div class="flex flex-col h-full">
  <!-- Toolbar -->
  <div class="flex items-center gap-2 p-3 border-b border-secondary-200 dark:border-secondary-700 bg-secondary-50 dark:bg-secondary-900/50 rounded-t-lg">
    <div class="flex items-center gap-1 border-r border-secondary-200 dark:border-secondary-700 pr-3 mr-3">
      <button
        onclick={prevPage}
        disabled={currentPage <= 1 || localLoading}
        class="p-2 rounded-lg hover:bg-secondary-200 dark:hover:bg-secondary-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        aria-label="Previous page"
        title="Previous page"
      >
        <ChevronLeft class="h-5 w-5" />
      </button>
      <button
        onclick={nextPage}
        disabled={currentPage >= totalPages || localLoading}
        class="p-2 rounded-lg hover:bg-secondary-200 dark:hover:bg-secondary-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        aria-label="Next page"
        title="Next page"
      >
        <ChevronRight class="h-5 w-5" />
      </button>
    </div>

    <div class="flex items-center gap-2 px-3 py-1 bg-secondary-100 dark:bg-secondary-800 rounded-lg">
      <span class="text-sm font-mono text-secondary-700 dark:text-secondary-300">
        Page {currentPage} of {totalPages}
      </span>
    </div>

    <div class="flex-1"></div>

    <div class="flex items-center gap-1 border-r border-secondary-200 dark:border-secondary-700 pl-3 ml-3">
      <button
        onclick={zoomOut}
        disabled={scale <= 0.5 || localLoading}
        class="p-2 rounded-lg hover:bg-secondary-200 dark:hover:bg-secondary-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        aria-label="Zoom out"
        title="Zoom out"
      >
        <ZoomOut class="h-5 w-5" />
      </button>
      <span class="px-2 text-sm font-mono text-secondary-700 dark:text-secondary-300 min-w-[4rem] text-center">
        {Math.round(scale * 100)}%
      </span>
      <button
        onclick={zoomIn}
        disabled={scale >= 3.0 || localLoading}
        class="p-2 rounded-lg hover:bg-secondary-200 dark:hover:bg-secondary-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        aria-label="Zoom in"
        title="Zoom in"
      >
        <ZoomIn class="h-5 w-5" />
      </button>
      <button
        onclick={resetZoom}
        disabled={scale === 1.0 || localLoading}
        class="p-2 rounded-lg hover:bg-secondary-200 dark:hover:bg-secondary-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        aria-label="Reset zoom"
        title="Reset zoom (100%)"
      >
        <RotateCcw class="h-5 w-5" />
      </button>
    </div>

    <div class="flex items-center gap-1">
      <button
        onclick={downloadPdf}
        disabled={!pdfData || localLoading}
        class="flex items-center gap-1 px-3 py-1.5 text-sm font-medium text-secondary-700 dark:text-secondary-300 bg-secondary-100 dark:bg-secondary-800 rounded-lg hover:bg-secondary-200 dark:hover:bg-secondary-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        aria-label="Download PDF"
        title="Download PDF"
      >
        <Download class="h-4 w-4" />
        <span class="hidden sm:inline">Download</span>
      </button>
      <button
        onclick={printPdf}
        disabled={!pdfData || localLoading}
        class="flex items-center gap-1 px-3 py-1.5 text-sm font-medium text-secondary-700 dark:text-secondary-300 bg-secondary-100 dark:bg-secondary-800 rounded-lg hover:bg-secondary-200 dark:hover:bg-secondary-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        aria-label="Print PDF"
        title="Print PDF"
      >
        <Printer class="h-4 w-4" />
        <span class="hidden sm:inline">Print</span>
      </button>
    </div>
  </div>

  <!-- PDF Canvas Area -->
  <div class="flex-1 overflow-auto bg-secondary-100 dark:bg-secondary-900 relative rounded-b-lg" style="min-height: 400px;">
    {#if localLoading && !pdfDoc}
      <div class="flex flex-col items-center justify-center h-full p-8">
        <Loader2 class="h-10 w-10 animate-spin text-primary-500" />
        <p class="mt-3 text-secondary-600 dark:text-secondary-400">Generating preview...</p>
      </div>
    {:else if localError}
      <div class="flex flex-col items-center justify-center h-full p-8 text-center">
        <AlertCircle class="h-12 w-12 text-red-500 mb-3" />
        <p class="text-red-600 dark:text-red-400 font-medium mb-1">Preview Error</p>
        <p class="text-secondary-600 dark:text-secondary-400 text-sm max-w-md">{localError}</p>
        <button
          onclick={() => {
            localError = null;
            if (pdfData) loadPdf(pdfData);
          }}
          class="mt-4 px-4 py-2 text-sm font-medium text-white bg-primary-600 rounded-lg hover:bg-primary-700 transition-colors"
        >
          Retry
        </button>
      </div>
    {:else if pdfDoc}
      <div class="flex justify-center p-4">
        <div class="shadow-lg bg-white dark:bg-secondary-900" style="max-width: 100%;">
          <canvas bind:this={canvasRef} class="block"></canvas>
        </div>
      </div>
    {:else}
      <div class="flex flex-col items-center justify-center h-full p-8 text-center">
        <FileText class="h-12 w-12 text-secondary-300 dark:text-secondary-600 mb-3" />
        <p class="text-secondary-600 dark:text-secondary-400">No PDF preview available</p>
        <p class="text-sm text-secondary-500 dark:text-secondary-500 mt-1">Generate a preview to see your CV</p>
      </div>
    {/if}
  </div>
</div>