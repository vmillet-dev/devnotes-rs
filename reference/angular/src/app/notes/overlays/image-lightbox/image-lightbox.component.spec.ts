import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { ImageLightboxComponent } from './image-lightbox.component';

const SOURCE = 'data:image/png;base64,AAA';

describe('ImageLightboxComponent', () => {
  let fixture: ComponentFixture<ImageLightboxComponent>;

  function image(): HTMLImageElement {
    return fixture.nativeElement.querySelector('.lightbox-image');
  }

  beforeEach(async () => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [ImageLightboxComponent],
      providers: [provideTranslocoTesting()],
    });
    fixture = TestBed.createComponent(ImageLightboxComponent);
    fixture.componentRef.setInput('source', SOURCE);
    fixture.componentRef.setInput('fileName', 'capture.png');
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  it('shows the bytes it was handed rather than fetching them again', () => {
    expect(image().getAttribute('src')).toBe(SOURCE);
  });

  it('names the file, in the header and for assistive tech', () => {
    expect(fixture.nativeElement.querySelector('.lightbox-name').textContent).toContain('capture.png');
    expect(image().getAttribute('alt')).toContain('capture.png');
  });

  it('closes on the button', async () => {
    let closed = 0;
    fixture.componentInstance.closed.subscribe(() => (closed += 1));

    fixture.nativeElement.querySelector('.lightbox-close').click();
    await fixture.whenStable();

    expect(closed).toBe(1);
  });
});
