<script lang="ts">
  import { cn } from '$lib/utils';

  interface Props {
    children: any;
    variant?: 'default' | 'success' | 'warning' | 'destructive' | 'secondary' | 'outline';
    size?: 'sm' | 'md' | 'lg';
    class?: string;
  }

  let { children, variant = 'default', size = 'md', class: className = '' }: Props = $props();

  const baseStyles = 'inline-flex items-center font-medium rounded-full';
  const variantStyles = {
    default: 'bg-primary-100 text-primary-800 dark:bg-primary-900/30 dark:text-primary-400',
    success: 'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400',
    warning: 'bg-amber-100 text-amber-800 dark:bg-amber-900/30 dark:text-amber-400',
    destructive: 'bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400',
    secondary: 'bg-secondary-100 text-secondary-800 dark:bg-secondary-800 dark:text-secondary-300',
    outline: 'border border-secondary-300 dark:border-secondary-600 bg-transparent text-secondary-700 dark:text-secondary-300',
  };
  const sizeStyles = {
    sm: 'px-2 py-0.5 text-xs',
    md: 'px-2.5 py-1 text-xs',
    lg: 'px-3 py-1 text-sm',
  };
</script>

<span class={cn(baseStyles, variantStyles[variant], sizeStyles[size], className)}>
  {@render children()}
</span>