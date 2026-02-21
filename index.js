'use strict';

const crypto = require('crypto');

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------
const SERVICE_NAME = 'research-lab-agents';
const BACKEND_URL = process.env.RESEARCH_LAB_BACKEND_URL;
const ALLOWED_ORIGINS = process.env.ALLOWED_ORIGINS
  ? process.env.ALLOWED_ORIGINS.split(',')
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
  return ALLOWED_ORIGINS[0];
}

function setCorsHeaders(req, res) {
  const origin = resolveOrigin(req.headers.origin || '');
  res.set('Access-Control-Allow-Origin', origin);
  res.set('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
  res.set(
    'Access-Control-Allow-Headers',
    'Content-Type, Authorization, x-correlation-id, x-request-id',
  );
  res.set('Access-Control-Max-Age', '3600');
}

// ---------------------------------------------------------------------------
// Health Handler
// ---------------------------------------------------------------------------
async function handleHealth(req, res, ctx) {
  const agents = HEALTH_AGENTS.map((name) => ({ name, status: 'healthy' }));

  if (BACKEND_URL) {
    try {
      const resp = await fetch(`${BACKEND_URL}/health`, {
        signal: AbortSignal.timeout(5000),
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
    return res.status(405).json({
      error: 'Method Not Allowed',
      message: `${route.name} agent only accepts POST requests`,
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
      message: err.message,
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
    return res.status(204).send('');
  }

  const startTime = Date.now();
  const traceId = req.headers['x-correlation-id'] || crypto.randomUUID();
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
