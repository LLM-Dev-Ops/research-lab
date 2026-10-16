const fs = require('node:fs');
const path = require('node:path');
const { execFileSync } = require('node:child_process');
const repo = path.resolve(process.argv[2]);
const yamlJson = execFileSync('python3', ['-c', 'import json,yaml,sys;print(json.dumps(yaml.safe_load(open(sys.argv[1]))))', path.join(repo, '.github/workflows/ci.yml')], { encoding: 'utf8' });
const jobs = JSON.parse(yamlJson).jobs;
const aggregate = jobs['all-checks'];
const original = ['cloud-function', 'lint', 'test', 'build', 'security-audit'];
const script = aggregate.steps[0].run;
function gate(values = {}) {
  const expanded = script.replace(/\$\{\{ needs\.([\w-]+)\.result \}\}/g, (_, name) => values[name] || 'success');
  try { execFileSync('bash', ['-c', expanded], { stdio: 'pipe' }); return true; } catch { return false; }
}
const checks = [];
const check = (name, pass) => checks.push({ name, pass: Boolean(pass) });
check('Docker is an aggregate dependency', aggregate.needs.includes('docker'));
check('Coverage is an aggregate dependency', aggregate.needs.includes('coverage'));
check('Aggregate runs even after dependency failure', aggregate.if === 'always()');
check('Docker runs for develop pushes too', !jobs.docker.if);
check('Failed Docker closes gate', !gate({docker:'failure'}));
check('Skipped Docker closes gate', !gate({docker:'skipped'}));
let isolation = false;
if (fs.existsSync(path.join(repo, 'test/offline_runtime_test.py'))) {
  try {
    const result = execFileSync('python3', ['-m', 'unittest', 'discover', '-s', 'test', '-p', '*_test.py'], { cwd:repo, encoding:'utf8', stdio:'pipe' });
    isolation = true;
  } catch {}
}
check('Actual kernel isolation suite passes', isolation);
let learning = false;
if (fs.existsSync(path.join(repo, 'scripts/offline-exec.py'))) {
  if (!process.env.RUFLO_CLI || !process.env.RUFLO_MODEL_DIR) throw new Error('Set RUFLO_CLI and RUFLO_MODEL_DIR before evaluation');
  try {
    const output = execFileSync('python3', [path.join(repo,'scripts/offline-exec.py'), process.execPath, '--import', path.join(repo,'scripts/ruflo-local-model.mjs'), process.env.RUFLO_CLI, 'hooks','post-task','--task-id','task-runtime-blockers-eval','--success','true','--quality','1','--agent','codex','--task','Validate offline runtime recovery','--store-results','--format','json'], { cwd:repo, encoding:'utf8', timeout:60000, env:{...process.env,RUFLO_MODEL_DIR:process.env.RUFLO_MODEL_DIR} });
    const observed = JSON.parse(output.slice(output.indexOf('{\n  "taskId"')));
    learning = observed.success && observed.learningUpdates.available && observed.learningUpdates.outcomePersisted && observed.feedback.recorded && observed.trajectory.recorded;
  } catch {}
}
check('Real local-model offline hook records feedback and trajectory', learning);
const anchor = [];
let node = false;
try { execFileSync('npm',['run','test:cf'],{cwd:repo,stdio:'pipe'}); node=true; } catch {}
anchor.push({name:'Existing 15 HTTP tests pass',pass:node});
anchor.push({name:'Every original required job remains gated',pass:original.every(n=>aggregate.needs.includes(n)&&!gate({[n]:'failure'}))});
anchor.push({name:'All-success condition passes',pass:gate()});
anchor.push({name:'Docker publication remains disabled',pass:jobs.docker.steps.some(s=>s.with?.push===false)});
console.log(JSON.stringify({holdout:{checks},anchor:{checks:anchor}}));
