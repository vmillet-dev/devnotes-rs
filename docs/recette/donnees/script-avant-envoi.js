// Avant l'envoi : ajoute un en-tête et fixe une variable lue par l'URL ({{page}}).
pm.request.headers.add({ key: 'X-Script', value: 'avant' });
pm.variables.set('page', '3');
console.log('avant envoi', pm.request.url.toString());
