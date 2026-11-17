<script lang="ts">
  import { cn } from '$lib/utils';

  let {
    variant = 'primary',
    size = 'md',
    disabled = false,
    loading = false,
    class: className,
    type = 'button',
    ...props
  } = $props();

  const baseStyles = 'inline-flex items-center justify-center font-medium transition-colors focus:outline-none focus:ring-2 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed';

  const variants = {
    primary: 'bg-primary-600 text-white hover:bg-primary-700 focus:ring-primary-500',
    secondary: 'bg-secondary-100 text-secondary-900 hover:bg-secondary-200 focus:ring-secondary-500 dark:bg-secondary-700 dark:text-secondary-100 dark:hover:bg-secondary-600',
    outline: 'border border-secondary-300 bg-transparent hover:bg-secondary-100 focus:ring-secondary-500 dark:border-secondary-600 dark:hover:bg-secondary-800',
    ghost: 'bg-transparent hover:bg-secondary-100 focus:ring-secondary-500 dark:hover:bg-secondary-800',
    destructive: 'bg-red-600 text-white hover:bg-red-700 focus:ring-red-500',
  };

  const sizes = {
    sm: 'px-3 py-1.5 text-sm gap-1.5',
    md: 'px-4 py-2 text-sm gap-2',
    lg: 'px-6 py-3 text-base gap-2',
    icon: 'p-2',
  };
</script>

<button
  type={type}
  disabled={disabled || loading}
  class={cn(baseStyles, variants[variant], sizes[size], className)}
  {...props}
>
  {#if loading}
    <svg class="animate-spin h-4 w-4" viewBox="0 0 24 24">
      <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" fill="none" />
      <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
    </svg>
  {:else}
    {@render children()}
  {/if}
</button>