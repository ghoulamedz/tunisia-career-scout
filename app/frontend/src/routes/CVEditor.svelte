<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { cvStore, toast, page } from '$lib/stores';
  import { cvApi } from '$lib/api';
  import { cn, generateId, debounce } from '$lib/utils';
  import Card from '$components/ui/Card.svelte';
  import Button from '$components/ui/Button.svelte';
  import Modal from '$components/ui/Modal.svelte';
  import CvPreview from '$components/CvPreview.svelte';
  import ExperienceForm from '$components/cv/ExperienceForm.svelte';
  import EducationForm from '$components/cv/EducationForm.svelte';
  import SkillForm from '$components/cv/SkillForm.svelte';
  import LanguageForm from '$components/cv/LanguageForm.svelte';
  import ProjectForm from '$components/cv/ProjectForm.svelte';
  import CertificationForm from '$components/cv/CertificationForm.svelte';
  import {
    Plus,
    Trash2,
    Edit,
    Eye,
    Download,
    Save,
    ChevronLeft,
    ChevronRight,
    GripVertical,
    Loader2,
    AlertCircle,
    Layout,
    Palette,
    ChevronUp,
    ChevronDown,
    Upload,
    FileJson,
    User,
  } from '@lucide/svelte';
  import type { CVData, Experience, Education, Skill, Language, Project, Certification, CVTemplate, CVTheme, CVProfile } from '$lib/types';

  // Convert backend CVData (snake_case) to frontend CVData (camelCase)
  function toFrontendCVData(data: any): CVData {
    if (!data) return getEmptyCVData();
    return {
      fullName: data.full_name || '',
      email: data.email || '',
      phone: data.phone || '',
      location: data.location || '',
      linkedin: data.linkedin || '',
      github: data.github || '',
      portfolio: data.portfolio || '',
      summary: data.summary || '',
      experience: (data.experience || []).map((e: any) => ({
        id: e.id || generateId(),
        title: e.title || '',
        company: e.company || '',
        location: e.location || '',
        startDate: e.start_date ? e.start_date.split('T')[0] : new Date().toISOString().split('T')[0],
        endDate: e.end_date ? e.end_date.split('T')[0] : '',
        current: e.current || false,
        description: e.description || '',
        skills: e.skills || [],
      })),
      education: (data.education || []).map((e: any) => ({
        id: e.id || generateId(),
        institution: e.institution || '',
        degree: e.degree || '',
        fieldOfStudy: e.field_of_study || '',
        location: e.location || '',
        startDate: e.start_date ? e.start_date.split('T')[0] : new Date().toISOString().split('T')[0],
        endDate: e.end_date ? e.end_date.split('T')[0] : '',
        current: e.current || false,
        description: e.description || '',
        grade: e.grade || '',
      })),
      skills: (data.skills || []).map((s: any) => ({
        id: s.id || generateId(),
        name: s.name || '',
        category: s.category || '',
        level: s.level || 'intermediate',
        years: s.years,
      })),
      languages: (data.languages || []).map((l: any) => ({
        id: l.id || generateId(),
        name: l.name || '',
        proficiency: l.proficiency || 'professional',
      })),
      projects: (data.projects || []).map((p: any) => ({
        id: p.id || generateId(),
        name: p.name || '',
        description: p.description || '',
        url: p.url || '',
        github: p.github || '',
        technologies: p.technologies || [],
        startDate: p.start_date ? p.start_date.split('T')[0] : '',
        endDate: p.end_date ? p.end_date.split('T')[0] : '',
      })),
      certifications: (data.certifications || []).map((c: any) => ({
        id: c.id || generateId(),
        name: c.name || '',
        issuer: c.issuer || '',
        date: c.date ? c.date.split('T')[0] : new Date().toISOString().split('T')[0],
        expiry: c.expiry ? c.expiry.split('T')[0] : '',
        url: c.url || '',
        credentialId: c.credential_id || '',
      })),
      template: data.template || 'modern',
      theme: data.theme || 'auto',
    };
  }

  // Convert frontend CVData (camelCase) to backend CVData (snake_case)
  function toBackendCVData(data: CVData): any {
    return {
      full_name: data.fullName,
      email: data.email,
      phone: data.phone,
      location: data.location,
      linkedin: data.linkedin,
      github: data.github,
      portfolio: data.portfolio,
      summary: data.summary,
      experience: data.experience.map((e) => ({
        id: e.id,
        title: e.title,
        company: e.company,
        location: e.location,
        start_date: e.startDate,
        end_date: e.endDate || null,
        current: e.current,
        description: e.description,
        skills: e.skills,
      })),
      education: data.education.map((e) => ({
        id: e.id,
        institution: e.institution,
        degree: e.degree,
        field_of_study: e.fieldOfStudy,
        location: e.location,
        start_date: e.startDate,
        end_date: e.endDate || null,
        current: e.current,
        description: e.description,
        grade: e.grade,
      })),
      skills: data.skills.map((s) => ({
        id: s.id,
        name: s.name,
        category: s.category,
        level: s.level,
        years: s.years,
      })),
      languages: data.languages.map((l) => ({
        id: l.id,
        name: l.name,
        proficiency: l.proficiency,
      })),
      projects: data.projects.map((p) => ({
        id: p.id,
        name: p.name,
        description: p.description,
        url: p.url,
        github: p.github,
        technologies: p.technologies,
        start_date: p.startDate || null,
        end_date: p.endDate || null,
      })),
      certifications: data.certifications.map((c) => ({
        id: c.id,
        name: c.name,
        issuer: c.issuer,
        date: c.date,
        expiry: c.expiry || null,
        url: c.url,
        credential_id: c.credentialId,
      })),
      template: data.template,
      theme: data.theme,
    };
  }

  function getEmptyCVData(): CVData {
    return {
      fullName: '',
      email: '',
      phone: '',
      location: '',
      linkedin: '',
      github: '',
      portfolio: '',
      summary: '',
      experience: [],
      education: [],
      skills: [],
      languages: [],
      projects: [],
      certifications: [],
      template: 'modern',
      theme: 'auto',
    };
  }

  let cvData = $state<CVData>(getEmptyCVData());
  let activeSection = $state<'personal' | 'experience' | 'education' | 'skills' | 'languages' | 'projects' | 'certifications' | 'template'>('personal');
  let saving = $state(false);
  let previewLoading = $state(false);
  let previewError = $state<string | null>(null);
  let previewPdf = $state<ArrayBuffer | null>(null);
  let showPreviewPanel = $state(true);
  let validationErrors = $state<Record<string, string>>({});
  let showImportModal = $state(false);
  let importMethod = $state<'linkedin' | 'json'>('json');
  let importJsonText = $state('');
  let importPlaceholder = $state('{"fullName": "John Doe", "email": "john@example.com", ...}');
  let importLoading = $state(false);

  const sections = [
    { id: 'personal', label: 'Personal', icon: 'User' },
    { id: 'experience', label: 'Experience', icon: 'Briefcase' },
    { id: 'education', label: 'Education', icon: 'GraduationCap' },
    { id: 'skills', label: 'Skills', icon: 'Code' },
    { id: 'languages', label: 'Languages', icon: 'Languages' },
    { id: 'projects', label: 'Projects', icon: 'FolderGit' },
    { id: 'certifications', label: 'Certifications', icon: 'Award' },
    { id: 'template', label: 'Template', icon: 'Layout' },
  ] as const;

  // Debounced preview generation (300ms)
  const generatePreviewDebounced = debounce(async () => {
    if (!cvData.fullName && !cvData.email) return;
    
    previewLoading = true;
    previewError = null;
    try {
      const pdf = await cvApi.compilePreview(cvData, cvData.template as CVTemplate);
      previewPdf = pdf;
    } catch (e) {
      console.error('Preview generation failed:', e);
      previewError = 'Failed to generate preview. Check console for details.';
      previewPdf = null;
    } finally {
      previewLoading = false;
    }
  }, 300);

  // Debounced auto-save to localStorage (2s)
  const autoSaveDebounced = debounce(() => {
    try {
      localStorage.setItem('cv_editor_autosave', JSON.stringify({
        data: cvData,
        timestamp: Date.now(),
      }));
    } catch (e) {
      console.warn('Auto-save failed:', e);
    }
  }, 2000);

  // Trigger preview and auto-save on cvData changes
  $effect.root(() => {
    $effect(() => {
      // Access all cvData fields to trigger reactivity
      cvData.fullName;
      cvData.email;
      cvData.phone;
      cvData.location;
      cvData.linkedin;
      cvData.github;
      cvData.portfolio;
      cvData.summary;
      cvData.template;
      cvData.theme;
      cvData.experience.length;
      cvData.education.length;
      cvData.skills.length;
      cvData.languages.length;
      cvData.projects.length;
      cvData.certifications.length;
      
      // Watch nested arrays
      cvData.experience.forEach(e => { e.title; e.company; e.description; e.skills; e.startDate; e.endDate; e.current; e.location; });
      cvData.education.forEach(e => { e.institution; e.degree; e.description; e.fieldOfStudy; e.startDate; e.endDate; e.current; e.location; e.grade; });
      cvData.skills.forEach(s => { s.name; s.category; s.level; s.years; });
      cvData.languages.forEach(l => { l.name; l.proficiency; });
      cvData.projects.forEach(p => { p.name; p.description; p.technologies; p.url; p.github; p.startDate; p.endDate; });
      cvData.certifications.forEach(c => { c.name; c.issuer; c.date; c.expiry; c.url; c.credentialId; });
      
      generatePreviewDebounced();
      autoSaveDebounced();
    });
  });

  async function loadCV() {
    const cvId = page.params.id;
    
    if (cvId) {
      // Editing existing CV
      try {
        const profile = await cvApi.get(cvId);
        if (profile) {
          cvData = toFrontendCVData(profile.data);
          return;
        }
      } catch (e) {
        console.error('Failed to load CV:', e);
        toast('error', 'Failed to load CV');
      }
    } else {
      // Creating new CV - load default or from localStorage
      try {
        const cvs = await cvApi.list();
        const defaultCv = cvs.find(c => c.isDefault) || cvs[0];
        if (defaultCv) {
          cvData = toFrontendCVData(defaultCv.data);
          return;
        }
      } catch (e) {
        console.error('Failed to load default CV:', e);
      }
      loadFromLocalStorage();
    }
  }

  function loadFromLocalStorage() {
    try {
      const saved = localStorage.getItem('cv_editor_autosave');
      if (saved) {
        const parsed = JSON.parse(saved);
        if (parsed.data) {
          cvData = toFrontendCVData(parsed.data);
          toast('info', 'Restored from auto-save');
        }
      }
    } catch (e) {
      console.warn('Failed to load from localStorage:', e);
    }
  }

  function validateCV(): boolean {
    const errors: Record<string, string> = {};
    
    if (!cvData.fullName.trim()) {
      errors.fullName = 'Full name is required';
    }
    if (!cvData.email.trim()) {
      errors.email = 'Email is required';
    } else if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(cvData.email)) {
      errors.email = 'Invalid email format';
    }
    
    // Validate experience
    cvData.experience.forEach((exp, i) => {
      if (!exp.title.trim()) errors[`experience_${i}_title`] = 'Job title is required';
      if (!exp.company.trim()) errors[`experience_${i}_company`] = 'Company is required';
      if (!exp.startDate) errors[`experience_${i}_startDate`] = 'Start date is required';
    });
    
    // Validate education
    cvData.education.forEach((edu, i) => {
      if (!edu.institution.trim()) errors[`education_${i}_institution`] = 'Institution is required';
      if (!edu.degree.trim()) errors[`education_${i}_degree`] = 'Degree is required';
      if (!edu.startDate) errors[`education_${i}_startDate`] = 'Start date is required';
    });
    
    // Validate skills
    cvData.skills.forEach((skill, i) => {
      if (!skill.name.trim()) errors[`skill_${i}_name`] = 'Skill name is required';
    });
    
    // Validate languages
    cvData.languages.forEach((lang, i) => {
      if (!lang.name.trim()) errors[`language_${i}_name`] = 'Language name is required';
    });
    
    // Validate projects
    cvData.projects.forEach((proj, i) => {
      if (!proj.name.trim()) errors[`project_${i}_name`] = 'Project name is required';
    });
    
    // Validate certifications
    cvData.certifications.forEach((cert, i) => {
      if (!cert.name.trim()) errors[`certification_${i}_name`] = 'Certification name is required';
      if (!cert.issuer.trim()) errors[`certification_${i}_issuer`] = 'Issuer is required';
      if (!cert.date) errors[`certification_${i}_date`] = 'Date is required';
    });
    
    validationErrors = errors;
    return Object.keys(errors).length === 0;
  }

  async function saveCV(asDefault = true) {
    if (!validateCV()) {
      toast('error', 'Please fix validation errors before saving');
      return;
    }
    
    const cvId = page.params.id;
    saving = true;
    try {
      const name = cvData.fullName || 'My CV';
      const backendData = toBackendCVData(cvData);
      
      if (cvId) {
        // Update existing CV
        await cvApi.update(cvId, { name, data: backendData, is_default: asDefault });
        toast('success', 'CV updated successfully');
      } else {
        // Create new CV
        await cvApi.save({ name, data: backendData, is_default: asDefault });
        toast('success', 'CV saved successfully');
      }
      await cvStore.load();
    } catch (e) {
      console.error('Failed to save CV:', e);
      toast('error', cvId ? 'Failed to update CV' : 'Failed to save CV');
    } finally {
      saving = false;
    }
  }

  async function generatePreviewManual() {
    previewLoading = true;
    previewError = null;
    try {
      const pdf = await cvApi.compilePreview(cvData, cvData.template as CVTemplate);
      previewPdf = pdf;
      showPreviewPanel = true;
    } catch (e) {
      console.error('Preview generation failed:', e);
      previewError = 'Failed to generate preview. Check console for details.';
      previewPdf = null;
    } finally {
      previewLoading = false;
    }
  }

  async function downloadPDF() {
    if (!previewPdf) return;
    const blob = new Blob([previewPdf], { type: 'application/pdf' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${cvData.fullName || 'CV'}.pdf`;
    a.click();
    URL.revokeObjectURL(url);
  }

  function handleImportJson() {
    try {
      const parsed = JSON.parse(importJsonText);
      // Handle both frontend and backend formats
      const data = parsed.data || parsed;
      cvData = toFrontendCVData(data);
      showImportModal = false;
      importJsonText = '';
      toast('success', 'CV imported from JSON');
    } catch (e) {
      console.error('Import failed:', e);
      toast('error', 'Invalid JSON format');
    }
  }

  async function handleImportLinkedIn() {
    // Placeholder for LinkedIn import
    toast('info', 'LinkedIn import coming soon. Use JSON import for now.');
    showImportModal = false;
  }

  function handleImport() {
    if (importMethod === 'json') {
      handleImportJson();
    } else {
      handleImportLinkedIn();
    }
  }

  function setSection(section: typeof activeSection) {
    activeSection = section;
  }

  onMount(async () => {
    await loadCV();
  });

  onDestroy(() => {
    generatePreviewDebounced.cancel();
    autoSaveDebounced.cancel();
  });
</script>

<div class="space-y-6">
  <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
    <div class="flex items-center gap-4">
      <a href="/cv" class="flex items-center gap-2 text-secondary-600 hover:text-secondary-900 dark:text-secondary-400 dark:hover:text-white">
        <ChevronLeft class="h-5 w-5" />
      </a>
      <div>
        <h1 class="text-2xl font-bold text-secondary-900 dark:text-white">{page.params.id ? 'Edit CV' : 'New CV'}</h1>
        <p class="text-secondary-600 dark:text-secondary-400">{page.params.id ? 'Update your existing CV profile' : 'Build your professional CV with live preview'}</p>
      </div>
    </div>
    <div class="flex flex-wrap items-center gap-2">
      <Button variant="secondary" on:click={() => showImportModal = true}>
        <Upload class="h-4 w-4 mr-2" />
        Import
      </Button>
      <Button variant="secondary" on:click={saveCV} disabled={saving}>
        <Save class="h-4 w-4 mr-2" />
        {saving ? 'Saving...' : (page.params.id ? 'Update CV' : 'Save as New')}
      </Button>
      <Button variant="secondary" on:click={() => showPreviewPanel = !showPreviewPanel}>
        <Layout class="h-4 w-4 mr-2" />
        Toggle Preview
      </Button>
      <Button variant="primary" on:click={generatePreviewManual} disabled={previewLoading || (!cvData.fullName && !cvData.email)}>
        <Eye class="h-4 w-4 mr-2" />
        {previewLoading ? 'Generating...' : 'Refresh Preview'}
      </Button>
      {#if previewPdf}
        <Button variant="secondary" on:click={downloadPDF}>
          <Download class="h-4 w-4 mr-2" />
          Download PDF
        </Button>
      {/if}
    </div>
  </div>

  <div class="flex gap-6">
    <!-- Sidebar Navigation -->
    <aside class="w-48 flex-shrink-0 hidden lg:block">
      <Card class="h-fit sticky top-24">
        <nav class="p-2 space-y-1">
          {#each sections as section}
            <button
              on:click={() => setSection(section.id as typeof activeSection)}
              class={cn(
                'w-full px-3 py-2 rounded-lg text-sm font-medium transition-colors text-left flex items-center gap-2',
                activeSection === section.id
                  ? 'bg-primary-50 dark:bg-primary-900/30 text-primary-700 dark:text-primary-300'
                  : 'text-secondary-600 hover:bg-secondary-100 dark:text-secondary-400 dark:hover:bg-secondary-800'
              )}
            >
              <span>{section.label}</span>
            </button>
          {/each}
        </nav>
      </Card>
    </aside>

    <!-- Main Content: Editor + Preview Panel -->
    <main class="flex-1 min-w-0 flex flex-col lg:flex-row">
      <!-- Editor Panel -->
      <div class="flex-1 min-w-0 lg:max-w-2xl">
        {#if activeSection === 'personal'}
          <Card>
            <div class="p-6 space-y-6">
              <h2 class="text-lg font-semibold">Personal Information</h2>
              <div class="grid gap-4 md:grid-cols-2">
                <div>
                  <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Full Name *</label>
                  <input
                    bind:value={cvData.fullName}
                    type="text"
                    class={cn(
                      'w-full px-3 py-2 border rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent',
                      validationErrors.fullName ? 'border-red-500' : 'border-secondary-300 dark:border-secondary-600'
                    )}
                    aria-invalid={!!validationErrors.fullName}
                    aria-describedby={validationErrors.fullName ? 'fullName-error' : undefined}
                  />
                  {#if validationErrors.fullName}
                    <p id="fullName-error" class="mt-1 text-sm text-red-600 dark:text-red-400">{validationErrors.fullName}</p>
                  {/if}
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Email *</label>
                  <input
                    bind:value={cvData.email}
                    type="email"
                    class={cn(
                      'w-full px-3 py-2 border rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent',
                      validationErrors.email ? 'border-red-500' : 'border-secondary-300 dark:border-secondary-600'
                    )}
                    aria-invalid={!!validationErrors.email}
                    aria-describedby={validationErrors.email ? 'email-error' : undefined}
                  />
                  {#if validationErrors.email}
                    <p id="email-error" class="mt-1 text-sm text-red-600 dark:text-red-400">{validationErrors.email}</p>
                  {/if}
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Phone</label>
                  <input bind:value={cvData.phone} type="tel" class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent" />
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Location</label>
                  <input bind:value={cvData.location} type="text" class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent" />
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">LinkedIn</label>
                  <input bind:value={cvData.linkedin} type="url" placeholder="https://linkedin.com/in/username" class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent" />
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">GitHub</label>
                  <input bind:value={cvData.github} type="url" placeholder="https://github.com/username" class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent" />
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Portfolio</label>
                  <input bind:value={cvData.portfolio} type="url" placeholder="https://yourportfolio.com" class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent" />
                </div>
              </div>
              <div>
                <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">Professional Summary</label>
                <textarea bind:value={cvData.summary} rows="4" class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent" placeholder="A brief summary of your professional background..."></textarea>
              </div>
            </div>
          </Card>

        {:else if activeSection === 'experience'}
          <Card>
            <div class="p-6">
              <ExperienceForm
                items={cvData.experience}
                onUpdate={(items) => cvData.experience = items}
                onReorder={(from, to) => {
                  const newItems = [...cvData.experience];
                  const [removed] = newItems.splice(from, 1);
                  newItems.splice(to, 0, removed);
                  cvData.experience = newItems;
                }}
              />
            </div>
          </Card>

        {:else if activeSection === 'education'}
          <Card>
            <div class="p-6">
              <EducationForm
                items={cvData.education}
                onUpdate={(items) => cvData.education = items}
                onReorder={(from, to) => {
                  const newItems = [...cvData.education];
                  const [removed] = newItems.splice(from, 1);
                  newItems.splice(to, 0, removed);
                  cvData.education = newItems;
                }}
              />
            </div>
          </Card>

        {:else if activeSection === 'skills'}
          <Card>
            <div class="p-6">
              <SkillForm
                items={cvData.skills}
                onUpdate={(items) => cvData.skills = items}
                onReorder={(from, to) => {
                  const newItems = [...cvData.skills];
                  const [removed] = newItems.splice(from, 1);
                  newItems.splice(to, 0, removed);
                  cvData.skills = newItems;
                }}
              />
            </div>
          </Card>

        {:else if activeSection === 'languages'}
          <Card>
            <div class="p-6">
              <LanguageForm
                items={cvData.languages}
                onUpdate={(items) => cvData.languages = items}
                onReorder={(from, to) => {
                  const newItems = [...cvData.languages];
                  const [removed] = newItems.splice(from, 1);
                  newItems.splice(to, 0, removed);
                  cvData.languages = newItems;
                }}
              />
            </div>
          </Card>

        {:else if activeSection === 'projects'}
          <Card>
            <div class="p-6">
              <ProjectForm
                items={cvData.projects}
                onUpdate={(items) => cvData.projects = items}
                onReorder={(from, to) => {
                  const newItems = [...cvData.projects];
                  const [removed] = newItems.splice(from, 1);
                  newItems.splice(to, 0, removed);
                  cvData.projects = newItems;
                }}
              />
            </div>
          </Card>

        {:else if activeSection === 'certifications'}
          <Card>
            <div class="p-6">
              <CertificationForm
                items={cvData.certifications}
                onUpdate={(items) => cvData.certifications = items}
                onReorder={(from, to) => {
                  const newItems = [...cvData.certifications];
                  const [removed] = newItems.splice(from, 1);
                  newItems.splice(to, 0, removed);
                  cvData.certifications = newItems;
                }}
              />
            </div>
          </Card>

        {:else if activeSection === 'template'}
          <Card>
            <div class="p-6 space-y-6">
              <h2 class="text-lg font-semibold flex items-center gap-2">
                <Layout class="h-5 w-5" />
                Template & Theme
              </h2>
              <div class="grid gap-4 md:grid-cols-3">
                {#each ['modern', 'classic', 'minimal'] as template}
                  <div class={cn('p-4 border-2 rounded-xl cursor-pointer transition-all relative', cvData.template === template ? 'border-primary-500 bg-primary-50 dark:bg-primary-900/20' : 'border-secondary-200 dark:border-secondary-700 hover:border-secondary-300')}>
                    <input type="radio" name="template" value={template} bind:group={cvData.template} class="sr-only" />
                    <div class="text-center">
                      <div class="text-capitalize font-medium mb-2">{template}</div>
                      <p class="text-sm text-secondary-600 dark:text-secondary-400">
                        {template === 'modern' && 'Clean, contemporary design with subtle accents'}
                        {template === 'classic' && 'Traditional professional layout with clear sections'}
                        {template === 'minimal' && 'Minimalist design focusing on content'}
                      </p>
                    </div>
                    {#if cvData.template === template}
                      <div class="absolute top-2 right-2 text-primary-500">
                        <Palette class="h-5 w-5" />
                      </div>
                    {/if}
                  </div>
                {/each}
              </div>
              <div class="border-t border-secondary-200 dark:border-secondary-700 pt-4">
                <h3 class="font-medium mb-3 flex items-center gap-2">
                  <Palette class="h-5 w-5" />
                  Theme
                </h3>
                <div class="flex gap-4">
                  {#each ['light', 'dark', 'auto'] as theme}
                    <label class="flex items-center gap-2 cursor-pointer">
                      <input type="radio" name="theme" value={theme} bind:group={cvData.theme} class="h-4 w-4 text-primary-600 focus:ring-primary-500" />
                      <span class="capitalize">{theme}</span>
                    </label>
                  {/each}
                </div>
              </div>
            </div>
          </Card>
        {/if}
      </div>

      <!-- Live Preview Panel -->
      {#if showPreviewPanel}
        <aside class="w-full lg:w-96 flex-shrink-0 hidden lg:block border-l border-secondary-200 dark:border-secondary-700 bg-secondary-50 dark:bg-secondary-900/50">
          <div class="h-full flex flex-col">
            <div class="px-4 py-3 border-b border-secondary-200 dark:border-secondary-700 bg-white dark:bg-secondary-900 flex items-center justify-between">
              <h3 class="font-semibold flex items-center gap-2">
                <Eye class="h-5 w-5" />
                Live Preview
              </h3>
              <span class="text-xs text-secondary-500 dark:text-secondary-400">
                {cvData.template} / {cvData.theme}
              </span>
            </div>
            <div class="flex-1 overflow-hidden min-h-0">
              <CvPreview 
                pdfData={previewPdf} 
                loading={previewLoading} 
                error={previewError} 
              />
            </div>
          </div>
        </aside>
      {/if}
    </main>
  </div>
</div>

{#if !showPreviewPanel && previewPdf}
  <Modal open={true} title="CV Preview" size="xl" on:close={() => showPreviewPanel = true}>
    <CvPreview pdfData={previewPdf} loading={false} error={null} />
  </Modal>
{/if}

{#if showImportModal}
  <Modal open={true} title="Import CV" size="lg" on:close={() => showImportModal = false}>
    <div class="p-6 space-y-6">
      <div class="flex gap-4 border-b border-secondary-200 dark:border-secondary-700 pb-4">
        <button
          class={cn('flex-1 py-2 px-4 rounded-lg font-medium transition-colors', importMethod === 'json' ? 'bg-primary-100 dark:bg-primary-900/30 text-primary-700 dark:text-primary-300' : 'text-secondary-600 hover:bg-secondary-100 dark:text-secondary-400 dark:hover:bg-secondary-800')}
          on:click={() => importMethod = 'json'}
        >
          <FileJson class="h-5 w-5 mr-2 inline-block" />
          JSON Import
        </button>
        <button
          class={cn('flex-1 py-2 px-4 rounded-lg font-medium transition-colors', importMethod === 'linkedin' ? 'bg-primary-100 dark:bg-primary-900/30 text-primary-700 dark:text-primary-300' : 'text-secondary-600 hover:bg-secondary-100 dark:text-secondary-400 dark:hover:bg-secondary-800')}
          on:click={() => importMethod = 'linkedin'}
        >
          <User class="h-5 w-5 mr-2 inline-block" />
          LinkedIn Import
        </button>
      </div>

      {#if importMethod === 'json'}
        <div class="space-y-4">
          <label class="block text-sm font-medium text-secondary-700 dark:text-secondary-300">Paste CV JSON</label>
          <textarea
            bind:value={importJsonText}
            rows="15"
            class="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg bg-white dark:bg-secondary-800 text-secondary-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent font-mono text-sm"
            placeholder={importPlaceholder}
          ></textarea>
          <p class="text-xs text-secondary-500">Export JSON from another CV editor or use the backend format</p>
        </div>
      {:else}
        <div class="space-y-4 text-center py-8">
          <User class="h-12 w-12 text-secondary-300 dark:text-secondary-600 mx-auto mb-4" />
          <p class="text-secondary-600 dark:text-secondary-400">LinkedIn import will be available in a future update.</p>
          <p class="text-sm text-secondary-500">For now, export your LinkedIn profile as PDF and manually enter the data, or use JSON import.</p>
        </div>
      {/if}

      <div class="flex justify-end gap-3 pt-4 border-t border-secondary-200 dark:border-secondary-700">
        <Button variant="secondary" on:click={() => showImportModal = false}>Cancel</Button>
        <Button variant="primary" on:click={handleImport} disabled={importLoading || (importMethod === 'json' && !importJsonText.trim())}>
          {importLoading ? 'Importing...' : 'Import'}
        </Button>
      </div>
    </div>
  </Modal>
{/if}