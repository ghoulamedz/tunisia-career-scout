// Route definitions for file-based routing

import Dashboard from '$routes/Dashboard.svelte';
import Sources from '$routes/Sources.svelte';
import SourceForm from '$routes/SourceForm.svelte';
import SourceDetail from '$routes/SourceDetail.svelte';
import Postings from '$routes/Postings.svelte';
import PostingDetail from '$routes/PostingDetail.svelte';
import CVEditor from '$routes/CVEditor.svelte';
import CVList from '$routes/CVList.svelte';
import CVPreview from '$routes/CVPreview.svelte';
import CVTemplates from '$routes/CVTemplates.svelte';
import Matches from '$routes/Matches.svelte';
import Settings from '$routes/Settings.svelte';

export interface Route {
  path: string;
  component: any;
  title: string;
  icon?: string;
}

export const routes: Route[] = [
  { path: '/', component: Dashboard, title: 'Dashboard', icon: 'layout-dashboard' },
  { path: '/sources', component: Sources, title: 'Sources', icon: 'database' },
  { path: '/sources/new', component: SourceForm, title: 'Add Source', icon: 'plus' },
  { path: '/sources/:id', component: SourceDetail, title: 'Source Detail', icon: 'eye' },
  { path: '/sources/edit/:id', component: SourceForm, title: 'Edit Source', icon: 'edit' },
  { path: '/postings', component: Postings, title: 'Job Postings', icon: 'briefcase' },
  { path: '/postings/:id', component: PostingDetail, title: 'Job Detail', icon: 'file-text' },
  { path: '/cv', component: CVList, title: 'CV Profiles', icon: 'file-text' },
  { path: '/cv/new', component: CVEditor, title: 'New CV', icon: 'plus' },
  { path: '/cv/edit/:id', component: CVEditor, title: 'Edit CV', icon: 'edit' },
  { path: '/cv/preview', component: CVPreview, title: 'CV Preview', icon: 'eye' },
  { path: '/cv/templates', component: CVTemplates, title: 'Templates', icon: 'layout' },
  { path: '/matches', component: Matches, title: 'Matches', icon: 'target' },
  { path: '/settings', component: Settings, title: 'Settings', icon: 'settings' },
];

// Match a URL to a route and extract params
export function getRoute(url: string): Route & { params: Record<string, string> } {
  const path = url.split('?')[0]; // Remove query string
  
  for (const route of routes) {
    const routeParts = route.path.split('/').filter(Boolean);
    const pathParts = path.split('/').filter(Boolean);
    
    if (routeParts.length !== pathParts.length) continue;
    
    const params: Record<string, string> = {};
    let match = true;
    
    for (let i = 0; i < routeParts.length; i++) {
      if (routeParts[i].startsWith(':')) {
        params[routeParts[i].slice(1)] = pathParts[i];
      } else if (routeParts[i] !== pathParts[i]) {
        match = false;
        break;
      }
    }
    
    if (match) {
      return { ...route, params };
    }
  }
  
  // Fallback to first route (Dashboard)
  return { ...routes[0], params: {} };
}