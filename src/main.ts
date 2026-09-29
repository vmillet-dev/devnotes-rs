import { bootstrapApplication } from '@angular/platform-browser';
import { AppComponent } from './app/app.component';
import { appConfig } from './app/app.config';

bootstrapApplication(AppComponent, appConfig).catch((err) => console.error(err));

const cspWindow = window as unknown as Record<string, unknown>;
document.addEventListener(
  'securitypolicyviolation',
  (e) => (cspWindow['cspViolation'] = `${e.violatedDirective} ${e.blockedURI}`),
);
try {
  cspWindow['cspProbe'] = `new Function allowed: ${new Function('return 1')()}`;
} catch (e) {
  cspWindow['cspProbe'] = `refused: ${String(e)}`;
}

let spikeWorker: Worker | undefined;
let spikeId = 0;
(window as unknown as Record<string, unknown>)['prettierSpike'] = (text: string, parser: string) => {
  const started = performance.now();
  spikeWorker ??= new Worker(new URL('./app/core/services/format/prettier.worker', import.meta.url), {
    type: 'module',
  });
  const id = ++spikeId;
  return new Promise((resolve) => {
    const worker = spikeWorker!;
    const onMessage = (event: MessageEvent<{ id: number }>) => {
      if (event.data.id !== id) return;
      worker.removeEventListener('message', onMessage);
      resolve({ ...event.data, totalMs: performance.now() - started });
    };
    worker.addEventListener('message', onMessage);
    worker.addEventListener('error', (e) => resolve({ ok: false, message: `worker error: ${e.message}` }), {
      once: true,
    });
    worker.postMessage({ id, text, parser });
  });
};
