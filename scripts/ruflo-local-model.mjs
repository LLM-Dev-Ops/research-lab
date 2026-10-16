// Preload before the pinned Ruflo CLI; remote model loading stays disabled.
import { createRequire } from 'node:module';
import { readFileSync, realpathSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { createHash } from 'node:crypto';

if (!process.env.RUFLO_MODEL_DIR) throw new Error('RUFLO_MODEL_DIR must point to preinstalled models');
const manifest = JSON.parse(readFileSync(new URL('./ruflo-model-manifest.json', import.meta.url), 'utf8'));
for (const [file, expected] of Object.entries(manifest.files)) {
  const bytes = readFileSync(resolve(process.env.RUFLO_MODEL_DIR, manifest.model, file));
  if (bytes.length !== expected.bytes || createHash('sha256').update(bytes).digest('hex') !== expected.sha256) {
    throw new Error(`Local model integrity check failed: ${file}`);
  }
}
const cli = realpathSync(process.argv[1]);
const require = createRequire(pathToFileURL(cli));
const wrapper = JSON.parse(readFileSync(resolve(dirname(cli), '../package.json'), 'utf8'));
if (wrapper.name !== 'ruflo' || wrapper.version !== '3.56.3') {
  throw new Error('This offline adapter requires the verified ruflo@3.56.3 installation');
}
const cjsEntry = require.resolve('@huggingface/transformers');
const packageRoot = resolve(dirname(cjsEntry), '..');
const metadata = JSON.parse(readFileSync(resolve(packageRoot, 'package.json'), 'utf8'));
if (metadata.version !== '4.3.1') throw new Error('Expected @huggingface/transformers@4.3.1');
const esmEntry = resolve(packageRoot, metadata.exports.node.import.default);
const transformers = await import(pathToFileURL(esmEntry).href);
for (const module of [transformers, require('@huggingface/transformers')]) {
  module.env.allowRemoteModels = false;
  module.env.allowLocalModels = true;
  module.env.localModelPath = resolve(process.env.RUFLO_MODEL_DIR) + '/';
}
process.env.RUFLO_REQUIRE_REAL_EMBEDDINGS = '1';
// Exercise a real local model before recording any task. No hash fallback.
const embed = await transformers.pipeline('feature-extraction', 'Xenova/all-MiniLM-L6-v2');
const probe = await embed('offline readiness proof', { pooling: 'mean', normalize: true });
if (probe.data.length !== 384 || !Array.from(probe.data).every(Number.isFinite)) {
  throw new Error('The local embedding model did not produce a valid 384-dimensional vector');
}
console.error('Offline local MiniLM model verified: 384 dimensions; remote loading disabled');
