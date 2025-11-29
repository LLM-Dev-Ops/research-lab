# Review Processes - Visual Workflow Diagrams

## Overall Review Decision Tree

```mermaid
graph TD
    A[Code Change Proposed] --> B{What type of change?}

    B -->|Bug Fix| C[Code Review 8.1]
    B -->|New Feature| D[Code Review 8.1 + Performance 8.4]
    B -->|New Service| E[Architecture 8.2 + Security 8.3 + Code 8.1]
    B -->|Database Schema| F[Architecture 8.2 + Code 8.1]
    B -->|New API Endpoint| G[API Review 8.5 + Code 8.1]
    B -->|API Breaking Change| H[API Review 8.5 + Architecture 8.2]
    B -->|Auth/Crypto| I[Security 8.3 + Code 8.1]
    B -->|New Dependency| J[Dependency Review 8.6]

    C --> K{Tests Pass?}
    D --> K
    E --> L{ARB Approval?}
    F --> L
    G --> K
    H --> L
    I --> M{Security Approved?}
    J --> N{License OK?}

    K -->|Yes| O[Merge to Main]
    K -->|No| P[Fix & Resubmit]

    L -->|Yes| K
    L -->|No| Q[Revise Design]

    M -->|Yes| K
    M -->|No| R[Security Remediation]

    N -->|Yes| K
    N -->|No| S[Find Alternative]

    O --> T{Ready for Release?}

    T -->|Yes| U[Pre-Production Review 8.7]
    T -->|No| V[Continue Development]

    U --> W{Go/No-Go?}

    W -->|GO| X[Deploy to Production]
    W -->|CONDITIONAL GO| Y[Phased Rollout]
    W -->|NO-GO| Z[Address Blockers]

    X --> AA[Monitor 24h]
    Y --> AA
    Z --> V
```

## Code Review Workflow (8.1)

```mermaid
sequenceDiagram
    participant Dev as Developer
    participant CI as CI/CD Pipeline
    participant Peer as Peer Reviewer
    participant TL as Tech Lead
    participant Repo as Repository

    Dev->>Dev: Run local checks (fmt, clippy, test)
    Dev->>Repo: Create Pull Request

    activate CI
    Repo->>CI: Trigger automated checks
    CI->>CI: cargo build
    CI->>CI: cargo test
    CI->>CI: cargo tarpaulin (coverage)
    CI->>CI: cargo audit
    CI->>CI: cargo-deny licenses

    alt CI Fails
        CI-->>Dev: Build/Test/Coverage Failed
        Dev->>Dev: Fix issues
        Dev->>Repo: Push fixes
    else CI Passes
        CI-->>Repo: ✓ All checks passed
        deactivate CI

        Repo->>Peer: Notify reviewer (4h SLA)

        activate Peer
        Peer->>Peer: Review against checklist

        alt Changes Requested
            Peer-->>Dev: Request changes
            Dev->>Dev: Address feedback
            Dev->>Repo: Push updates
            Repo->>Peer: Re-review (2h SLA)
        else Approved
            Peer-->>Repo: ✓ Approval
            deactivate Peer

            alt Critical Change
                Repo->>TL: Request Tech Lead review
                activate TL
                TL->>TL: Secondary review
                TL-->>Repo: ✓ Approval
                deactivate TL
            end

            Dev->>Repo: Merge (squash & merge)
            Repo->>Repo: Auto-deploy to Staging
        end
    end
```

## Architecture Review Workflow (8.2)

```mermaid
gantt
    title Architecture Review Timeline (7 Days)
    dateFormat  YYYY-MM-DD
    section Submission
    Prepare ADR, diagrams, threat model     :a1, 2025-01-01, 1d
    Submit to ARB                           :a2, after a1, 1d

    section Initial Review
    ARB members review materials            :b1, after a2, 2d
    Feedback and questions                  :b2, after b1, 1d

    section Revision
    Team addresses feedback                 :c1, after b2, 3d
    Update documentation                    :c2, after b2, 3d
    POC if needed                           :c3, after b2, 3d

    section Approval
    ARB meeting (90 min)                    :d1, after c1, 1d
    Final approval/conditions               :d2, after d1, 1d

    section Fast-Track
    Emergency submission                    :e1, 2025-01-15, 1h
    Same-day review                         :e2, after e1, 4h
    Approval                                :e3, after e2, 2h
```

## Security Review Workflow (8.3)

```mermaid
flowchart TD
    A[Security-Sensitive Change] --> B{Classify Severity}

    B -->|Critical: Auth/Crypto| C[24h SLA, 2 Reviewers]
    B -->|High: External API/Data| D[48h SLA, 1 Reviewer]
    B -->|Medium: Internal/Config| E[72h SLA, 1 Reviewer]
    B -->|Low: Docs/Non-functional| F[Standard Code Review]

    C --> G[Threat Modeling STRIDE]
    D --> G
    E --> G

    G --> H[Identify Threats]
    H --> I[Rate Risk: Likelihood × Impact]

    I --> J{Risk Level}

    J -->|Critical 8-9| K[BLOCK: Must Fix Now]
    J -->|High 6-7| L[Must Fix Before Release]
    J -->|Medium 4-5| M[Should Fix, Can Backlog]
    J -->|Low 1-3| N[Fix If Time Permits]

    K --> O[Define Mitigations]
    L --> O
    M --> O
    N --> O

    O --> P[Implement Controls]
    P --> Q[Validate: Testing + Review]

    Q --> R{All Critical/High Mitigated?}

    R -->|Yes| S[Security Approval]
    R -->|No| T[Continue Remediation]

    T --> P

    F --> U[Automated Tools Only]
    U --> V[cargo audit, gitleaks, trivy]
    V --> S
```

## Performance Review Workflow (8.4)

```mermaid
flowchart LR
    A[Code Change] --> B[Run Benchmarks]

    B --> C{Compare vs Baseline}

    C -->|p50 >+50%| D[BLOCKER: Block PR]
    C -->|p50 >+25%| E[CRITICAL: Review Required]
    C -->|p50 >+10%| F[WARNING: Document]
    C -->|p50 <+10%| G[ACCEPTABLE: Pass]

    D --> H[Profile with flamegraph]
    E --> H

    H --> I[Identify Bottleneck]
    I --> J{Can Optimize?}

    J -->|Yes| K[Optimize Code]
    J -->|No| L[Justify Trade-off]

    K --> B
    L --> M[Tech Lead Approval]

    M --> N[Document Decision]
    N --> O[Update Baseline]

    F --> P[Create Follow-up Ticket]

    G --> Q[Merge]
    O --> Q
    P --> Q
```

## Dependency Review Workflow (8.6)

```mermaid
stateDiagram-v2
    [*] --> Proposed: New dependency needed

    Proposed --> Evaluation: Start evaluation

    Evaluation --> Functionality: Check features
    Functionality --> Security: Check vulnerabilities
    Security --> Maintenance: Check activity
    Maintenance --> License: Check compatibility
    License --> CodeQuality: Check quality

    CodeQuality --> Scoring: Calculate weighted score

    Scoring --> Approved: Score ≥75
    Scoring --> Conditional: Score 60-74
    Scoring --> Rejected: Score <60

    Approved --> ConfigureMonitoring: Setup cargo-audit
    Conditional --> DefineConditions: Monitor/Contribute/Replace plan
    Rejected --> FindAlternative: Evaluate alternatives

    ConfigureMonitoring --> AddToCargo: Add to Cargo.toml
    DefineConditions --> AddToCargo
    FindAlternative --> Evaluation

    AddToCargo --> UpdateDocs: Update DEPENDENCIES.md
    UpdateDocs --> VerifyLicense: cargo-deny check
    VerifyLicense --> [*]: Complete
```

## Pre-Production Review Workflow (8.7)

```mermaid
graph TB
    A[Feature Complete on Main] --> B[Release Readiness Checklist]

    B --> C{All Blockers Passed?}

    C -->|No| D[Address Blockers]
    D --> B

    C -->|Yes| E[Deploy to Staging]

    E --> F[Staging Validation]
    F --> G{Staging Tests Pass?}

    G -->|No| H[Fix Issues]
    H --> E

    G -->|Yes| I[Schedule Go/No-Go Meeting]

    I --> J[60-min Decision Meeting]
    J --> K[Stakeholders Present:<br/>Eng Mgr, Principal Eng,<br/>SRE, QA, Security, Product]

    K --> L{Decision?}

    L -->|GO| M[Production Deployment]
    L -->|CONDITIONAL GO| N[Phased Rollout]
    L -->|NO-GO| O[Document Reasons]

    O --> P[Create Action Plan]
    P --> Q[Resolve Blockers]
    Q --> B

    M --> R[Canary Phase 1: Internal 0%]
    N --> R

    R --> S{Metrics OK?}
    S -->|No| T[Rollback]
    S -->|Yes| U[Canary Phase 2: 5%]

    U --> V{Metrics OK?}
    V -->|No| T
    V -->|Yes| W[Canary Phase 3: 25%]

    W --> X{Metrics OK?}
    X -->|No| T
    X -->|Yes| Y[Canary Phase 4: 50%]

    Y --> Z{Metrics OK?}
    Z -->|No| T
    Z -->|Yes| AA[Canary Phase 5: 100%]

    AA --> AB[Monitor 48h]
    AB --> AC[Post-Mortem Meeting]
    AC --> AD[Document Learnings]

    T --> AE[Incident Investigation]
    AE --> AF[Root Cause Analysis]
    AF --> Q
```

## Review SLA Timeline

```mermaid
gantt
    title Review SLAs by Priority
    dateFormat  HH:mm
    axisFormat %H:%M

    section Critical
    Initial Response (1h)          :crit1, 00:00, 1h
    Review & Approval (2h total)   :crit2, after crit1, 1h

    section High Priority
    Initial Response (2h)          :high1, 00:00, 2h
    Review Iterations              :high2, after high1, 2h
    Final Approval (4h total)      :high3, after high2, 0h

    section Normal
    Initial Response (4h)          :norm1, 00:00, 4h
    Review Iterations              :norm2, after norm1, 12h
    Re-review (2h)                 :norm3, after norm2, 2h
    Final Approval (24h total)     :norm4, after norm3, 6h
```

## Deployment Validation Flow

```mermaid
flowchart TD
    A[Release Approved] --> B[Deploy to Staging]

    B --> C[Smoke Tests]
    C --> D[Integration Tests]
    D --> E[Performance Tests]
    E --> F[Security Tests]
    F --> G[Monitoring Validation]

    G --> H{All Staging Validations Pass?}

    H -->|No| I[Fix Issues]
    I --> B

    H -->|Yes| J[Canary Deployment]

    J --> K[Phase 1: Internal 0%<br/>4 hours]
    K --> L{Error Rate < 1%?<br/>Latency < SLA?}

    L -->|No| M[Auto-Rollback]
    L -->|Yes| N[Phase 2: 5%<br/>12 hours]

    N --> O{Error < Baseline+1%?}

    O -->|No| M
    O -->|Yes| P[Phase 3: 25%<br/>24 hours]

    P --> Q{Metrics Stable?}

    Q -->|No| M
    Q -->|Yes| R[Phase 4: 50%<br/>24 hours]

    R --> S{No Escalations?}

    S -->|No| M
    S -->|Yes| T[Phase 5: 100%]

    T --> U[Enhanced Monitoring<br/>24 hours]
    U --> V[Daily Reviews<br/>7 days]
    V --> W[Post-Mortem Meeting<br/>Week 1]

    M --> X[Incident Report]
    X --> Y[Root Cause Analysis]
    Y --> Z[Fix & Retry]
    Z --> B
```

## Escalation Paths

```mermaid
graph LR
    A[Issue Detected] --> B{Which Review?}

    B -->|Code Review| C{SLA Exceeded?}
    B -->|Architecture| D{ARB Blocked?}
    B -->|Security| E{Critical Vuln?}
    B -->|Performance| F{Blocker Regression?}
    B -->|Deployment| G{Rollback Needed?}

    C -->|2x SLA| H[Escalate to Team Lead]
    C -->|3x SLA| I[Escalate to Eng Manager]

    D -->|Decision Delay| J[Escalate to Principal Engineer]

    E -->|Found| K[Immediate Security Team]

    F -->|Confirmed| L[Tech Lead + Performance Eng]

    G -->|Yes| M[SRE On-Call]

    H --> N{Resolved?}
    I --> N
    J --> N
    K --> N
    L --> N
    M --> N

    N -->|No| O[Executive Escalation<br/>CTO/VP Engineering]
    N -->|Yes| P[Document Resolution]
```

## Metrics Dashboard View

```
┌─────────────────────── Code Review Metrics ───────────────────────┐
│                                                                    │
│  Avg Cycle Time: 18h ────────────────── ✓ Target: <24h           │
│  First Response: 3.2h ──────────────── ✓ Target: <4h              │
│  Approval Rate: 97% ────────────────── ✓ Target: >95%             │
│  Review Iterations: 2.1 ───────────── ✓ Target: <3                │
│                                                                    │
└────────────────────────────────────────────────────────────────────┘

┌─────────────────────── Performance Metrics ───────────────────────┐
│                                                                    │
│  p50 Latency: 18ms ───────────────────── ✓ Target: <20ms          │
│  p99 Latency: 92ms ───────────────────── ✓ Target: <100ms         │
│  Throughput: 5,200 req/s ───────────── ✓ Target: >5,000           │
│  CPU Usage: 65% ──────────────────────── ✓ Target: <70%           │
│  Memory: 850MB ───────────────────────── ✓ Target: <1GB           │
│                                                                    │
└────────────────────────────────────────────────────────────────────┘

┌─────────────────────── Security Metrics ──────────────────────────┐
│                                                                    │
│  Critical Vulns: 0 ─────────────────── ✓ Target: 0                │
│  High Vulns: 0 ────────────────────── ✓ Target: 0                 │
│  Audit Pass Rate: 100% ────────────── ✓ Target: 100%              │
│  License Compliance: ✓ ────────────── ✓ All deps compliant        │
│  Secrets Detected: 0 ──────────────── ✓ Target: 0                 │
│                                                                    │
└────────────────────────────────────────────────────────────────────┘

┌─────────────────────── Deployment Metrics ────────────────────────┐
│                                                                    │
│  Deployment Frequency: 2.5/week ──── ✓ Target: ≥2/week            │
│  Lead Time: 3.2 days ────────────── ✓ Target: <5 days             │
│  Change Failure Rate: 4% ───────── ✓ Target: <5%                  │
│  MTTR: 45 minutes ──────────────── ✓ Target: <1 hour              │
│  Rollback Rate: 2% ─────────────── ✓ Target: <5%                  │
│                                                                    │
└────────────────────────────────────────────────────────────────────┘
```

---

## Diagram Usage Guide

### For Onboarding
- **Start with**: Overall Review Decision Tree
- **Then review**: Specific workflow for your role (Dev → Code Review, Arch → Architecture Review)
- **Reference**: SLA Timeline for expectations

### For Daily Work
- **Quick lookup**: Review Decision Tree to determine which reviews needed
- **Process questions**: Follow specific workflow diagram (e.g., Code Review Workflow)
- **Time planning**: Reference SLA Timeline

### For Incident Response
- **Use**: Escalation Paths diagram
- **Then**: Deployment Validation Flow if rollback needed
- **Document**: Using templates referenced in workflows

### For Leadership
- **Monitor**: Metrics Dashboard View
- **Review**: Pre-Production Review Workflow for release decisions
- **Analyze**: Deployment Validation Flow for process improvements

---

**File**: `/workspaces/llm-research-lab/review-processes-workflow-diagram.md`

**Tools for Viewing**:
- GitHub/GitLab: Renders Mermaid diagrams natively
- VS Code: Markdown Preview Mermaid Support extension
- Draw.io: Import Mermaid code
- Mermaid Live Editor: https://mermaid.live

**Integration**:
- Include in team wiki for quick reference
- Print SLA Timeline for team workspace
- Use Escalation Paths in incident runbooks
- Reference Metrics Dashboard for sprint reviews
