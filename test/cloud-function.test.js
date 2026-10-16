'use strict';

const assert = require('node:assert/strict');
const path = require('node:path');
const test = require('node:test');

const modulePath = path.resolve(__dirname, '..', 'index.js');

function loadHandler({ allowedOrigins, backendUrl } = {}) {
  if (allowedOrigins === undefined) {
    delete process.env.ALLOWED_ORIGINS;
  } else {
    process.env.ALLOWED_ORIGINS = allowedOrigins;
  }
  if (backendUrl === undefined) {
    delete process.env.RESEARCH_LAB_BACKEND_URL;
  } else {
    process.env.RESEARCH_LAB_BACKEND_URL = backendUrl;
  }
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

async function invoke({
  allowedOrigins,
  backendUrl,
  origin,
  method = 'GET',
  pathName = '/missing',
  headers = {},
  body,
  rawBody,
}) {
  const handler = loadHandler({ allowedOrigins, backendUrl });
  const req = { method, path: pathName, headers: { ...headers }, body, rawBody };
  if (origin !== undefined) req.headers.origin = origin;
  const res = createResponse();
  await handler(req, res);
  return res;
}

test.afterEach(() => {
  delete process.env.ALLOWED_ORIGINS;
  delete process.env.RESEARCH_LAB_BACKEND_URL;
  delete require.cache[modulePath];
  delete global.fetch;
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

test('preflight rejects a disallowed origin', async () => {
  const res = await invoke({
    allowedOrigins: 'https://allowed.example',
    origin: 'https://attacker.example',
    method: 'OPTIONS',
  });
  assert.equal(res.statusCode, 403);
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
    headers: { 'content-type': 'application/json' },
    body: { prompt: 'test' },
  });
  assert.equal(unavailable.statusCode, 503);
  assert.equal(unavailable.body.error, 'Service Unavailable');
});

test('agent endpoints require POST and advertise the allowed method', async () => {
  const res = await invoke({
    allowedOrigins: '*',
    method: 'GET',
    pathName: '/v1/research-lab/hypothesis',
  });
  assert.equal(res.statusCode, 405);
  assert.equal(res.headers.get('allow'), 'POST');
});

test('health endpoint requires GET and advertises the allowed method', async () => {
  const res = await invoke({
    allowedOrigins: '*',
    method: 'POST',
    pathName: '/v1/research-lab/health',
  });
  assert.equal(res.statusCode, 405);
  assert.equal(res.headers.get('allow'), 'GET');
});

test('agent endpoints require a JSON media type before backend lookup', async () => {
  const res = await invoke({
    allowedOrigins: '*',
    backendUrl: 'https://backend.example',
    method: 'POST',
    pathName: '/v1/research-lab/hypothesis',
    headers: { 'content-type': 'text/plain' },
    body: { prompt: 'test' },
  });
  assert.equal(res.statusCode, 415);
  assert.equal(res.body.error, 'Unsupported Media Type');
});

test('agent endpoints reject non-object JSON payloads', async () => {
  for (const body of [null, [], 'text']) {
    const res = await invoke({
      allowedOrigins: '*',
      backendUrl: 'https://backend.example',
      method: 'POST',
      pathName: '/v1/research-lab/hypothesis',
      headers: { 'content-type': 'application/json' },
      body,
    });
    assert.equal(res.statusCode, 400);
    assert.equal(res.body.error, 'Bad Request');
  }
});

test('agent endpoints reject payloads larger than 64 KiB', async () => {
  const res = await invoke({
    allowedOrigins: '*',
    backendUrl: 'https://backend.example',
    method: 'POST',
    pathName: '/v1/research-lab/metrics',
    headers: { 'content-type': 'application/json' },
    body: { values: [] },
    rawBody: Buffer.alloc((64 * 1024) + 1),
  });
  assert.equal(res.statusCode, 413);
  assert.equal(res.body.error, 'Payload Too Large');
});

test('valid JSON is forwarded without redirects', async () => {
  let request;
  global.fetch = async (url, options) => {
    request = { url, options };
    return { ok: true, status: 200, json: async () => ({ result: 'ok' }) };
  };

  const res = await invoke({
    allowedOrigins: '*',
    backendUrl: 'https://backend.example',
    method: 'POST',
    pathName: '/v1/research-lab/hypothesis',
    headers: { 'content-type': 'application/problem+json' },
    body: { prompt: 'test' },
  });

  assert.equal(res.statusCode, 200);
  assert.equal(res.body.result, 'ok');
  assert.equal(request.url, 'https://backend.example/api/v1/agents/hypothesis');
  assert.equal(request.options.redirect, 'error');
  assert.deepEqual(JSON.parse(request.options.body), { prompt: 'test' });
});

test('unsafe correlation identifiers are not reflected or forwarded', async () => {
  let forwardedId;
  global.fetch = async (_url, options) => {
    forwardedId = options.headers['x-correlation-id'];
    return { ok: true, status: 200, json: async () => ({ result: 'ok' }) };
  };

  const unsafe = 'invalid id\r\nx-injected: true';
  const res = await invoke({
    allowedOrigins: '*',
    backendUrl: 'https://backend.example',
    method: 'POST',
    pathName: '/v1/research-lab/hypothesis',
    headers: { 'content-type': 'application/json', 'x-correlation-id': unsafe },
    body: { prompt: 'test' },
  });

  assert.notEqual(forwardedId, unsafe);
  assert.match(forwardedId, /^[0-9a-f-]{36}$/);
  assert.equal(res.body.execution_metadata.trace_id, forwardedId);
});

test('backend failures do not expose internal error details', async () => {
  global.fetch = async () => {
    throw new Error('dial tcp 10.0.0.7:5432 secret-internal-host');
  };

  const res = await invoke({
    allowedOrigins: '*',
    backendUrl: 'https://backend.example',
    method: 'POST',
    pathName: '/v1/research-lab/metrics',
    headers: { 'content-type': 'application/json' },
    body: { values: [1] },
  });

  assert.equal(res.statusCode, 502);
  assert.equal(res.body.message, 'The research backend request failed');
  assert.doesNotMatch(JSON.stringify(res.body), /10\.0\.0\.7|secret-internal-host/);
});
