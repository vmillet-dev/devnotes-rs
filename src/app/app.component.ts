import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { ErrorBannerComponent } from '@banners/error-banner/error-banner.component';
import { NotesPageComponent } from '@notes/notes-page.component';
import { ToolsPageComponent } from '@tools/tools-page.component';
import { HttpPageComponent } from '@http/http-page.component';
import { StatusToastComponent } from '@banners/status-toast/status-toast.component';
import { TitlebarComponent } from '@titlebar/titlebar.component';
import { UpdatePromptComponent } from '@banners/update-prompt/update-prompt.component';
import { PassphrasePromptComponent } from './passphrase-prompt/passphrase-prompt.component';
import { VaultGateComponent } from './vault-gate/vault-gate.component';
import { AreaStore } from '@core/services/areas/area.store';
import { VaultStore } from '@core/state/vault.store';
import { AreaKeysDirective } from './area-keys.directive';

@Component({
  selector: 'app-root',
  imports: [
    TitlebarComponent,
    ErrorBannerComponent,
    StatusToastComponent,
    NotesPageComponent,
    ToolsPageComponent,
    HttpPageComponent,
    UpdatePromptComponent,
    VaultGateComponent,
    PassphrasePromptComponent,
  ],
  templateUrl: './app.component.html',
  styleUrl: './app.component.scss',
  hostDirectives: [AreaKeysDirective],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class AppComponent {
  protected readonly vault = inject(VaultStore);
  protected readonly areas = inject(AreaStore);
}
