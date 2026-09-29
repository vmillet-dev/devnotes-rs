import { FormatRequest } from './format.model';
import { loadPlugins } from './prettier-plugins';
import { runPrettier } from './prettier-runner';

addEventListener('message', (event: MessageEvent<{ id: number; request: FormatRequest }>) => {
  const { id, request } = event.data;
  void runPrettier(request, loadPlugins).then((answer) => postMessage({ id, answer }));
});
