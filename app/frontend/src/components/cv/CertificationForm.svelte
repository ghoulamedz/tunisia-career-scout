<script lang="ts">
  import { generateId } from '$lib/utils';
  import Button from '$components/ui/Button.svelte';
  import { Trash2, GripVertical, ChevronUp, ChevronDown, Plus } from '@lucide/svelte';
  import type { Certification } from '$lib/types';

  interface Props {
    items: Certification[];
    onUpdate: (items: Certification[]) => void;
    onReorder?: (from: number, to: number) => void;
  }

  let { items, onUpdate, onReorder }: Props = $props();

  function createCertification(): Certification {
    return {
      id: generateId(),
      name: '',
      issuer: '',
      date: new Date().toISOString().split('T')[0],
      expiry: '',
      url: '',
      credentialId: '',
    };
  }

  function addCertification() {
    onUpdate([...items, createCertification()]);
  }

  function removeCertification(id: string) {
    onUpdate(items.filter((item) => item.id !== id));
  }

  function updateCertification(id: string, field: keyof Certification, value: any) {
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
    <h2 class="text-lg font-semibold">Certifications</h2>
    <Button variant="secondary" size="sm" on:click={addCertification}>
      <Plus class="h-4 w-4 mr-2" />
      Add Certification
    </Button>
  </div>

  {#if items.length === 0}
    <div class="text-center py-8 text-secondary-500 border-2 border-dashed border-secondary-300 dark:border-secondary-600 rounded-lg">
      <p class="font-medium">No certifications added yet</p>
      <p class="text-sm mt-1">Click "Add Certification" to start</p>
    </div>
  {:else}
    {#each items as cert, i}
      <div class="border border-secondary-200 dark:border-secondary-700 rounded-lg p-4 space-y-4">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3">
            <button
              class="p-1 text-secondary-400 hover:text-secondary-600 dark:hover:text-secondary-300 cursor-grab active:cursor-grabbing"
              aria-label="Drag to reorder"
            >
              <GripVertical class="h-5 w-5" />
            </button>
            <h3 class="font-medium">Certification #{i + 1}</h3>
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
              on:click={() => removeCertification(cert.id)}
              aria-label="Remove certification"
            >
              <Trash2 class="h-4 w-4" />
            </Button>
          </div>
        </div>

        <div class="grid gap-4 md:grid-cols-2">
          <div>
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Name *</label>
            <input
              bind:value={cert.name}
              on:input={(e) => updateCertification(cert.id, 'name', (e.target as HTMLInputElement).value)}
              type="text"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              placeholder="e.g., AWS Certified Solutions Architect"
              required
            />
          </div>
          <div>
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Issuer *</label>
            <input
              bind:value={cert.issuer}
              on:input={(e) => updateCertification(cert.id, 'issuer', (e.target as HTMLInputElement).value)}
              type="text"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              placeholder="e.g., Amazon Web Services"
              required
            />
          </div>
          <div>
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Date *</label>
            <input
              bind:value={cert.date}
              on:input={(e) => updateCertification(cert.id, 'date', (e.target as HTMLInputElement).value)}
              type="date"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              required
            />
          </div>
          <div>
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Expiry</label>
            <input
              bind:value={cert.expiry}
              on:input={(e) => updateCertification(cert.id, 'expiry', (e.target as HTMLInputElement).value)}
              type="date"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
            />
          </div>
          <div class="md:col-span-2">
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Verification URL</label>
            <input
              bind:value={cert.url}
              on:input={(e) => updateCertification(cert.id, 'url', (e.target as HTMLInputElement).value)}
              type="url"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              placeholder="https://verify.certification.com/..."
            />
          </div>
          <div class="md:col-span-2">
            <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Credential ID</label>
            <input
              bind:value={cert.credentialId}
              on:input={(e) => updateCertification(cert.id, 'credentialId', (e.target as HTMLInputElement).value)}
              type="text"
              class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              placeholder="e.g., ABC123XYZ"
            />
          </div>
        </div>
      </div>
    {/each}
  {/if}
</div>