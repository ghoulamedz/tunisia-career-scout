<script lang="ts">
  import { cn } from '$lib/utils';
  import { toasts, toast } from '$lib/stores';
  import { CheckCircle, AlertCircle, AlertTriangle, Info, X } from '@lucide/svelte';

  const icons = {
    success: CheckCircle,
    error: AlertCircle,
    warning: AlertTriangle,
    info: Info,
  };

  const iconColors = {
    success: 'text-green-500',
    error: 'text-red-500',
    warning: 'text-yellow-500',
    info: 'text-blue-500',
  };

  function dismiss(id: string) {
    const idx = toasts.findIndex(t => t.id === id);
    if (idx !== -1) toasts.splice(idx, 1);
  }
</script>

<div class="fixed bottom-4 right-4 z-50 flex flex-col gap-2">
  {#each toasts as t}
    <div class={cn('flex items-start gap-3 p-4 rounded-xl shadow-lg min-w-[300px] max-w-md animate-slide-in', {
      'bg-green-50 border-l-4 border-green-500': t.type === 'success',
      'bg-red-50 border-l-4 border-red-500': t.type === 'error',
      'bg-yellow-50 border-l-4 border-yellow-500': t.type === 'warning',
      'bg-blue-50 border-l-4 border-blue-500': t.type === 'info',
    })}>
      <div class="flex-shrink-0 mt-0.5">
        <span class={iconColors[t.type]}>
          <svelte:component this={icons[t.type]} class="h-5 w-5" />
        </span>
      </div>
      <div class="flex-1">
        <p class="text-sm text-secondary-900 dark:text-white">{t.message}</p>
      </div>
      <button class="flex-shrink-0 text-secondary-400 hover:text-secondary-600" onclick={() => dismiss(t.id)}>
        <X class="h-5 w-5" />
      </button>
    </div>
  {/each}
</div>

<style>
  @keyframes slide-in {
    from {
      opacity: 0;
      transform: translateX(100%);
    }
    to {
      opacity: 1;
      transform: translateX(0);
    }
  }
  .animate-slide-in {
    animation: slide-in 0.3s ease-out;
  }
</style>