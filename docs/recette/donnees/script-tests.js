pm.test('Le statut est 200', () => pm.response.to.have.status(200));
pm.test('La réponse est du JSON', () =>
  pm.expect(pm.response.headers.get('Content-Type')).to.include('application/json'),
);
test('La page vaut 3', () => expect(pm.response.json().args.page).toBe('3'));
pm.test('Échec voulu', () => pm.expect(pm.response.code).to.equal(201));
