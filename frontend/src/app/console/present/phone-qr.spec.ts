import { TestBed } from '@angular/core/testing';
import { PhoneQr } from './phone-qr';

describe('PhoneQr', () => {
  function render(number: string) {
    const fixture = TestBed.createComponent(PhoneQr);
    fixture.componentRef.setInput('number', number);
    fixture.detectChanges();
    return fixture;
  }

  it('encodes only a TEL: dial link to the inbound number', () => {
    const fixture = render('+15555550134');
    expect(fixture.componentInstance.payload()).toBe('TEL:+15555550134');
    expect(fixture.nativeElement.querySelector('svg path')).not.toBeNull();
  });

  it('draws nothing for anything that is not a phone number', () => {
    for (const value of ['Not configured', 'https://example.com/console?token=x', '']) {
      const fixture = render(value);
      expect(fixture.componentInstance.payload()).toBe('');
      expect(fixture.nativeElement.querySelector('svg')).toBeNull();
    }
  });
});
