import { ApplicationRef, EnvironmentInjector, Injector, createComponent } from '@angular/core';
import { EditorView, Panel } from '@codemirror/view';
import { SearchBarComponent } from './search-bar/search-bar.component';

/**
 * CodeMirror's panel, drawn by an Angular component: its markup is ours, under our lint and our
 * translations. Attached to the application so its signals render it; destroyed with the panel.
 */
export function searchPanel(injector: Injector): (view: EditorView) => Panel {
  return (view) => {
    const app = injector.get(ApplicationRef);
    const host = document.createElement('div');
    const bar = createComponent(SearchBarComponent, {
      environmentInjector: injector.get(EnvironmentInjector),
      elementInjector: injector,
      hostElement: host,
    });
    bar.setInput('view', view);
    app.attachView(bar.hostView);
    bar.changeDetectorRef.detectChanges();
    return {
      dom: host,
      top: true,
      mount: () => bar.instance.mounted(),
      update: (update) => bar.instance.follow(update),
      destroy: () => {
        app.detachView(bar.hostView);
        bar.destroy();
      },
    };
  };
}
