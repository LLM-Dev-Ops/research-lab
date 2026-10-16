'use strict';

const crypto = require('crypto');

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------
const SERVICE_NAME = 'research-lab-agents';
const BACKEND_URL = process.env.RESEARCH_LAB_BACKEND_URL;
const ALLOWED_ORIGINS = process.env.ALLOWED_ORIGINS
  ? process.env.ALLOWED_ORIGINS.split(',').map((origin) => origin.trim()).filter(Boolean)
  : ['*'];

const AGENT_ROUTES = {
  '/v1/research-lab/hypothesis': {
    name: 'hypothesis',
    backendPath: '/api/v1/agents/hypothesis',
    layerName: 'RESEARCH_LAB_HYPOTHESIS',
  },
  '/v1/research-lab/metrics': {
    name: 'metrics',
    backendPath: '/api/v1/agents/metric',
    layerName: 'RESEARCH_LAB_METRICS',
  },
};

const HEALTH_PATH = '/v1/research-lab/health';
const HEALTH_AGENTS = ['hypothesis', 'metrics'];
const BACKEND_TIMEOUT_MS = 110_000; // slightly under the 120s Cloud Function timeout
const MAX_BODY_BYTES = 64 * 1024;
const SAFE_REQUEST_ID = /^[A-Za-z0-9._:-]{1,128}$/;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
function buildExecutionMetadata(traceId, executionId) {
  return {
    trace_id: traceId,
    timestamp: new Date().toISOString(),
    service: SERVICE_NAME,
    execution_id: executionId,
  };
}

function resolveOrigin(reqOrigin) {
  if (ALLOWED_ORIGINS.includes('*')) return '*';
  if (reqOrigin && ALLOWED_ORIGINS.includes(reqOrigin)) return reqOrigin;
  return undefined;
}

function setCorsHeaders(req, res) {
  const origin = resolveOrigin(req.headers.origin || '');
  if (!ALLOWED_ORIGINS.includes('*')) {
    res.set('Vary', 'Origin');
  }
  if (origin) {
    res.set('Access-Control-Allow-Origin', origin);
  }
  res.set('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
  res.set(
    'Access-Control-Allow-Headers',
    'Content-Type, Authorization, x-correlation-id, x-request-id',
  );
  res.set('Access-Control-Max-Age', '3600');
}

function isTrustedOrigin(req) {
  const origin = req.headers.origin || '';
  return !origin || ALLOWED_ORIGINS.includes('*') || ALLOWED_ORIGINS.includes(origin);
}

function isJsonContentType(value = '') {
  const mediaType = value.split(';', 1)[0].trim().toLowerCase();
  return mediaType === 'application/json' || mediaType.endsWith('+json');
}

function requestBodyBytes(req) {
  if (Buffer.isBuffer(req.rawBody)) return req.rawBody.length;

  const declared = Number(req.headers['content-length']);
  if (Number.isSafeInteger(declared) && declared >= 0) return declared;

  if (req.body === undefined) return 0;
  return Buffer.byteLength(JSON.stringify(req.body), 'utf8');
}

function isObjectPayload(body) {
  return body !== null && typeof body === 'object' && !Array.isArray(body);
}

function requestId(value) {
  return typeof value === 'string' && SAFE_REQUEST_ID.test(value)
    ? value
    : crypto.randomUUID();
}

function methodNotAllowed(res, allowed, message, ctx) {
  res.set('Allow', allowed);
  return res.status(405).json({
    error: 'Method Not Allowed',
    message,
    execution_metadata: buildExecutionMetadata(ctx.traceId, ctx.executionId),
    layers_executed: [
      {
        layer: 'AGENT_ROUTING',
        status: 'failed',
        duration_ms: Date.now() - ctx.startTime,
      },
    ],
  });
}

// ---------------------------------------------------------------------------
// Health Handler
// ---------------------------------------------------------------------------
async function handleHealth(req, res, ctx) {
  if (req.method !== 'GET') {
    return methodNotAllowed(res, 'GET', 'Health endpoint only accepts GET requests', ctx);
  }

  const agents = HEALTH_AGENTS.map((name) => ({ name, status: 'healthy' }));

  if (BACKEND_URL) {
    try {
      const resp = await fetch(`${BACKEND_URL}/health`, {
        signal: AbortSignal.timeout(5000),
        redirect: 'error',
      });
      if (!resp.ok) {
        agents.forEach((a) => { a.status = 'degraded'; });
      }
    } catch {
      agents.forEach((a) => { a.status = 'degraded'; });
    }
  }

  const overall = agents.every((a) => a.status === 'healthy') ? 'healthy' : 'degraded';

  res.status(overall === 'healthy' ? 200 : 503).json({
    status: overall,
    service: SERVICE_NAME,
    agents,
    execution_metadata: buildExecutionMetadata(ctx.traceId, ctx.executionId),
    layers_executed: [
      {
        layer: 'AGENT_ROUTING',
        status: 'completed',
        duration_ms: Date.now() - ctx.startTime,
      },
    ],
  });
}

// ---------------------------------------------------------------------------
// Agent Proxy Handler
// ---------------------------------------------------------------------------
async function handleAgent(req, res, route, ctx) {
  if (req.method !== 'POST') {
    return methodNotAllowed(
      res,
      'POST',
      `${route.name} agent only accepts POST requests`,
      ctx,
    );
  }

  if (!isJsonContentType(req.headers['content-type'])) {
    return res.status(415).json({
      error: 'Unsupported Media Type',
      message: 'Content-Type must be application/json',
      execution_metadata: buildExecutionMetadata(ctx.traceId, ctx.executionId),
    });
  }

  if (requestBodyBytes(req) > MAX_BODY_BYTES) {
    return res.status(413).json({
      error: 'Payload Too Large',
      message: `Request body exceeds the ${MAX_BODY_BYTES}-byte limit`,
      execution_metadata: buildExecutionMetadata(ctx.traceId, ctx.executionId),
    });
  }

  if (!isObjectPayload(req.body)) {
    return res.status(400).json({
      error: 'Bad Request',
      message: 'Request body must be a JSON object',
      execution_metadata: buildExecutionMetadata(ctx.traceId, ctx.executionId),
    });
  }

  if (!BACKEND_URL) {
    return res.status(503).json({
      error: 'Service Unavailable',
      message: 'Backend service URL not configured (RESEARCH_LAB_BACKEND_URL)',
      execution_metadata: buildExecutionMetadata(ctx.traceId, ctx.executionId),
      layers_executed: [
        {
          layer: 'AGENT_ROUTING',
          status: 'completed',
          duration_ms: Date.now() - ctx.startTime,
        },
        { layer: route.layerName, status: 'failed', duration_ms: 0 },
      ],
    });
  }

  const routingDuration = Date.now() - ctx.startTime;
  const agentStart = Date.now();

  try {
    const backendRes = await fetch(`${BACKEND_URL}${route.backendPath}`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'x-correlation-id': ctx.traceId,
        'x-request-id': ctx.executionId,
      },
      body: JSON.stringify(req.body),
      signal: AbortSignal.timeout(BACKEND_TIMEOUT_MS),
      redirect: 'error',
    });

    const data = await backendRes.json();
    const agentDuration = Date.now() - agentStart;

    res.status(backendRes.status).json({
      ...data,
      execution_metadata: buildExecutionMetadata(ctx.traceId, ctx.executionId),
      layers_executed: [
        { layer: 'AGENT_ROUTING', status: 'completed', duration_ms: routingDuration },
        { layer: route.layerName, status: 'completed', duration_ms: agentDuration },
      ],
    });
  } catch (err) {
    const agentDuration = Date.now() - agentStart;
    const status = err.name === 'TimeoutError' ? 504 : 502;

    res.status(status).json({
      error: status === 504 ? 'Gateway Timeout' : 'Bad Gateway',
      message: status === 504
        ? 'The research backend did not respond in time'
        : 'The research backend request failed',
      execution_metadata: buildExecutionMetadata(ctx.traceId, ctx.executionId),
      layers_executed: [
        { layer: 'AGENT_ROUTING', status: 'completed', duration_ms: routingDuration },
        { layer: route.layerName, status: 'failed', duration_ms: agentDuration },
      ],
    });
  }
}

// ---------------------------------------------------------------------------
// Cloud Function Entry Point
// ---------------------------------------------------------------------------
exports.handler = async (req, res) => {
  // CORS
  setCorsHeaders(req, res);
  if (req.method === 'OPTIONS') {
    if (!isTrustedOrigin(req)) {
      return res.status(403).json({ error: 'Forbidden', message: 'Origin is not allowed' });
    }
    return res.status(204).send('');
  }

  const startTime = Date.now();
  const traceId = requestId(req.headers['x-correlation-id']);
  const executionId = crypto.randomUUID();
  const ctx = { traceId, executionId, startTime };

  const path = req.path || '/';

  // Health
  if (path === HEALTH_PATH) {
    return handleHealth(req, res, ctx);
  }

  // Agent routing
  const route = AGENT_ROUTES[path];
  if (route) {
    return handleAgent(req, res, route, ctx);
  }

  // Not found
  res.status(404).json({
    error: 'Not Found',
    message: `Unknown route: ${path}. Available: ${Object.keys(AGENT_ROUTES).join(', ')}, ${HEALTH_PATH}`,
    execution_metadata: buildExecutionMetadata(ctx.traceId, ctx.executionId),
    layers_executed: [
      {
        layer: 'AGENT_ROUTING',
        status: 'failed',
        duration_ms: Date.now() - startTime,
      },
    ],
  });
};
