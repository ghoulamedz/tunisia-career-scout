<script lang="ts">
  import { generateId } from '$lib/utils';
  import Button from '$components/ui/Button.svelte';
  import { Trash2, GripVertical, ChevronUp, ChevronDown, Plus } from '@lucide/svelte';
  import type { Skill, SkillLevel } from '$lib/types';

  interface Props {
    items: Skill[];
    onUpdate: (items: Skill[]) => void;
    onReorder?: (from: number, to: number) => void;
  }

  let { items, onUpdate, onReorder }: Props = $props();

  const skillLevels: { value: SkillLevel; label: string }[] = [
    { value: 'beginner', label: 'Beginner' },
    { value: 'intermediate', label: 'Intermediate' },
    { value: 'advanced', label: 'Advanced' },
    { value: 'expert', label: 'Expert' },
  ];

  const skillCategories = [
    'Technical',
    'Soft',
    'Language',
    'Tool',
    'Framework',
    'Database',
    'Cloud',
    'Other',
  ];

  function createSkill(): Skill {
    return {
      id: generateId(),
      name: '',
      category: '',
      level: 'intermediate',
      years: undefined,
    };
  }

  function addSkill() {
    onUpdate([...items, createSkill()]);
  }

  function removeSkill(id: string) {
    onUpdate(items.filter((item) => item.id !== id));
  }

  function updateSkill(id: string, field: keyof Skill, value: any) {
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
    <h2 class="text-lg font-semibold">Skills</h2>
    <Button variant="secondary" size="sm" on:click={addSkill}>
      <Plus class="h-4 w-4 mr-2" />
      Add Skill
    </Button>
  </div>

  {#if items.length === 0}
    <div class="text-center py-8 text-secondary-500 border-2 border-dashed border-secondary-300 dark:border-secondary-600 rounded-lg">
      <p class="font-medium">No skills added yet</p>
      <p class="text-sm mt-1">Click "Add Skill" to start building your skill set</p>
    </div>
  {:else}
    <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
      {#each items as skill, i}
        <div class="border border-secondary-200 dark:border-secondary-700 rounded-lg p-4 space-y-3">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-3">
              <button
                class="p-1 text-secondary-400 hover:text-secondary-600 dark:hover:text-secondary-300 cursor-grab active:cursor-grabbing"
                aria-label="Drag to reorder"
              >
                <GripVertical class="h-5 w-5" />
              </button>
              <h3 class="font-medium">Skill #{i + 1}</h3>
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
                on:click={() => removeSkill(skill.id)}
                aria-label="Remove skill"
              >
                <Trash2 class="h-4 w-4" />
              </Button>
            </div>
          </div>
          <div>
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Name *</label>
            <input
              bind:value={skill.name}
              on:input={(e) => updateSkill(skill.id, 'name', (e.target as HTMLInputElement).value)}
              type="text"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              placeholder="e.g., Rust, Python, Leadership"
              required
            />
          </div>
          <div>
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Category</label>
            <input
              bind:value={skill.category}
              on:input={(e) => updateSkill(skill.id, 'category', (e.target as HTMLInputElement).value)}
              type="text"
              list="skill-categories"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              placeholder="e.g., Technical, Soft, Tool"
            />
            <datalist id="skill-categories">
              {#each skillCategories as cat}
                <option value={cat} />
              {/each}
            </datalist>
          </div>
          <div class="grid gap-2 md:grid-cols-2">
            <div>
              <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Level</label>
              <select
                bind:value={skill.level}
                on:change={(e) => updateSkill(skill.id, 'level', (e.target as HTMLSelectElement).value)}
                class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              >
                {#each skillLevels as level}
                  <option value={level.value}>{level.label}</option>
                {/each}
              </select>
            </div>
            <div>
              <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Years</label>
              <input
                bind:value={skill.years}
                on:input={(e) => updateSkill(skill.id, 'years', parseInt((e.target as HTMLInputElement).value) || undefined)}
                type="number"
                min="0"
                max="50"
                class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              />
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>