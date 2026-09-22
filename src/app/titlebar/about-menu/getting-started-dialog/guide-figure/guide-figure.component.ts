import { ChangeDetectionStrategy, Component, input } from '@angular/core';
import { GuideChapter } from '@core/services/help/guide.model';

/**
 * A schematic of the screen the chapter is about.
 *
 * ⚠️ Inline SVG and not an image: the CSP is `script-src 'self'` with nothing remote, and a
 * PNG would need one file per theme. These are drawn from the palette's own custom
 * properties, so they follow the theme for free and cost nothing over the wire.
 *
 * ⚠️ Schematic on purpose — boxes where the cards are, a bar where the rail is. A screenshot
 * would be a fourth thing to keep in step with the interface, and it is the *arrangement*
 * these have to show, not the pixels.
 */
@Component({
  selector: 'app-guide-figure',
  templateUrl: './guide-figure.component.html',
  styleUrl: './guide-figure.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class GuideFigureComponent {
  readonly chapter = input.required<GuideChapter>();
}
