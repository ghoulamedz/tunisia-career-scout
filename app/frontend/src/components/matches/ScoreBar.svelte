<script lang="ts">
  import { cn } from '$lib/utils';

  let {
    score,
    size = 'md',
    showLabel = true,
    label = 'Match',
    class: className,
  } = $props();

  const sizes: Record<string, string> = {
    sm: 'h-1.5',
    md: 'h-2.5',
    lg: 'h-4',
    xl: 'h-6',
  };

  const labelSizes: Record<string, string> = {
    sm: 'text-xs',
    md: 'text-sm',
    lg: 'text-base',
    xl: 'text-lg',
  };

  function getScoreColorClass(score: number): string {
    if (score >= 80) return 'bg-green-500';
    if (score >= 60) return 'bg-yellow-500';
    if (score >= 40) return 'bg-orange-500';
    return 'bg-red-500';
  }

  function getScoreTextClass(score: number): string {
    if (score >= 80) return 'text-green-700 dark:text-green-300';
    if (score >= 60) return 'text-yellow-700 dark:text-yellow-300';
    if (score >= 40) return 'text-orange-700 dark:text-orange-300';
    return 'text-red-700 dark:text-red-300';
  }
</script>

<div class={cn('w-full', className)}>
  {#if showLabel}
    <div class="flex items-center justify-between mb-1">
      <span class={cn('font-medium', labelSizes[size] || labelSizes.md)}>{label}</span>
      <span class={cn('font-semibold tabular-nums', labelSizes[size] || labelSizes.md, getScoreTextClass(score))}>
        {Math.round(score)}%
      </span>
    </div>
  {/if}
  <div class={cn('w-full rounded-full overflow-hidden bg-secondary-200 dark:bg-secondary-700', sizes[size] || sizes.md)}>
    <div
      class={cn('h-full transition-all duration-500 ease-out', getScoreColorClass(score))}
      style="width: {Math.min(100, Math.max(0, score))}%"
      role="progressbar"
      aria-valuenow={Math.round(score)}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-label="{label} score"
    ></div>
  </div>
</div>