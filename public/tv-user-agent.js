(() => {
  const userAgent =
    'Mozilla/5.0 (Linux; Android 13; Android TV; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/120.0.0.0 Safari/537.36';

  Object.defineProperty(Navigator.prototype, 'userAgent', {
    configurable: true,
    get: () => userAgent,
  });
})();
