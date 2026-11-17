<script lang="ts">
  import { cn } from '$lib/utils';
  import Button from './Button.svelte';
  import { onMount } from 'svelte';

  let { open = false, title, size = 'md', closeOnOverlay = true, ...props } = $props();
  let contentRef: HTMLDivElement;

  const sizes = {
    sm: 'max-w-sm',
    md: 'max-w-lg',
    lg: 'max-w-2xl',
    xl: 'max-w-4xl',
    full: 'max-w-full mx-4',
  };

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') close();
  }

  function close() {
    open = false;
  }

  function handleOverlayClick(e: MouseEvent) {
    if (closeOnOverlay && e.target === e.currentTarget) close();
  }

  $effect(() => {
    if (open) {
      document.body.style.overflow = 'hidden';
    } else {
      document.body.style.overflow = '';
    }
  });
</script>

{#if open}
  <div class="fixed inset-0 z-50 overflow-y-auto" on:click={handleOverlayClick}>
    <div class="fixed inset-0 bg-black/50 transition-opacity" aria-hidden="true" />
    <div class="flex min-h-full items-center justify-center p-4">
      <div
        class={cn('relative w-full bg-white dark:bg-secondary-800 rounded-xl shadow-xl transform transition-all', sizes[size])}
        use:contentRef
        role="dialog"
        aria-modal="true"
        aria-labelledby="modal-title"
        on:keydown={handleKeydown}
      >
        {#if title}
          <div class="flex items-center justify-between p-4 border-b border-secondary-200 dark:border-secondary-700">
            <h2 id="modal-title" class="text-lg font-semibold text-secondary-900 dark:text-white">{title}</h2>
            <Button variant="ghost" size="icon" on:click={close} aria-label="Close">
              <svg class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </Button>
          </div>
        {/if}
        <div class="p-4">{@render children()}</div>
      </div>
    </div>
  </div>
{/if}