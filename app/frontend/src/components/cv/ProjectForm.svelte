<script lang="ts">
  import { generateId } from '$lib/utils';
  import Button from '$components/ui/Button.svelte';
  import { Trash2, GripVertical, ChevronUp, ChevronDown, Plus } from '@lucide/svelte';
  import type { Project } from '$lib/types';

  interface Props {
    items: Project[];
    onUpdate: (items: Project[]) => void;
    onReorder?: (from: number, to: number) => void;
  }

  let { items, onUpdate, onReorder }: Props = $props();

  function createProject(): Project {
    return {
      id: generateId(),
      name: '',
      description: '',
      url: '',
      github: '',
      technologies: [],
      startDate: '',
      endDate: '',
    };
  }

  function addProject() {
    onUpdate([...items, createProject()]);
  }

  function removeProject(id: string) {
    onUpdate(items.filter((item) => item.id !== id));
  }

  function updateProject(id: string, field: keyof Project, value: any) {
    onUpdate(
      items.map((item) =>
        item.id === id ? { ...item, [field]: value } : item
      )
    );
  }

  function handleTechnologiesChange(id: string, value: string) {
    const technologies = value
      .split(',')
      .map((s) => s.trim())
      .filter((s) => s.length > 0);
    updateProject(id, 'technologies', technologies);
  }

  function moveUp(index: number) {
    if (index > 0 && onReorder) {
      onReorder(index, index - 1);
    }
  }

  function moveDown(index: number) {
    if (index < items.length - 1 && onReorder) {
      onReorder(index, index + 1);
    }
  }
</script>

<div class="space-y-6">
  <div class="flex items-center justify-between">
    <h2 class="text-lg font-semibold">Projects</h2>
    <Button variant="secondary" size="sm" on:click={addProject}>
      <Plus class="h-4 w-4 mr-2" />
      Add Project
    </Button>
  </div>

  {#if items.length === 0}
    <div class="text-center py-8 text-secondary-500 border-2 border-dashed border-secondary-300 dark:border-secondary-600 rounded-lg">
      <p class="font-medium">No projects added yet</p>
      <p class="text-sm mt-1">Click "Add Project" to start</p>
    </div>
  {:else}
    {#each items as proj, i}
      <div class="border border-secondary-200 dark:border-secondary-700 rounded-lg p-4 space-y-4">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3">
            <button
              class="p-1 text-secondary-400 hover:text-secondary-600 dark:hover:text-secondary-300 cursor-grab active:cursor-grabbing"
              aria-label="Drag to reorder"
            >
              <GripVertical class="h-5 w-5" />
            </button>
            <h3 class="font-medium">Project #{i + 1}</h3>
          </div>
          <div class="flex items-center gap-2">
            <Button
              variant="ghost"
              size="icon"
              class="text-secondary-500"
              disabled={i === 0}
              on:click={() => moveUp(i)}
              aria-label="Move up"
            >
              <ChevronUp class="h-4 w-4" />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              class="text-secondary-500"
              disabled={i === items.length - 1}
              on:click={() => moveDown(i)}
              aria-label="Move down"
            >
              <ChevronDown class="h-4 w-4" />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              class="text-red-600 hover:text-red-700"
              on:click={() => removeProject(proj.id)}
              aria-label="Remove project"
            >
              <Trash2 class="h-4 w-4" />
            </Button>
          </div>
        </div>

        <div class="grid gap-4 md:grid-cols-2">
          <div>
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Name *</label>
            <input
              bind:value={proj.name}
              on:input={(e) => updateProject(proj.id, 'name', (e.target as HTMLInputElement).value)}
              type="text"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              placeholder="e.g., Distributed Task Queue"
              required
            />
          </div>
          <div class="md:col-span-2">
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Description</label>
            <textarea
              bind:value={proj.description}
              on:input={(e) => updateProject(proj.id, 'description', (e.target as HTMLTextAreaElement).value)}
              rows="2"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              placeholder="Brief description of the project..."
            ></textarea>
          </div>
          <div>
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Live URL</label>
            <input
              bind:value={proj.url}
              on:input={(e) => updateProject(proj.id, 'url', (e.target as HTMLInputElement).value)}
              type="url"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              placeholder="https://project-demo.com"
            />
          </div>
          <div>
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">GitHub</label>
            <input
              bind:value={proj.github}
              on:input={(e) => updateProject(proj.id, 'github', (e.target as HTMLInputElement).value)}
              type="url"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              placeholder="https://github.com/user/project"
            />
          </div>
          <div class="md:col-span-2">
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Technologies (comma-separated)</label>
            <input
              value={proj.technologies.join(', ')}
              on:input={(e) => handleTechnologiesChange(proj.id, (e.target as HTMLInputElement).value)}
              type="text"
              placeholder="React, TypeScript, Tailwind, Node.js"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
            />
            <p class="text-xs text-secondary-500 mt-1">Will be split by commas</p>
          </div>
          <div class="grid gap-2 md:grid-cols-2">
            <div>
              <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Start Date</label>
              <input
                bind:value={proj.startDate}
                on:input={(e) => updateProject(proj.id, 'startDate', (e.target as HTMLInputElement).value)}
                type="date"
                class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              />
            </div>
            <div>
              <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">End Date</label>
              <input
                bind:value={proj.endDate}
                on:input={(e) => updateProject(proj.id, 'endDate', (e.target as HTMLInputElement).value)}
                type="date"
                class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              />
            </div>
          </div>
        </div>
      </div>
    {/each}
  {/if}
</div>