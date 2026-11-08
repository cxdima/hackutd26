import { Component, Injectable } from '@angular/core';

/**
 * The console is writable only on the presenting laptop, where the server
 * trusts its own computer. The public copy (for example firstring.life) reads
 * the same API through a read-only proxy, so every control that would change
 * something is hidden there and replaced by one quiet explanation.
 *
 * `?view=public` previews the read-only view on the laptop (it only hides
 * controls; it can never unlock anything).
 */
const LOCAL_HOSTS = new Set(['localhost', '127.0.0.1', '[::1]', '::1']);

export function isLocalHost(hostname: string): boolean {
  return LOCAL_HOSTS.has(hostname.toLowerCase());
}

export function readOnlyFor(location: Pick<Location, 'hostname' | 'search'>): boolean {
  if (new URLSearchParams(location.search).get('view') === 'public') return true;
  return !isLocalHost(location.hostname);
}

export const READ_ONLY_NOTE = 'View only. Changes are made on the presenting laptop.';

@Injectable({ providedIn: 'root' })
export class Access {
  /** True on the public copy: hide or disable everything that writes. */
  readonly readOnly =
    typeof location === 'undefined'
      ? false
      : readOnlyFor({ hostname: location.hostname, search: location.search });
  readonly canWrite = !this.readOnly;
  readonly note = READ_ONLY_NOTE;
}

/** The one quiet line shown where write controls would otherwise be. */
@Component({
  selector: 'app-read-only-note',
  template: `<p class="text-[1rem] text-muted">{{ note }}</p>`,
})
export class ReadOnlyNote {
  readonly note = READ_ONLY_NOTE;
}
