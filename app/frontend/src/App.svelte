<script lang="ts">
  import { page } from '$lib/stores';
  import { routes, getRoute } from '$lib/routes';
  import NavBar from '$components/ui/NavBar.svelte';
  import ToastContainer from '$components/ui/ToastContainer.svelte';
  import { onMount } from 'svelte';

  // Initialize app
  onMount(() => {
    // Load settings, check for CV, etc.
  });
</script>

<div class="min-h-screen bg-secondary-50 dark:bg-secondary-900 transition-colors duration-200">
  <NavBar />

  <main class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
    {#key $page.url}
      {@const route = getRoute($page.url)}
      <svelte:component this={route.component} />
    {/key}
  </main>

  <ToastContainer />
</div>