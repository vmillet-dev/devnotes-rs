import { ApplicationRef, EnvironmentInjector, Injector, createComponent } from '@angular/core';
import {
  SearchQuery,
  closeSearchPanel,
  findNext,
  findPrevious,
  getSearchQuery,
  replaceAll,
  replaceNext,
  setSearchQuery,
} from '@codemirror/search';
import { EditorView, Panel, ViewUpdate, runScopeHandlers } from '@codemirror/view';
import { SearchBarComponent } from '../search/search-bar/search-bar.component';
import { SearchChange, SearchTarget } from '../search/search-target';
import { countMatches, firstMatchFrom, showReplace, specOf } from './search-query';

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
    bar.setInput('target', codeMirrorSearch(view));
    app.attachView(bar.hostView);
    bar.changeDetectorRef.detectChanges();
    return {
      dom: host,
      top: true,
      mount: () => bar.instance.mounted(),
      update: (update) => bar.instance.follow(changeOf(update)),
      destroy: () => {
        app.detachView(bar.hostView);
        bar.destroy();
      },
    };
  };
}

/** The bar's questions, answered by `@codemirror/search`: its query, highlighting and replacing. */
export function codeMirrorSearch(view: EditorView): SearchTarget {
  return {
    size: () => view.state.doc.length,
    caret: () => view.state.selection.main.from,
    query: () => specOf(getSearchQuery(view.state)),
    setQuery: (spec) => view.dispatch({ effects: setSearchQuery.of(new SearchQuery(spec)) }),
    seek: (spec, origin) => {
      const match = firstMatchFrom(new SearchQuery(spec), view.state, origin);
      if (!match) return;
      view.dispatch({
        selection: { anchor: match.from, head: match.to },
        effects: EditorView.scrollIntoView(match.from, { y: 'center' }),
        userEvent: 'select.search',
      });
    },
    count: () => countMatches(getSearchQuery(view.state), view.state),
    next: () => void findNext(view),
    previous: () => void findPrevious(view),
    replaceOne: () => void replaceNext(view),
    replaceAll: () => void replaceAll(view),
    close: () => void closeSearchPanel(view),
    handleKey: (event) => runScopeHandlers(view, event, 'search-panel'),
  };
}

function changeOf(update: ViewUpdate): SearchChange {
  let query: SearchChange['query'];
  let replace = false;
  for (const transaction of update.transactions) {
    for (const effect of transaction.effects) {
      if (effect.is(setSearchQuery)) query = specOf(effect.value);
      if (effect.is(showReplace)) replace = true;
    }
  }
  const edited = update.docChanged || update.selectionSet;
  return query ? { query, replace, edited } : { replace, edited };
}
