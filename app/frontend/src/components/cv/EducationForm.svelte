<script lang="ts">
  import { generateId } from '$lib/utils';
  import Button from '$components/ui/Button.svelte';
  import { Trash2, GripVertical, ChevronUp, ChevronDown, Plus } from '@lucide/svelte';
  import type { Education } from '$lib/types';

  interface Props {
    items: Education[];
    onUpdate: (items: Education[]) => void;
    onReorder?: (from: number, to: number) => void;
  }

  let { items, onUpdate, onReorder }: Props = $props();

  function createEducation(): Education {
    return {
      id: generateId(),
      institution: '',
      degree: '',
      fieldOfStudy: '',
      location: '',
      startDate: new Date().toISOString().split('T')[0],
      endDate: '',
      current: false,
      description: '',
      grade: '',
    };
  }

  function addEducation() {
    onUpdate([...items, createEducation()]);
  }

  function removeEducation(id: string) {
    onUpdate(items.filter((item) => item.id !== id));
  }

  function updateEducation(id: string, field: keyof Education, value: any) {
    onUpdate(
      items.map((item) =>
        item.id === id ? { ...item, [field]: value } : item
      )
    );
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
    <h2 class="text-lg font-semibold">Education</h2>
    <Button variant="secondary" size="sm" on:click={addEducation}>
      <Plus class="h-4 w-4 mr-2" />
      Add Education
    </Button>
  </div>

  {#if items.length === 0}
    <div class="text-center py-8 text-secondary-500 border-2 border-dashed border-secondary-300 dark:border-secondary-600 rounded-lg">
      <p class="font-medium">No education added yet</p>
      <p class="text-sm mt-1">Click "Add Education" to start building your academic history</p>
    </div>
  {:else}
    {#each items as edu, i}
      <div class="border border-secondary-200 dark:border-secondary-700 rounded-lg p-4 space-y-4">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3">
            <button
              class="p-1 text-secondary-400 hover:text-secondary-600 dark:hover:text-secondary-300 cursor-grab active:cursor-grabbing"
              aria-label="Drag to reorder"
            >
              <GripVertical class="h-5 w-5" />
            </button>
            <h3 class="font-medium">Education #{i + 1}</h3>
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
              on:click={() => removeEducation(edu.id)}
              aria-label="Remove education"
            >
              <Trash2 class="h-4 w-4" />
            </Button>
          </div>
        </div>

        <div class="grid gap-4 md:grid-cols-2">
          <div>
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Institution *</label>
            <input
              bind:value={edu.institution}
              on:input={(e) => updateEducation(edu.id, 'institution', (e.target as HTMLInputElement).value)}
              type="text"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              placeholder="e.g., Stanford University"
              required
            />
          </div>
          <div>
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Degree *</label>
            <input
              bind:value={edu.degree}
              on:input={(e) => updateEducation(edu.id, 'degree', (e.target as HTMLInputElement).value)}
              type="text"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              placeholder="e.g., M.S. Computer Science"
              required
            />
          </div>
          <div>
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Field of Study</label>
            <input
              bind:value={edu.fieldOfStudy}
              on:input={(e) => updateEducation(edu.id, 'fieldOfStudy', (e.target as HTMLInputElement).value)}
              type="text"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              placeholder="e.g., Distributed Systems"
            />
          </div>
          <div>
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Location</label>
            <input
              bind:value={edu.location}
              on:input={(e) => updateEducation(edu.id, 'location', (e.target as HTMLInputElement).value)}
              type="text"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              placeholder="e.g., Stanford, CA"
            />
          </div>
          <div class="grid gap-2 md:grid-cols-3">
            <div>
              <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Start Date</label>
              <input
                bind:value={edu.startDate}
                on:input={(e) => updateEducation(edu.id, 'startDate', (e.target as HTMLInputElement).value)}
                type="date"
                class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              />
            </div>
            <div>
              <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">End Date</label>
              <input
                bind:value={edu.endDate}
                on:input={(e) => updateEducation(edu.id, 'endDate', (e.target as HTMLInputElement).value)}
                type="date"
                class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              />
            </div>
            <div class="flex items-end">
              <label class="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  bind:checked={edu.current}
                  on:change={(e) => updateEducation(edu.id, 'current', (e.target as HTMLInputElement).checked)}
                  class="h-4 w-4 rounded border-secondary-300 text-primary-600 focus:ring-primary-500"
                />
                <span class="text-sm">Current</span>
              </label>
            </div>
          </div>
          <div>
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Grade</label>
            <input
              bind:value={edu.grade}
              on:input={(e) => updateEducation(edu.id, 'grade', (e.target as HTMLInputElement).value)}
              type="text"
              placeholder="e.g., 3.8/4.0, First Class Honours"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
            />
          </div>
          <div class="md:col-span-2">
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Description</label>
            <textarea
              bind:value={edu.description}
              on:input={(e) => updateEducation(edu.id, 'description', (e.target as HTMLTextAreaElement).value)}
              rows="2"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              placeholder="Relevant coursework, honors, activities..."
            ></textarea>
          </div>
        </div>
      </div>
    {/each}
  {/if}
</div>