const fs=require('node:fs'),cp=require('node:child_process'),path=require('node:path'),crypto=require('node:crypto');
const {pathToFileURL}=require('node:url');
async function main(){
 const [base,head,tools,out]=process.argv.slice(2).map(p=>path.resolve(p));
 const fw=await import(pathToFileURL(path.join(tools,'node_modules/@metaharness/flywheel/dist/index.js')));
 const rev=r=>cp.execFileSync('git',['rev-parse','HEAD'],{cwd:r,encoding:'utf8'}).trim();
 // Node's default stdin 'pipe' is a socketpair on this Linux runtime.
 // These tests require no input; explicitly use /dev/null rather than grant it
 // an inherited writable socket. Python capture_output still uses real pipes.
 const run=(root,bin,args)=>cp.spawnSync(bin,args,{cwd:root,encoding:'utf8',timeout:20000,stdio:['ignore','pipe','pipe']});
 const probe=root=>{const r=run(head,'python3',['scripts/probe-offline-stdio.py',root]);if(![0,1].includes(r.status))throw Error(r.stderr);return JSON.parse(r.stdout)};
 const anchors=root=>{const p=run(root,'python3',['-m','unittest','discover','-s','test','-p','offline_runtime_test.py']),n=run(root,process.execPath,['--test','--test-reporter=tap','test/cloud-function.test.js']);if(p.status||n.status)throw Error(p.stderr+n.stderr);return {pythonExit:p.status,python:p.stderr,nodeExit:n.status,node:n.stdout}};
 // Existing assertions/locks must be byte-identical. No regrading of tests.
 for(const file of ['test/offline_runtime_test.py','test/cloud-function.test.js','Cargo.lock','package-lock.json'])if(!fs.readFileSync(path.join(base,file)).equals(fs.readFileSync(path.join(head,file))))throw Error('Original modified: '+file);
 const trials=[];for(let i=0;i<3;i++)for(const root of i%2?[head,base]:[base,head])trials.push({trial:i,revision:rev(root),...probe(root)});
 const tests={};for(const root of [base,head])tests[rev(root)]=anchors(root);
 const parentBytes=cp.execFileSync('git',['show',rev(base)+':scripts/offline-exec.py'],{cwd:base,encoding:'utf8'});
 const inverse={path:'scripts/offline-exec.py',parentBytes,sha256:crypto.createHash('sha256').update(parentBytes).digest('hex')};
 const result=await fw.runFlywheelGenerations({rootPolicy:{revision:rev(base)},proposer:async()=>({value:rev(head),summary:'Refuse socket-backed stdio before offline task execution',inverse}),evaluator:async(policy,suite)=>{const root=policy.revision===rev(base)?base:head;const wins=suite.id==='holdout'?probe(root).cases.map(c=>c.pass):[anchors(root).pythonExit===0,anchors(root).nodeExit===0];return {primary:wins.filter(Boolean).length/wins.length,itemWins:wins,noopRate:0,costPerWin:0,regressed:true}},holdout:{id:'holdout',items:['stdin-direct','stdin-child','stdout-direct','stdout-child','stderr-direct','stderr-child']},anchor:{id:'anchors',items:['original-kernel-suite','original-node-suite']},maxGenerations:1,signer:fw.makeSigner(),promotionRule:fw.meetsPromotionRule,dataSource:'LIVE_LOCAL_SOCKETPAIR'});
 const gate=fw.gateFingerprint(fw.meetsPromotionRule),replay=fw.verifyReplayBundle(result.replayBundle,{pinnedGateFingerprint:gate});if(!replay.pass||result.promotions.length)throw Error('Unexpected promotion/replay');
 fs.writeFileSync(out,JSON.stringify({trials,tests,inverse,result,replay,gate,boundary:'regressed=true is an explicit full-promotion veto: current Rust/security/CI and real Ruflo execution/learning are not cleared. It is not a newly regressed test. cost/noop0 neutral, unmeasured. Root-only replay is not candidate acceptance. Local socketpair only; no model/deployed integration claim.'},null,2));console.log(JSON.stringify({promotions:0,replay}));
}main().catch(e=>{console.error(e);process.exitCode=1});
