# Review Processes - Quick Reference Guide

## At-a-Glance Decision Matrix

### What review do I need?

| Change Type | Required Reviews | Approval SLA | Blockers |
|-------------|-----------------|--------------|----------|
| Bug fix (non-critical path) | Code Review | 4h response | Tests passing, 1 approval |
| New feature | Code Review + Performance | 8h response | Tests, coverage ≥85%, benchmarks |
| New microservice | Architecture + Security + Code | 7 days | ARB approval, security sign-off |
| Database schema change | Architecture + Code | 7 days | ARB approval, migration tested |
| API endpoint (new) | API Review + Code | 8h response | OpenAPI docs, tests |
| API breaking change | API Review + Architecture | 7 days + 6 months | ARB approval, migration guide |
| Auth/crypto change | Security + Code | 24h response | 2 security reviewers |
| External API integration | Security + Architecture | 2-7 days | Security sign-off, ARB approval |
| New dependency | Dependency Review | Immediate | License compliance, security audit |
| Dependency major update | Dependency Review + Performance | 1-2 days | Breaking change analysis, benchmarks |
| Production release | Pre-Production (all) | 7-14 days | All checklists, go/no-go meeting |
| Hotfix | Code + Security | 1h response | Critical bug fix, expedited |

---

## Code Review (8.1) - Quick Checklist

**Before requesting review:**
- [ ] All tests passing locally
- [ ] `cargo fmt --check` clean
- [ ] `cargo clippy -- -D warnings` clean
- [ ] Coverage ≥85% (run `cargo tarpaulin`)
- [ ] No hardcoded secrets

**Review SLAs:**
- Normal: 4h initial response, <24h merge
- High: 2h initial response, <8h merge
- Critical: 1h initial response, <2h merge

**Required approvals:**
- Normal changes: 1 peer reviewer
- Critical changes: 2 reviewers (1 must be tech lead)

**Auto-escalation:** No response → 2x SLA: escalate to team lead → 3x SLA: escalate to engineering manager

---

## Architecture Review (8.2) - Quick Checklist

**When required:**
- [ ] New service/module
- [ ] Service boundary changes
- [ ] New external dependency
- [ ] Database schema affecting multiple services
- [ ] Security-critical changes
- [ ] SLA-affecting changes

**Before submission:**
- [ ] ADR draft written
- [ ] C4 diagrams created (Context, Container)
- [ ] Threat model (STRIDE) documented
- [ ] Capacity planning with cost estimates
- [ ] Alternatives considered and documented

**ARB Members (quorum = 4):**
- Principal Engineer (required)
- Security Engineer (required)
- SRE Representative (required)
- Tech Lead (required)
- Domain experts (as needed)

**Timeline:** 7 days total (2 days initial review → 3 days revision → 1 day meeting → 1 day approval)

**Scoring:** Need ≥75 points to approve (90-100=Excellent, 75-89=Good, 60-74=Acceptable, <60=Needs revision)

---

## Security Review (8.3) - Quick Checklist

**Classification:**
- **Critical** (24h SLA, 2 reviewers): Auth, crypto, data protection
- **High** (48h SLA, 1 reviewer): External APIs, data handling
- **Medium** (72h SLA, 1 reviewer): Internal services, config
- **Low** (N/A): Docs, non-functional

**Mandatory automated checks:**
- [ ] `cargo audit` (no critical/high vulns)
- [ ] `cargo-deny check licenses` (passing)
- [ ] `trivy image <image>` (0 critical, 0 high)
- [ ] `gitleaks detect` (no secrets)

**Threat modeling (STRIDE):**
- [ ] Spoofing (authentication)
- [ ] Tampering (integrity)
- [ ] Repudiation (audit logs)
- [ ] Information Disclosure (encryption)
- [ ] Denial of Service (rate limiting)
- [ ] Elevation of Privilege (authorization)

**Risk rating:** Critical (8-9) = block release, High (6-7) = must fix, Medium (4-5) = should fix, Low (1-3) = optional

---

## Performance Review (8.4) - Quick Checklist

**Benchmark targets:**
- [ ] p50 latency <20ms
- [ ] p95 latency <50ms
- [ ] p99 latency <100ms
- [ ] Throughput >5000 req/s per instance
- [ ] CPU <70% at peak load
- [ ] Memory <1GB per instance at peak
- [ ] No memory leaks (soak test)

**Regression thresholds (vs baseline):**
- **Blocker**: p50 >+50%, p99 >+100%, throughput <-30%, memory >+100%
- **Critical**: p50 >+25%, p99 >+50%, throughput <-15%, memory >+50%
- **Warning**: p50 >+10%, p99 >+20%, throughput <-5%, memory >+20%
- **Acceptable**: All below warning thresholds

**Required tests:**
- [ ] `cargo bench` (Criterion.rs) - run on every PR
- [ ] Load test (k6) - 1000 users, 10 min
- [ ] Stress test (k6) - 5000 users, 10 min
- [ ] Soak test (k6) - 2000 users, 2 hours

**On regression:**
1. Run profiler (flamegraph, perf)
2. Identify bottleneck
3. Optimize or justify trade-off
4. Re-run benchmarks
5. Update baseline if intentional

---

## API Review (8.5) - Quick Checklist

**REST API design:**
- [ ] Resources are nouns (not verbs)
- [ ] Proper HTTP methods (GET, POST, PUT, DELETE)
- [ ] Consistent response structure (data, meta, errors)
- [ ] Appropriate status codes
- [ ] Pagination metadata included
- [ ] API versioning (/v1/)
- [ ] Rate limiting configured
- [ ] CORS properly configured

**Documentation:**
- [ ] OpenAPI/Swagger spec complete (utoipa)
- [ ] Request/response examples for each endpoint
- [ ] Error responses documented
- [ ] Authentication flow documented

**Breaking change? (if YES, see below)**
- Removing endpoint/parameter/field
- Changing field type
- Adding required parameter
- Changing HTTP method/status code

**Breaking change process:**
1. Get ARB approval (Principal Engineer, Tech Lead, Product Manager)
2. Write migration guide
3. Communicate 6 months in advance
4. Add deprecation warnings
5. Monitor usage
6. Remove when usage <1%

---

## Dependency Review (8.6) - Quick Checklist

**Before adding dependency:**
- [ ] Functionality: Does it solve our problem completely?
- [ ] Security: Any known CVEs? (`cargo audit`)
- [ ] Maintenance: Active development? (commits last year >50)
- [ ] License: Compatible? (MIT, Apache-2.0, BSD preferred)
- [ ] Code Quality: Test coverage >70%, good docs?
- [ ] Score ≥75 points (see evaluation criteria)

**License compliance:**
- [ ] `cargo-deny check licenses` passing
- [ ] No GPL/AGPL licenses
- [ ] THIRD_PARTY_LICENSES.md updated

**Approval required:**
- Low risk (dev deps): Any team member
- Medium risk (app deps): Tech lead
- High risk (core, database, security): Principal engineer + Security team

**Monitoring:**
- [ ] `cargo audit` daily (automated)
- [ ] `cargo-outdated` weekly
- [ ] Quarterly dependency health review

---

## Pre-Production Review (8.7) - Quick Checklist

**Code Quality:**
- [ ] All code merged to main
- [ ] Zero compiler warnings
- [ ] Zero clippy warnings
- [ ] Coverage ≥85%
- [ ] All unsafe blocks audited

**Testing:**
- [ ] Unit tests passing
- [ ] Integration tests passing
- [ ] E2E tests passing
- [ ] Load test successful (10k req/s, p99 <100ms)
- [ ] Soak test successful (2h, no leaks)

**Security:**
- [ ] Security audit completed
- [ ] Zero critical/high vulnerabilities
- [ ] Penetration testing (major releases only)
- [ ] Secrets scanning clean
- [ ] License compliance verified

**Documentation:**
- [ ] Release notes drafted (CHANGELOG.md)
- [ ] API documentation up to date
- [ ] User documentation updated
- [ ] Runbooks updated
- [ ] Migration guide (if breaking changes)

**Infrastructure:**
- [ ] Database migrations tested
- [ ] Rollback plan documented and tested
- [ ] Monitoring dashboards configured
- [ ] Alerts configured
- [ ] Capacity planning validated

**Deployment:**
- [ ] Staging deployment successful
- [ ] Canary deployment plan ready
- [ ] Feature flags configured (if needed)
- [ ] Blue-green infrastructure ready (major releases)

**Go/No-Go Decision:**
- [ ] 60-min meeting scheduled
- [ ] All stakeholders present (Eng Mgr, Principal Eng, SRE, QA, Security, Product)
- [ ] Decision: GO / CONDITIONAL GO / NO-GO

**Automatic NO-GO triggers:**
- Any critical bug open
- Any critical security vulnerability
- Code coverage <85%
- Any test suite failing
- Performance regression >threshold
- No rollback plan
- Security audit not completed

---

## Deployment Validation (8.7) - Quick Reference

### Staging Validation
1. Deploy using production process
2. Smoke tests (health checks, auth, CRUD, background jobs)
3. Integration tests (E2E workflows)
4. Performance validation (load test)
5. Security validation (OWASP ZAP)
6. Monitoring validation (dashboards, alerts, logs)

### Canary Deployment (5 phases)
| Phase | Traffic | Duration | Rollback Trigger |
|-------|---------|----------|------------------|
| 1. Internal | 0% (header only) | 4h | Error rate >1%, p99 >200ms |
| 2. Small | 5% | 12h | Error >baseline+1%, latency >+20% |
| 3. Medium | 25% | 24h | Same as phase 2 |
| 4. Large | 50% | 24h | Same as phase 2 |
| 5. Full | 100% | Permanent | Monitor 48h |

**Automated rollback:** Error rate >baseline+2% for 5min OR p99 >150ms for 10min → instant rollback

### Post-Deployment Monitoring
- **First hour**: Continuous monitoring
- **Hours 1-6**: Every 30 minutes
- **Hours 6-24**: Every 2 hours
- **Days 1-7**: Daily review
- **Week 1**: Post-mortem meeting

---

## Emergency Procedures

### Fast-Track Architecture Review
**When:** Production incident, customer escalation, security vulnerability

**Timeline:** Same day initial review → Day 1 approval

**Requirements:**
- Principal Engineer approval
- Written justification
- Post-implementation review required

### Critical Hotfix Process
**Code Review SLA:** 1 hour

**Security Review SLA:** 24 hours (if applicable)

**Deployment:** Skip canary, go straight to blue-green with 5-min validation

**Post-Hotfix:** Mandatory post-mortem within 48h

---

## Contact & Escalation

### Review Bottlenecks
- **Code Review >2x SLA**: Escalate to Team Lead
- **Architecture Review delayed**: Escalate to Principal Engineer
- **Security Review blocked**: Escalate to Security Lead
- **Go/No-Go contentious**: Escalate to Engineering Manager + CTO

### Tools & Resources
- **Code Quality**: `cargo fmt`, `cargo clippy`, `cargo tarpaulin`
- **Security**: `cargo audit`, `cargo-deny`, `trivy`, `gitleaks`
- **Performance**: `cargo bench`, `k6`, `flamegraph`
- **API**: `utoipa` (OpenAPI), Swagger UI
- **Dependencies**: `cargo-outdated`, `cargo-about`, `cargo-sbom`

### Documentation
- **Full Spec**: `/workspaces/llm-research-lab/review-processes-section8.md`
- **Summary**: `/workspaces/llm-research-lab/review-processes-summary.md`
- **This Guide**: `/workspaces/llm-research-lab/review-processes-quick-reference.md`

---

**Pro Tips:**
1. Run all automated checks locally before requesting review
2. Write comprehensive PR descriptions (context, testing, risks)
3. Tag reviewers explicitly and set priority labels
4. Use draft PRs for early feedback on architecture
5. Keep PRs small (<500 lines) for faster review
6. Address all blocker comments before re-requesting review
7. Thank your reviewers (builds good team culture)

---

Last Updated: 2025-11-28
