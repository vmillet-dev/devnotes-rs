// Après la réponse : garde l'origine dans l'environnement actif.
const corps = pm.response.json();
pm.environment.set('origine', corps.origin);
console.log('origine', corps.origin);
