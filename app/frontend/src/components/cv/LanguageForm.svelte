<script lang="ts">
  import { generateId } from '$lib/utils';
  import Button from '$components/ui/Button.svelte';
  import { Trash2, GripVertical, ChevronUp, ChevronDown, Plus } from '@lucide/svelte';
  import type { Language, LanguageProficiency } from '$lib/types';

  interface Props {
    items: Language[];
    onUpdate: (items: Language[]) => void;
    onReorder?: (from: number, to: number) => void;
  }

  let { items, onUpdate, onReorder }: Props = $props();

  const proficiencyLevels: { value: LanguageProficiency; label: string }[] = [
    { value: 'native', label: 'Native' },
    { value: 'fluent', label: 'Fluent' },
    { value: 'professional', label: 'Professional' },
    { value: 'conversational', label: 'Conversational' },
    { value: 'basic', label: 'Basic' },
  ];

  function createLanguage(): Language {
    return {
      id: generateId(),
      name: '',
      proficiency: 'professional',
    };
  }

  function addLanguage() {
    onUpdate([...items, createLanguage()]);
  }

  function removeLanguage(id: string) {
    onUpdate(items.filter((item) => item.id !== id));
  }

  function updateLanguage(id: string, field: keyof Language, value: any) {
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
    <h2 class="text-lg font-semibold">Languages</h2>
    <Button variant="secondary" size="sm" on:click={addLanguage}>
      <Plus class="h-4 w-4 mr-2" />
      Add Language
    </Button>
  </div>

  {#if items.length === 0}
    <div class="text-center py-8 text-secondary-500 border-2 border-dashed border-secondary-300 dark:border-secondary-600 rounded-lg">
      <p class="font-medium">No languages added yet</p>
      <p class="text-sm mt-1">Click "Add Language" to start</p>
    </div>
  {:else}
    <div class="grid gap-4 md:grid-cols-2">
      {#each items as lang, i}
        <div class="border border-secondary-200 dark:border-secondary-700 rounded-lg p-4 space-y-3">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-3">
              <button
                class="p-1 text-secondary-400 hover:text-secondary-600 dark:hover:text-secondary-300 cursor-grab active:cursor-grabbing"
                aria-label="Drag to reorder"
              >
                <GripVertical class="h-5 w-5" />
              </button>
              <h3 class="font-medium">Language #{i + 1}</h3>
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
                on:click={() => removeLanguage(lang.id)}
                aria-label="Remove language"
              >
                <Trash2 class="h-4 w-4" />
              </Button>
            </div>
          </div>
          <div>
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Language *</label>
            <input
              bind:value={lang.name}
              on:input={(e) => updateLanguage(lang.id, 'name', (e.target as HTMLInputElement).value)}
              type="text"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              placeholder="e.g., English, Spanish, French"
              required
            />
          </div>
          <div>
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Proficiency *</label>
            <select
              bind:value={lang.proficiency}
              on:change={(e) => updateLanguage(lang.id, 'proficiency', (e.target as HTMLSelectElement).value)}
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
            >
              {#each proficiencyLevels as level}
                <option value={level.value}>{level.label}</option>
              {/each}
            </select>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>