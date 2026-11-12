pm.sendRequest('https://httpbin.org/uuid', (err, res) => {
  console.log('uuid reçu', res.json().uuid);
});
