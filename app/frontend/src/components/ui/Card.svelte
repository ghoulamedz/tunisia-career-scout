<script lang="ts">
  import { cn } from '$lib/utils';

  let { variant = 'default', class: className, ...props } = $props();

  const variants = {
    default: 'bg-white dark:bg-secondary-800 shadow-sm border border-secondary-200 dark:border-secondary-700',
    outlined: 'bg-transparent border border-secondary-300 dark:border-secondary-600',
    elevated: 'bg-white dark:bg-secondary-800 shadow-lg border border-secondary-200 dark:border-secondary-700',
  };
</script>

<div class={cn('rounded-xl', variants[variant], className)} {...props}>
  {@render children()}
</div>