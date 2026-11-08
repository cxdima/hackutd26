import { Component, computed, input } from '@angular/core';
import qrcode from 'qrcode-generator';
import { isE164 } from '../../core/format';

/**
 * A QR code that opens a tel: link to FirstRing's own inbound number. It
 * encodes only "tel:+<digits>": never a token, dashboard URL or personal number.
 */
@Component({
  selector: 'app-phone-qr',
  template: `
    @if (path(); as p) {
      <svg
        [attr.viewBox]="'0 0 ' + p.size + ' ' + p.size"
        class="block size-full"
        role="img"
        [attr.aria-label]="'QR code that dials ' + number()"
        shape-rendering="crispEdges"
      >
        <rect [attr.width]="p.size" [attr.height]="p.size" fill="#ffffff" />
        <path [attr.d]="p.d" fill="#171c18" />
      </svg>
    }
  `,
  host: { class: 'block' },
})
export class PhoneQr {
  readonly number = input.required<string>();
  /** The exact payload, exposed for tests. */
  readonly payload = computed(() => (isE164(this.number()) ? `tel:${this.number()}` : ''));
  readonly path = computed(() => {
    const payload = this.payload();
    if (!payload) return undefined;
    const qr = qrcode(0, 'M');
    qr.addData(payload);
    qr.make();
    const count = qr.getModuleCount();
    const margin = 4;
    let d = '';
    for (let row = 0; row < count; row++)
      for (let col = 0; col < count; col++)
        if (qr.isDark(row, col)) d += `M${col + margin} ${row + margin}h1v1h-1z`;
    return { d, size: count + margin * 2 };
  });
}
