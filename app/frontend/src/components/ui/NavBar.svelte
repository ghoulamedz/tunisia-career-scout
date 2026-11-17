<script lang="ts">
  import { page } from '$lib/stores';
  import { routes } from '$lib/routes';
  import { cn } from '$lib/utils';

  const navItems = routes.filter(r => !r.path.includes(':'));

  function navigate(path: string) {
    page.navigate(path);
  }
</script>

<nav class="sticky top-0 z-40 bg-white/80 dark:bg-secondary-900/80 backdrop-blur-sm border-b border-secondary-200 dark:border-secondary-700">
  <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
    <div class="flex items-center justify-between h-16">
      <div class="flex items-center gap-8">
        <a href="/" onclick={(e) => { e.preventDefault(); navigate('/'); }} class="flex items-center gap-2 text-xl font-bold text-primary-600 dark:text-primary-400">
          <svg class="h-8 w-8" fill="currentColor" viewBox="0 0 24 24">
            <path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5" />
          </svg>
          <span>Career Scout</span>
        </a>

        <div class="hidden md:flex items-center gap-1">
          {#each navItems as item}
            <button
              onclick={() => navigate(item.path)}
              class={cn(
                'px-3 py-2 rounded-lg text-sm font-medium transition-colors',
                page.url === item.path || page.url.startsWith(item.path + '/')
                  ? 'bg-primary-50 text-primary-700 dark:bg-primary-900/30 dark:text-primary-300'
                  : 'text-secondary-600 hover:bg-secondary-100 dark:text-secondary-400 dark:hover:bg-secondary-800'
              )}
            >
              {item.title}
            </button>
          {/each}
        </div>
      </div>

      <div class="flex items-center gap-4">
        <button class="md:hidden p-2 rounded-lg hover:bg-secondary-100 dark:hover:bg-secondary-800" aria-label="Menu">
          <svg class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16" />
          </svg>
        </button>
      </div>
    </div>
  </div>
</nav>