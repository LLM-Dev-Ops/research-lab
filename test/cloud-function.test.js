'use strict';

const assert = require('node:assert/strict');
const path = require('node:path');
const test = require('node:test');

const modulePath = path.resolve(__dirname, '..', 'index.js');

function loadHandler(allowedOrigins) {
  if (allowedOrigins === undefined) {
    delete process.env.ALLOWED_ORIGINS;
  } else {
    process.env.ALLOWED_ORIGINS = allowedOrigins;
  }
  delete process.env.RESEARCH_LAB_BACKEND_URL;
  delete require.cache[modulePath];
  return require(modulePath).handler;
}

function createResponse() {
  const headers = new Map();
  return {
    headers,
    statusCode: 200,
    body: undefined,
    set(name, value) {
      headers.set(name.toLowerCase(), value);
      return this;
    },
    status(code) {
      this.statusCode = code;
      return this;
    },
    json(value) {
      this.body = value;
      return this;
    },
    send(value) {
      this.body = value;
      return this;
    },
  };
}

async function invoke({ allowedOrigins, origin, method = 'GET', pathName = '/missing' }) {
  const handler = loadHandler(allowedOrigins);
  const req = { method, path: pathName, headers: {} };
  if (origin !== undefined) req.headers.origin = origin;
  const res = createResponse();
  await handler(req, res);
  return res;
}

test.afterEach(() => {
  delete process.env.ALLOWED_ORIGINS;
  delete process.env.RESEARCH_LAB_BACKEND_URL;
  delete require.cache[modulePath];
});

test('returns an exact configured origin', async () => {
  const res = await invoke({
    allowedOrigins: 'https://allowed.example,https://second.example',
    origin: 'https://second.example',
  });
  assert.equal(res.headers.get('access-control-allow-origin'), 'https://second.example');
  assert.equal(res.headers.get('vary'), 'Origin');
});

test('does not grant CORS to a disallowed origin', async () => {
  const res = await invoke({
    allowedOrigins: 'https://allowed.example,https://second.example',
    origin: 'https://attacker.example',
  });
  assert.equal(res.headers.has('access-control-allow-origin'), false);
  assert.equal(res.headers.get('vary'), 'Origin');
});

test('does not emit an allow-origin header without an Origin request header', async () => {
  const res = await invoke({
    allowedOrigins: 'https://allowed.example,https://second.example',
  });
  assert.equal(res.headers.has('access-control-allow-origin'), false);
});

test('trims configured origins before exact matching', async () => {
  const res = await invoke({
    allowedOrigins: ' https://allowed.example , https://second.example ',
    origin: 'https://allowed.example',
  });
  assert.equal(res.headers.get('access-control-allow-origin'), 'https://allowed.example');
});

test('preserves explicitly configured wildcard behavior', async () => {
  const res = await invoke({ allowedOrigins: '*', origin: 'https://any.example' });
  assert.equal(res.headers.get('access-control-allow-origin'), '*');
  assert.equal(res.headers.has('vary'), false);
});

test('preflight returns 204 while withholding CORS from a disallowed origin', async () => {
  const res = await invoke({
    allowedOrigins: 'https://allowed.example',
    origin: 'https://attacker.example',
    method: 'OPTIONS',
  });
  assert.equal(res.statusCode, 204);
  assert.equal(res.headers.has('access-control-allow-origin'), false);
});

test('unknown route and missing backend remain fail-closed', async () => {
  const notFound = await invoke({ allowedOrigins: '*', pathName: '/missing' });
  assert.equal(notFound.statusCode, 404);
  assert.equal(notFound.body.error, 'Not Found');

  const unavailable = await invoke({
    allowedOrigins: '*',
    method: 'POST',
    pathName: '/v1/research-lab/hypothesis',
  });
  assert.equal(unavailable.statusCode, 503);
  assert.equal(unavailable.body.error, 'Service Unavailable');
});
