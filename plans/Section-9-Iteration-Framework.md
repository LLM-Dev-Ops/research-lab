# Section 9: Iteration Framework

## 9.1 Continuous Improvement Cycles

### Sprint Structure

```yaml
# sprint_structure.yaml

sprint_cadence:
  duration: "2 weeks"

  regular_sprint:
    planning:
      duration: "2 hours"
      inputs: ["Prioritized backlog", "Team velocity", "Tech debt allocation (15-20%)"]
      outputs: ["Sprint backlog", "Capacity plan", "Risk assessment"]

    daily_standup:
      duration: "15 minutes"
      format: "Yesterday/Today/Blockers"

    refinement:
      frequency: "Mid-sprint"
      duration: "1 hour"
      scope: "Next 2-3 sprints"

    review:
      duration: "1 hour"
      demo: "Working software only"
      acceptance: "Definition of Done checklist"

    retrospective:
      duration: "1 hour"
      format: "Start/Stop/Continue"
      action_items: "Tracked in next sprint"
```

### Refinement Backlog

```yaml
# refinement_backlog.yaml

backlog_allocation:
  feature_work: "60-70%"
  technical_debt: "15-20%"
  bug_fixes: "10-15%"
  innovation: "5-10%"

estimation:
  method: "Planning poker (Fibonacci)"
  unit: "Story points"

acceptance_criteria:
  required:
    - "Testable requirements"
    - "Performance expectations"
    - "Security considerations"
    - "Rollback plan"
```

### Improvement Metrics

```yaml
# improvement_metrics.yaml

velocity_metrics:
  sprint_velocity:
    measurement: "Story points completed per sprint"
    variance_threshold: "±20%"

  cycle_time:
    target: "< 3 days (median)"
    p95_target: "< 5 days"

quality_metrics:
  code_coverage:
    target: ">= 85%"
    measurement: "Tarpaulin"

  mutation_score:
    target: ">= 70%"
    measurement: "cargo-mutants"

  defect_rate:
    target: "< 5 bugs per 100 story points"

deployment_metrics:
  deployment_frequency:
    target: "Daily to production"

  lead_time:
    target: "< 4 hours (commit to production)"

  change_failure_rate:
    target: "< 5%"

  mttr:
    target: "< 30 minutes"
```

## 9.2 Feedback Integration

### Production Telemetry Loop

```yaml
# production_telemetry.yaml

observability_stack:
  metrics:
    tool: "Prometheus"
    retention: "30 days (5m), 1 year (1h)"

    golden_signals:
      latency:
        - "HTTP p50/p95/p99"
        - "Database query latency"
        alerts:
          warning: "p95 > 100ms"
          critical: "p99 > 500ms"

      traffic:
        - "Requests per second"
        - "Active connections"
        alerts:
          warning: "RPS > 80% capacity"

      errors:
        - "HTTP 5xx rate"
        - "Database errors"
        alerts:
          warning: "5xx > 0.5%"
          critical: "5xx > 1%"

      saturation:
        - "CPU utilization"
        - "Memory utilization"
        alerts:
          warning: "CPU > 70%"
          critical: "CPU > 90%"

  logs:
    tool: "Structured logging (tracing crate)"
    format: "JSON"
    retention: "90 days"

    required_fields:
      - "request_id"
      - "user_id"
      - "service_name"
      - "environment"

  traces:
    tool: "OpenTelemetry + Jaeger"
    sampling: "100% errors, 10% success"
    retention: "7 days"

telemetry_feedback_loop:
  real_time:
    trigger: "Alert threshold breached"
    action: "PagerDuty incident + runbook"
    escalation:
      level_1: "5 min - on-call engineer"
      level_2: "15 min - tech lead"
      level_3: "30 min - engineering manager"

  daily:
    report: "Service health dashboard"
    content:
      - "SLO compliance (99.9% uptime)"
      - "Error budget remaining"
      - "Performance trends"
      - "Incident summary"

  weekly:
    meeting: "Production review (30 min)"
    agenda:
      - "Incident retrospectives"
      - "Performance trends"
      - "Capacity planning"
```

### Incident Learnings

```yaml
# incident_learnings.yaml

incident_response:
  severity_classification:
    sev1_critical:
      definition: "Service down or major functionality broken"
      response_time: "< 5 minutes"

    sev2_major:
      definition: "Degraded performance or partial outage"
      response_time: "< 15 minutes"

    sev3_minor:
      definition: "Non-critical issue or isolated errors"
      response_time: "< 1 hour"

  response_process:
    - "Assess severity and impact"
    - "Assign incident commander"
    - "Mitigation (rollback, circuit breaker)"
    - "Communicate status every 15 min"
    - "Document timeline"
    - "Postmortem within 48 hours"

postmortem_template:
  required_sections:
    - "Timeline and user impact"
    - "Root cause (5 Whys)"
    - "Resolution steps"
    - "Action items (owner + due date)"
    - "Lessons learned"

  action_item_tracking:
    tool: "GitHub Issues with 'incident-followup' label"
    priority: "High (within 2 sprints)"
    review: "Weekly in standup"

continuous_learning:
  incident_metrics:
    mttr:
      target: "< 30 minutes (median)"

    mttd:
      target: "< 5 minutes"

    recurrence_rate:
      target: "0% (same root cause within 90 days)"

  improvement_areas:
    - "Add alerts for blind spots"
    - "Automate common mitigation steps"
    - "Update runbooks with learnings"
    - "Architecture review if design flaw"
```

## 9.3 Refactoring Guidelines

### When to Refactor vs Rewrite

```yaml
# refactoring_decision_matrix.yaml

refactor_candidates:
  indicators:
    - "Cyclomatic complexity > 15"
    - "Function length > 100 lines"
    - "Code duplication (DRY violations)"
    - "Poor test coverage in critical paths"
    - "Performance bottleneck with known fix"

  approach: "Incremental refactoring"

  prerequisites:
    - "Existing test coverage >= 70%"
    - "Well-understood domain"
    - "Clear refactoring goal"
    - "Backward compatibility possible"

rewrite_candidates:
  indicators:
    - "Technology stack obsolete"
    - "Architecture fundamentally flawed"
    - "Security vulnerabilities throughout"
    - "Cost to refactor > cost to rewrite"

  approach: "Strangler pattern or justified big-bang"

  prerequisites:
    - "Business case approved"
    - "Migration plan documented"
    - "Rollback strategy defined"
    - "Success metrics identified"

decision_criteria:
  refactor_score:
    formula: "(CodeQuality × 0.3) + (TestCoverage × 0.3) + (TeamFamiliarity × 0.2) + (BusinessValue × 0.2)"
    thresholds:
      refactor: ">= 6/10"
      rewrite: "< 4/10"
      hybrid: "4-6/10 (use strangler pattern)"
```

### Safe Refactoring Patterns

```yaml
# safe_refactoring_patterns.yaml

rust_specific_patterns:
  extract_function:
    when: "Function > 50 lines or multiple responsibilities"
    example: |
      // Before: 100 lines of logic
      fn complex_handler(req: Request) -> Result<Response> { ... }

      // After: Extracted functions
      fn complex_handler(req: Request) -> Result<Response> {
          let validated = validate_request(&req)?;
          let processed = process_data(validated)?;
          build_response(processed)
      }

  introduce_type_alias:
    when: "Complex generic types repeated"
    example: |
      type UserCache = Arc<RwLock<HashMap<String, Vec<User>>>>;
      fn process(data: UserCache) {}

  replace_error_with_result:
    when: "Panic/unwrap in library code"
    example: |
      // Before: unwrap() can panic
      fn get_config() -> Config {
          std::fs::read_to_string("config.toml").unwrap()
      }

      // After: Explicit error handling
      fn get_config() -> Result<Config, ConfigError> {
          let contents = std::fs::read_to_string("config.toml")?;
          Ok(toml::from_str(&contents)?)
      }

  introduce_builder:
    when: "Struct with many optional fields"
    example: |
      let config = Config::builder()
          .host("localhost")
          .port(5432)
          .database("mydb")
          .build()?;

refactoring_workflow:
  preparation:
    - "Create refactoring branch"
    - "Ensure all tests pass"
    - "Document goal in PR description"

  execution:
    - "Make small, atomic commits"
    - "Run tests after each change"
    - "Use compiler as guide"

  validation:
    - "All tests pass"
    - "Coverage maintained or improved"
    - "Performance benchmarks unchanged (±5%)"
    - "No new clippy warnings"

automated_safety_nets:
  pre_commit:
    - "cargo fmt --check"
    - "cargo clippy -- -D warnings"
    - "cargo test (fast tests)"

  ci_checks:
    - "cargo test --all-features"
    - "cargo tarpaulin (coverage threshold)"
    - "cargo mutants (mutation testing)"
    - "cargo bench (regression detection)"

  deployment_gates:
    - "Canary deployment (10% traffic)"
    - "Error rate monitoring (< 0.1%)"
    - "Automated rollback on threshold breach"
```

## 9.4 Technical Debt Management

### Categorization

```yaml
# technical_debt_categories.yaml

debt_taxonomy:
  architecture_debt:
    severity: "High"
    examples:
      - name: "Missing service boundaries"
        impact: "Difficult to scale independently"
        effort: "3-5 sprints"

      - name: "Circular dependencies"
        impact: "Build complexity, tight coupling"
        effort: "2-3 sprints"

  code_debt:
    severity: "Medium"
    examples:
      - name: "God objects (>500 lines)"
        impact: "Hard to understand, test, modify"
        effort: "1-2 days"

      - name: "Missing error handling"
        impact: "Panics in production"
        effort: "1-2 days"

  test_debt:
    severity: "Medium-High"
    examples:
      - name: "Coverage gaps in critical paths"
        impact: "Regressions reach production"
        effort: "1-3 days"

      - name: "Flaky tests"
        impact: "CI instability"
        effort: "2-4 hours per test"

  documentation_debt:
    severity: "Low-Medium"
    examples:
      - name: "Missing API documentation"
        impact: "Onboarding friction, misuse"
        effort: "1-2 days"

      - name: "Outdated architecture diagrams"
        impact: "Incorrect mental models"
        effort: "4 hours"

  dependency_debt:
    severity: "Variable (Low to Critical)"
    examples:
      - name: "Security vulnerabilities"
        impact: "Exploitable attack surface"
        effort: "2 hours - 3 days"
        severity: "Critical"

      - name: "Deprecated dependencies"
        impact: "No security patches"
        effort: "1-5 days"
        severity: "Medium"

  performance_debt:
    severity: "Medium (unless customer-impacting)"
    examples:
      - name: "N+1 queries"
        impact: "Latency spikes under load"
        effort: "1-2 days"

      - name: "Missing database indices"
        impact: "Slow queries"
        effort: "2-4 hours"
```

### Tracking

```yaml
# technical_debt_tracking.yaml

tracking_system:
  tool: "GitHub Issues"
  labels: ["tech-debt", "debt-architecture", "debt-code", "debt-test", "debt-docs", "debt-dependency", "debt-performance"]

issue_template:
  required_fields:
    category: "Architecture | Code | Test | Documentation | Dependency | Performance"

    impact:
      scale: "1-5"
      definition: |
        5 = Critical (blocks releases, security risk)
        4 = High (significant productivity loss)
        3 = Medium (moderate friction)
        2 = Low (minor annoyance)
        1 = Trivial (nice-to-have)

    effort:
      scale: "1-5"
      definition: |
        5 = Epic (>2 sprints)
        4 = Large (1-2 sprints)
        3 = Medium (3-5 days)
        2 = Small (1-2 days)
        1 = Trivial (<1 day)

    risk_if_unaddressed: "Text description"
    proposed_solution: "High-level approach + alternatives"
    affected_components: "API | Database | Auth | Job Queue | Admin | Frontend"

review_process:
  weekly_triage:
    duration: "30 minutes"
    attendees: "Tech lead, 2-3 senior engineers"

    prioritization_formula: "Priority Score = (Impact × Risk) / Effort"
    thresholds:
      high_priority: "Score >= 4.0"
      medium_priority: "Score 2.0-3.9"
      low_priority: "Score < 2.0"

  quarterly_audit:
    analysis:
      - "Debt trend (increasing/decreasing)"
      - "Category distribution"
      - "Age of oldest debt items"
      - "Paydown velocity"

metrics_dashboard:
  total_debt_items:
    measurement: "Count by category"
    target: "Decreasing trend"

  debt_age:
    measurement: "Days since creation (median, p95)"
    target: "Median < 90 days, p95 < 180 days"

  paydown_rate:
    measurement: "Items resolved per sprint"
    target: ">= Items created per sprint"
```

### Paydown Scheduling

```yaml
# debt_paydown_scheduling.yaml

allocation_strategy:
  continuous_allocation:
    budget: "15-20% of sprint velocity"
    frequency: "Every sprint"
    selection:
      - "Highest priority score"
      - "Related to sprint work (opportunistic)"
      - "Quick wins (effort = 1, impact >= 3)"

  dedicated_sprints:
    frequency: "Every 4th sprint"
    scope: "100% focus on debt reduction"
    planning:
      - "Select debt cluster (related items)"
      - "Set measurable goal"
      - "No new features (except critical bugs)"

  opportunistic_refactoring:
    trigger: "Touching existing code for features/bugs"
    guideline: "Boy Scout Rule - leave better than found"
    allowed:
      - "Extract functions for clarity"
      - "Add missing tests"
      - "Fix clippy warnings"
    not_allowed:
      - "Large architectural changes"
      - "Rewriting unrelated code"

  emergency_debt_paydown:
    trigger:
      - "Security vulnerability discovered"
      - "Incident caused by known debt"
      - "Debt blocking critical feature"
    process:
      - "Create P0 ticket"
      - "Pull engineers from sprint"
      - "Postmortem on why debt wasn't prioritized"

capacity_planning:
  velocity_calculation:
    - "Exclude first 2 sprints (baseline)"
    - "Use rolling 6-sprint average"
    - "Adjust for holidays, PTO, oncall"

  debt_capacity:
    calculation: "Velocity × 0.18 (midpoint of 15-20%)"
    minimum: "At least 1 debt item per sprint"
    maximum: "No more than 30% (except dedicated sprints)"

success_indicators:
  leading:
    - "Debt items closed >= created per sprint"
    - "Average debt age decreasing"
    - "Code coverage trending up"

  lagging:
    - "Incident rate decreasing"
    - "Time to implement features decreasing"
    - "Developer satisfaction increasing"

reporting:
  sprint_report:
    - "Debt allocation used: X% (target 15-20%)"
    - "Items closed: X (High: Y, Medium: Z, Low: W)"
    - "Metrics improved: Coverage +2%, Complexity -5%"

  quarterly_report:
    - "Total debt trend (graph)"
    - "Category breakdown (pie chart)"
    - "ROI of debt paydown (velocity improvement)"
```

---

## Key Takeaways

### Sprint Execution
- **2-week sprints** with clear ceremonies (planning, standup, refinement, review, retro)
- **Backlog allocation**: 60-70% features, 15-20% tech debt, 10-15% bugs, 5-10% innovation
- **Metrics focus**: Velocity, cycle time, coverage, deployment frequency, MTTR

### Production Feedback
- **Golden Signals**: Latency (p95 < 100ms), Traffic, Errors (5xx < 1%), Saturation
- **Observability**: Prometheus metrics, structured logs (JSON), OpenTelemetry traces
- **Incident Response**: SEV1 < 5min, SEV2 < 15min, SEV3 < 1hr
- **Postmortems**: Blameless, 5 Whys, action items tracked to completion

### Refactoring Strategy
- **Refactor when**: Complexity > 15, test coverage >= 70%, incremental approach viable
- **Rewrite when**: Stack obsolete, architecture flawed, refactor cost > rewrite cost
- **Decision formula**: (CodeQuality × 0.3) + (TestCoverage × 0.3) + (TeamFamiliarity × 0.2) + (BusinessValue × 0.2)
- **Safety nets**: Pre-commit hooks, CI checks, canary deployments, automated rollback

### Technical Debt
- **6 Categories**: Architecture (High), Code (Medium), Test (Medium-High), Docs (Low-Medium), Dependency (Variable), Performance (Medium)
- **Tracking**: GitHub Issues with impact/effort scores (1-5 scale)
- **Prioritization**: Priority = (Impact × Risk) / Effort
- **Paydown**: 15-20% continuous allocation + dedicated sprints every 4th sprint + Boy Scout Rule
- **Success metrics**: Debt closed >= created, age decreasing, coverage increasing
