import { searchIndex } from './doc-search';
import { DocPage } from './doc.model';

/** The questions page as structured data: each section is a question, what is under it the answer. */
export function questions(page: DocPage): object {
  return {
    '@type': 'FAQPage',
    mainEntity: searchIndex([page])
      .filter(({ anchor }) => anchor)
      .map(({ heading, text }) => ({
        '@type': 'Question',
        name: heading,
        acceptedAnswer: { '@type': 'Answer', text },
      })),
  };
}
