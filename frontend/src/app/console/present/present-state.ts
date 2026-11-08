import { Injectable, signal } from '@angular/core';
import type { PresentSelection } from '../../core/present-selection';

/**
 * Display-only state for the projected screen. Resetting it clears what is
 * shown; it never deletes or changes a call record.
 */
@Injectable({ providedIn: 'root' })
export class PresentState {
  readonly selection = signal<PresentSelection>({
    mode: 'follow',
    pinnedId: null,
    since: Date.now(),
  });
  readonly scene = signal<'call' | 'community'>('call');
  readonly showQr = signal(false);

  follow(): void {
    this.selection.update((s) => ({ ...s, mode: 'follow', pinnedId: null }));
  }

  pin(id: string): void {
    this.selection.update((s) => ({ ...s, mode: 'pinned', pinnedId: id }));
  }

  /** Clears selection and display state. Calls that started before now count as history. */
  reset(now = Date.now()): void {
    this.selection.set({ mode: 'follow', pinnedId: null, since: now });
    this.scene.set('call');
    this.showQr.set(false);
  }
}
