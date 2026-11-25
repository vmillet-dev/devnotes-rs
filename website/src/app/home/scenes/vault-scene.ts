import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';

import { part } from '../../core/browser';
import { Lang } from '../../core/lang';
import { Language } from '../../core/language';
import { Logo } from '../../shell/logo';
import { ScenePlayer } from '../scene-player';

const COMMAND = 'ssh -N -L 5433:{{db_host}}:5432 \\\n    {{user}}@bastion.internal\n\n';

const FR = {
  title: 'Tunnel vers la base de staging',
  body: `${COMMAND}# Ouvre la base de staging sur le port 5433`,
  disk: 'Sur le disque',
  library: 'Bibliothèque Travail',
  passphrase: 'Phrase de passe',
  unlock: 'Déverrouiller',
  nowhere: "Elle n'est enregistrée nulle part.",
  libraries: 'Bibliothèques',
  list: [
    { name: 'Travail', state: 'ouverte', backup: 'Sauvegardée ce matin à 08:12' },
    { name: 'Personnel', state: 'verrouillée', backup: 'Sauvegardée hier à 19:40' },
    { name: 'Client Atlas', state: 'verrouillée', backup: 'Sauvegardée lundi à 09:05' },
  ],
};

const EN: typeof FR = {
  title: 'Tunnel to the staging database',
  body: `${COMMAND}# Opens the staging database on port 5433`,
  disk: 'On disk',
  library: 'Work library',
  passphrase: 'Passphrase',
  unlock: 'Unlock',
  nowhere: 'It is stored nowhere.',
  libraries: 'Libraries',
  list: [
    { name: 'Work', state: 'open', backup: 'Backed up this morning at 08:12' },
    { name: 'Personal', state: 'locked', backup: 'Backed up yesterday at 19:40' },
    { name: 'Client Atlas', state: 'locked', backup: 'Backed up on Monday at 09:05' },
  ],
};

const TEXT: Readonly<Record<Lang, typeof FR>> = { fr: FR, en: EN };

const ALPHABET = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';
const PASSPHRASE_LENGTH = 18;

/** A fixed hash rather than `Math.random`: the build and the browser must draw the same noise. */
function noise(i: number): number {
  let x = Math.imul(i + 1, 2654435761) >>> 0;
  x ^= x >>> 13;
  x = Math.imul(x, 1274126177) >>> 0;
  return (x % 10007) / 10007;
}

/** The text as it sits on disk, `ratio` of its characters sealed. */
function seal(text: string, ratio: number, offset: number): string {
  return [...text]
    .map((char, i) =>
      char === '\n' || noise(i + offset) >= ratio
        ? char
        : (ALPHABET[Math.floor(noise(i * 31 + offset + 7) * 64)] ?? char),
    )
    .join('');
}

/** The note sealed on disk, the passphrase that opens it, then the libraries and their backups. */
@Component({
  selector: 'app-vault-scene',
  imports: [Logo],
  templateUrl: './vault-scene.html',
  styleUrl: './vault-scene.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: {
    '[attr.data-step]': 'step()',
    '[class.pressed]': 'pressed()',
  },
})
export class VaultScene {
  private readonly player = inject(ScenePlayer);
  protected readonly step = this.player.step;
  private readonly t = this.player.t;
  private readonly lang = inject(Language).lang;
  protected readonly text = computed(() => TEXT[this.lang()]);

  private readonly sealed = computed(() => {
    const step = this.step();
    const ratio = step === 0 ? part(this.t(), 0.1, 0.85) : step === 1 ? 1 - part(this.t(), 0.78, 1) : 0;
    return Math.round(ratio * 50) / 50;
  });
  protected readonly title = computed(() => seal(this.text().title, this.sealed(), 0));
  protected readonly body = computed(() => seal(this.text().body, this.sealed(), 400));
  protected readonly passphrase = computed(() => {
    const step = this.step();
    if (step === 2) return '•'.repeat(PASSPHRASE_LENGTH);
    return step === 1 ? '•'.repeat(Math.round(PASSPHRASE_LENGTH * part(this.t(), 0.1, 0.65))) : '';
  });
  protected readonly pressed = computed(() => this.step() === 1 && this.t() > 0.72 && this.t() < 0.8);
}
