<script lang="ts">
  import { generateId } from '$lib/utils';
  import Button from '$components/ui/Button.svelte';
  import { Trash2, GripVertical } from '@lucide/svelte';
  import type { Experience } from '$lib/types';

  interface Props {
    items: Experience[];
    onUpdate: (items: Experience[]) => void;
    onReorder?: (from: number, to: number) => void;
  }

  let { items, onUpdate, onReorder }: Props = $props();

  function createExperience(): Experience {
    return {
      id: generateId(),
      title: '',
      company: '',
      location: '',
      startDate: new Date().toISOString().split('T')[0],
      endDate: '',
      current: false,
      description: '',
      skills: [],
    };
  }

  function addExperience() {
    onUpdate([...items, createExperience()]);
  }

  function removeExperience(id: string) {
    onUpdate(items.filter((item) => item.id !== id));
  }

  function updateExperience(id: string, field: keyof Experience, value: any) {
    onUpdate(
      items.map((item) =>
        item.id === id ? { ...item, [field]: value } : item
      )
    );
  }

  function handleSkillsChange(id: string, value: string) {
    const skills = value
      .split(',')
      .map((s) => s.trim())
      .filter((s) => s.length > 0);
    updateExperience(id, 'skills', skills);
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
    <h2 class="text-lg font-semibold">Work Experience</h2>
    <Button variant="secondary" size="sm" on:click={addExperience}>
      <Plus class="h-4 w-4 mr-2" />
      Add Experience
    </Button>
  </div>

  {#if items.length === 0}
    <div class="text-center py-8 text-secondary-500 border-2 border-dashed border-secondary-300 dark:border-secondary-600 rounded-lg">
      <p class="font-medium">No experience added yet</p>
      <p class="text-sm mt-1">Click "Add Experience" to start building your work history</p>
    </div>
  {:else}
    {#each items as exp, i}
      <div class="border border-secondary-200 dark:border-secondary-700 rounded-lg p-4 space-y-4">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3">
            <button
              class="p-1 text-secondary-400 hover:text-secondary-600 dark:hover:text-secondary-300 cursor-grab active:cursor-grabbing"
              on:mousedown={() => {}}
              aria-label="Drag to reorder"
            >
              <GripVertical class="h-5 w-5" />
            </button>
            <h3 class="font-medium">Experience #{i + 1}</h3>
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
              on:click={() => removeExperience(exp.id)}
              aria-label="Remove experience"
            >
              <Trash2 class="h-4 w-4" />
            </Button>
          </div>
        </div>

        <div class="grid gap-4 md:grid-cols-2">
          <div>
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Job Title *</label>
            <input
              bind:value={exp.title}
              on:input={(e) => updateExperience(exp.id, 'title', (e.target as HTMLInputElement).value)}
              type="text"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              placeholder="e.g., Senior Software Engineer"
              required
            />
          </div>
          <div>
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Company *</label>
            <input
              bind:value={exp.company}
              on:input={(e) => updateExperience(exp.id, 'company', (e.target as HTMLInputElement).value)}
              type="text"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              placeholder="e.g., TechCorp Inc."
              required
            />
          </div>
          <div>
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Location</label>
            <input
              bind:value={exp.location}
              on:input={(e) => updateExperience(exp.id, 'location', (e.target as HTMLInputElement).value)}
              type="text"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              placeholder="e.g., San Francisco, CA"
            />
          </div>
          <div class="grid gap-2 md:grid-cols-3">
            <div>
              <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Start Date</label>
              <input
                bind:value={exp.startDate}
                on:input={(e) => updateExperience(exp.id, 'startDate', (e.target as HTMLInputElement).value)}
                type="date"
                class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              />
            </div>
            <div>
              <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">End Date</label>
              <input
                bind:value={exp.endDate}
                on:input={(e) => updateExperience(exp.id, 'endDate', (e.target as HTMLInputElement).value)}
                type="date"
                class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              />
            </div>
            <div class="flex items-end">
              <label class="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  bind:checked={exp.current}
                  on:change={(e) => updateExperience(exp.id, 'current', (e.target as HTMLInputElement).checked)}
                  class="h-4 w-4 rounded border-secondary-300 text-primary-600 focus:ring-primary-500"
                />
                <span class="text-sm">Current</span>
              </label>
            </div>
          </div>
          <div class="md:col-span-2">
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Description</label>
            <textarea
              bind:value={exp.description}
              on:input={(e) => updateExperience(exp.id, 'description', (e.target as HTMLTextAreaElement).value)}
              rows="3"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              placeholder="Key responsibilities and achievements..."
            ></textarea>
          </div>
          <div class="md:col-span-2">
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Skills (comma-separated)</label>
            <input
              value={exp.skills.join(', ')}
              on:input={(e) => handleSkillsChange(exp.id, (e.target as HTMLInputElement).value)}
              type="text"
              placeholder="React, TypeScript, Node.js, PostgreSQL"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
            />
            <p class="text-xs text-secondary-500 mt-1">Will be split by commas</p>
          </div>
        </div>
      </div>
    {/each}
  {/if}
</div>