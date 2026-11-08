import { Routes } from '@angular/router';

export const routes: Routes = [
  {
    path: '',
    title: 'FirstRing · Before you answer, FirstRing checks',
    loadComponent: () => import('./landing/landing-page').then((m) => m.LandingPage),
  },
  {
    path: 'console',
    loadComponent: () => import('./console/console-shell').then((m) => m.ConsoleShell),
    children: [
      {
        path: '',
        title: 'Overview · FirstRing',
        loadComponent: () => import('./console/overview-page').then((m) => m.OverviewPage),
      },
      {
        path: 'calls',
        title: 'Calls · FirstRing',
        loadComponent: () => import('./console/calls-page').then((m) => m.CallsPage),
      },
      {
        path: 'calls/:id',
        title: 'Call · FirstRing',
        loadComponent: () => import('./console/calls-page').then((m) => m.CallsPage),
      },
      {
        path: 'callers',
        title: 'Callers · FirstRing',
        loadComponent: () => import('./console/callers-page').then((m) => m.CallersPage),
      },
      {
        path: 'callers/:id',
        title: 'Caller · FirstRing',
        loadComponent: () => import('./console/callers-page').then((m) => m.CallersPage),
      },
      {
        path: 'community',
        title: 'Community · FirstRing',
        loadComponent: () => import('./console/community-page').then((m) => m.CommunityPage),
      },
      {
        path: 'present',
        title: 'Presentation · FirstRing',
        loadComponent: () => import('./console/present/present-page').then((m) => m.PresentPage),
      },
      {
        path: 'settings',
        title: 'Settings · FirstRing',
        loadComponent: () => import('./console/settings-page').then((m) => m.SettingsPage),
      },
    ],
  },
  { path: '**', redirectTo: '' },
];
