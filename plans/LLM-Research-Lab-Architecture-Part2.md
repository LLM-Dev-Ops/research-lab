# LLM-Research-Lab Architecture - Part 2: APIs, Security & Observability

> **SPARC Phase 3: Architecture (2 of 2)**
> Part of the LLM DevOps Ecosystem

---

## 6. API Architecture

### 6.1 API Gateway Configuration

```yaml
# Kong Gateway Configuration
apiVersion: configuration.konghq.com/v1
kind: KongPlugin
metadata:
  name: rate-limiting
config:
  minute: 1000
  hour: 10000
  policy: redis
  redis_host: redis.data.svc.cluster.local
  redis_port: 6379
---
apiVersion: configuration.konghq.com/v1
kind: KongPlugin
metadata:
  name: jwt-auth
config:
  key_claim_name: kid
  claims_to_verify:
    - exp
    - nbf
  run_on_preflight: false
---
apiVersion: configuration.konghq.com/v1
kind: KongPlugin
metadata:
  name: cors
config:
  origins:
    - "*"
  methods:
    - GET
    - POST
    - PUT
    - PATCH
    - DELETE
    - OPTIONS
  headers:
    - Authorization
    - Content-Type
    - X-Request-ID
    - X-Correlation-ID
  exposed_headers:
    - X-Request-ID
    - X-Correlation-ID
    - X-RateLimit-Remaining
  credentials: true
  max_age: 3600
---
apiVersion: configuration.konghq.com/v1
kind: KongPlugin
metadata:
  name: request-transformer
config:
  add:
    headers:
      - "X-Gateway-Version:1.0"
      - "X-Forwarded-Service:research-lab"
---
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: research-lab-api
  annotations:
    konghq.com/plugins: rate-limiting,jwt-auth,cors,request-transformer
    konghq.com/protocols: https
    konghq.com/https-redirect-status-code: "308"
spec:
  ingressClassName: kong
  tls:
    - hosts:
        - api.research-lab.example.com
      secretName: research-lab-tls
  rules:
    - host: api.research-lab.example.com
      http:
        paths:
          - path: /api/v1/experiments
            pathType: Prefix
            backend:
              service:
                name: experiment-service
                port:
                  number: 8001
          - path: /api/v1/runs
            pathType: Prefix
            backend:
              service:
                name: experiment-service
                port:
                  number: 8001
          - path: /api/v1/metrics
            pathType: Prefix
            backend:
              service:
                name: metric-service
                port:
                  number: 8002
          - path: /api/v1/benchmarks
            pathType: Prefix
            backend:
              service:
                name: metric-service
                port:
                  number: 8002
          - path: /api/v1/datasets
            pathType: Prefix
            backend:
              service:
                name: dataset-service
                port:
                  number: 8003
          - path: /api/v1/workflows
            pathType: Prefix
            backend:
              service:
                name: workflow-service
                port:
                  number: 8004
          - path: /api/v1/states
            pathType: Prefix
            backend:
              service:
                name: reproducibility-service
                port:
                  number: 8005
          - path: /api/v1/certificates
            pathType: Prefix
            backend:
              service:
                name: reproducibility-service
                port:
                  number: 8005
```

### 6.2 OpenAPI Specification

```yaml
openapi: 3.1.0
info:
  title: LLM Research Lab API
  description: |
    API for the LLM Research Lab - experimental evaluation and
    reproducible research workflows for Large Language Models.
  version: 1.0.0
  contact:
    name: LLM DevOps Team
    email: support@llm-devops.io
  license:
    name: Apache 2.0
    url: https://www.apache.org/licenses/LICENSE-2.0

servers:
  - url: https://api.research-lab.example.com/api/v1
    description: Production
  - url: https://api.staging.research-lab.example.com/api/v1
    description: Staging

security:
  - bearerAuth: []

tags:
  - name: experiments
    description: Experiment management
  - name: runs
    description: Experiment run management
  - name: metrics
    description: Metric definitions and benchmarking
  - name: datasets
    description: Dataset versioning and management
  - name: workflows
    description: Workflow orchestration
  - name: reproducibility
    description: Reproducibility and state management

paths:
  /experiments:
    post:
      summary: Create experiment
      operationId: createExperiment
      tags: [experiments]
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/CreateExperimentRequest'
      responses:
        '201':
          description: Experiment created
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Experiment'
          headers:
            Location:
              description: URL of created experiment
              schema:
                type: string
        '400':
          $ref: '#/components/responses/BadRequest'
        '401':
          $ref: '#/components/responses/Unauthorized'
        '422':
          $ref: '#/components/responses/UnprocessableEntity'

    get:
      summary: List experiments
      operationId: listExperiments
      tags: [experiments]
      parameters:
        - $ref: '#/components/parameters/PageOffset'
        - $ref: '#/components/parameters/PageLimit'
        - name: status
          in: query
          schema:
            type: array
            items:
              $ref: '#/components/schemas/ExperimentStatus'
        - name: tags
          in: query
          schema:
            type: array
            items:
              type: string
        - name: owner_id
          in: query
          schema:
            type: string
            format: uuid
        - name: sort
          in: query
          schema:
            type: string
            enum: [created_at, updated_at, name]
            default: created_at
        - name: order
          in: query
          schema:
            type: string
            enum: [asc, desc]
            default: desc
      responses:
        '200':
          description: List of experiments
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ExperimentList'

  /experiments/{experimentId}:
    parameters:
      - $ref: '#/components/parameters/ExperimentId'

    get:
      summary: Get experiment
      operationId: getExperiment
      tags: [experiments]
      responses:
        '200':
          description: Experiment details
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Experiment'
        '404':
          $ref: '#/components/responses/NotFound'

    put:
      summary: Update experiment
      operationId: updateExperiment
      tags: [experiments]
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/UpdateExperimentRequest'
      responses:
        '200':
          description: Experiment updated
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Experiment'
        '404':
          $ref: '#/components/responses/NotFound'
        '409':
          $ref: '#/components/responses/Conflict'

    delete:
      summary: Delete experiment
      operationId: deleteExperiment
      tags: [experiments]
      responses:
        '204':
          description: Experiment deleted
        '404':
          $ref: '#/components/responses/NotFound'
        '409':
          description: Cannot delete - has active runs
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Error'

  /experiments/{experimentId}/runs:
    parameters:
      - $ref: '#/components/parameters/ExperimentId'

    post:
      summary: Start experiment run
      operationId: startRun
      tags: [runs]
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/StartRunRequest'
      responses:
        '201':
          description: Run started
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ExperimentRun'
        '400':
          $ref: '#/components/responses/BadRequest'
        '404':
          $ref: '#/components/responses/NotFound'
        '409':
          description: Experiment not in active state
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Error'

    get:
      summary: List runs for experiment
      operationId: listRuns
      tags: [runs]
      parameters:
        - $ref: '#/components/parameters/PageOffset'
        - $ref: '#/components/parameters/PageLimit'
        - name: status
          in: query
          schema:
            type: array
            items:
              $ref: '#/components/schemas/RunStatus'
      responses:
        '200':
          description: List of runs
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/RunList'

  /runs/{runId}:
    parameters:
      - $ref: '#/components/parameters/RunId'

    get:
      summary: Get run details
      operationId: getRun
      tags: [runs]
      responses:
        '200':
          description: Run details
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ExperimentRun'
        '404':
          $ref: '#/components/responses/NotFound'

  /runs/{runId}/metrics:
    parameters:
      - $ref: '#/components/parameters/RunId'

    post:
      summary: Log metrics
      operationId: logMetrics
      tags: [runs]
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/LogMetricsRequest'
      responses:
        '202':
          description: Metrics accepted
        '400':
          $ref: '#/components/responses/BadRequest'
        '404':
          $ref: '#/components/responses/NotFound'
        '409':
          description: Run not in running state
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Error'

    get:
      summary: Get run metrics
      operationId: getRunMetrics
      tags: [runs]
      parameters:
        - name: metric_names
          in: query
          schema:
            type: array
            items:
              type: string
        - name: start_time
          in: query
          schema:
            type: string
            format: date-time
        - name: end_time
          in: query
          schema:
            type: string
            format: date-time
      responses:
        '200':
          description: Run metrics
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/RunMetrics'

  /runs/{runId}/artifacts:
    parameters:
      - $ref: '#/components/parameters/RunId'

    post:
      summary: Upload artifact
      operationId: uploadArtifact
      tags: [runs]
      requestBody:
        required: true
        content:
          multipart/form-data:
            schema:
              type: object
              required: [name, artifact_type, file]
              properties:
                name:
                  type: string
                artifact_type:
                  $ref: '#/components/schemas/ArtifactType'
                file:
                  type: string
                  format: binary
                metadata:
                  type: string
                  description: JSON-encoded metadata
      responses:
        '201':
          description: Artifact uploaded
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ArtifactRef'
        '413':
          description: Artifact too large
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Error'

    get:
      summary: List artifacts
      operationId: listArtifacts
      tags: [runs]
      responses:
        '200':
          description: List of artifacts
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/ArtifactRef'

  /runs/{runId}/status:
    parameters:
      - $ref: '#/components/parameters/RunId'

    put:
      summary: Update run status
      operationId: updateRunStatus
      tags: [runs]
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/UpdateRunStatusRequest'
      responses:
        '200':
          description: Status updated
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ExperimentRun'
        '409':
          description: Invalid state transition
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Error'

  /runs/compare:
    post:
      summary: Compare runs
      operationId: compareRuns
      tags: [runs]
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/CompareRunsRequest'
      responses:
        '200':
          description: Comparison results
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/RunComparison'

  /benchmarks:
    post:
      summary: Submit benchmark
      operationId: submitBenchmark
      tags: [metrics]
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/BenchmarkSubmitRequest'
      responses:
        '202':
          description: Benchmark accepted
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/BenchmarkJob'

  /benchmarks/{jobId}:
    parameters:
      - name: jobId
        in: path
        required: true
        schema:
          type: string
          format: uuid

    get:
      summary: Get benchmark status
      operationId: getBenchmarkStatus
      tags: [metrics]
      responses:
        '200':
          description: Benchmark status
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/BenchmarkJob'

  /benchmarks/{jobId}/results:
    parameters:
      - name: jobId
        in: path
        required: true
        schema:
          type: string
          format: uuid

    get:
      summary: Get benchmark results
      operationId: getBenchmarkResults
      tags: [metrics]
      responses:
        '200':
          description: Benchmark results
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/BenchmarkResults'
        '202':
          description: Benchmark still running
        '404':
          $ref: '#/components/responses/NotFound'

  /datasets:
    post:
      summary: Register dataset
      operationId: registerDataset
      tags: [datasets]
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/RegisterDatasetRequest'
      responses:
        '201':
          description: Dataset registered
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Dataset'

    get:
      summary: List datasets
      operationId: listDatasets
      tags: [datasets]
      parameters:
        - $ref: '#/components/parameters/PageOffset'
        - $ref: '#/components/parameters/PageLimit'
      responses:
        '200':
          description: List of datasets
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/DatasetList'

  /datasets/{datasetId}/versions:
    parameters:
      - name: datasetId
        in: path
        required: true
        schema:
          type: string
          format: uuid

    post:
      summary: Create dataset version
      operationId: createDatasetVersion
      tags: [datasets]
      requestBody:
        required: true
        content:
          multipart/form-data:
            schema:
              type: object
              required: [file]
              properties:
                file:
                  type: string
                  format: binary
                parent_version_id:
                  type: string
                  format: uuid
                message:
                  type: string
                tags:
                  type: array
                  items:
                    type: string
      responses:
        '201':
          description: Version created
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/DatasetVersion'

  /datasets/{datasetId}/versions/{versionId}/stream:
    parameters:
      - name: datasetId
        in: path
        required: true
        schema:
          type: string
          format: uuid
      - name: versionId
        in: path
        required: true
        schema:
          type: string
          format: uuid

    get:
      summary: Stream dataset
      operationId: streamDataset
      tags: [datasets]
      parameters:
        - name: columns
          in: query
          schema:
            type: array
            items:
              type: string
        - name: limit
          in: query
          schema:
            type: integer
        - name: offset
          in: query
          schema:
            type: integer
      responses:
        '200':
          description: Dataset stream
          content:
            application/x-ndjson:
              schema:
                type: object

  /workflows:
    post:
      summary: Create workflow
      operationId: createWorkflow
      tags: [workflows]
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/CreateWorkflowRequest'
      responses:
        '201':
          description: Workflow created
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Workflow'

  /workflows/{workflowId}/runs:
    parameters:
      - name: workflowId
        in: path
        required: true
        schema:
          type: string
          format: uuid

    post:
      summary: Submit workflow run
      operationId: submitWorkflowRun
      tags: [workflows]
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/SubmitWorkflowRunRequest'
      responses:
        '202':
          description: Workflow run submitted
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/WorkflowRun'

  /workflow-runs/{runId}:
    parameters:
      - name: runId
        in: path
        required: true
        schema:
          type: string
          format: uuid

    get:
      summary: Get workflow run
      operationId: getWorkflowRun
      tags: [workflows]
      responses:
        '200':
          description: Workflow run details
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/WorkflowRun'

  /workflow-runs/{runId}/pause:
    parameters:
      - name: runId
        in: path
        required: true
        schema:
          type: string
          format: uuid

    post:
      summary: Pause workflow run
      operationId: pauseWorkflowRun
      tags: [workflows]
      responses:
        '200':
          description: Workflow paused
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/WorkflowRun'

  /workflow-runs/{runId}/resume:
    parameters:
      - name: runId
        in: path
        required: true
        schema:
          type: string
          format: uuid

    post:
      summary: Resume workflow run
      operationId: resumeWorkflowRun
      tags: [workflows]
      responses:
        '202':
          description: Workflow resumed
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/WorkflowRun'

  /workflow-runs/{runId}/cancel:
    parameters:
      - name: runId
        in: path
        required: true
        schema:
          type: string
          format: uuid

    post:
      summary: Cancel workflow run
      operationId: cancelWorkflowRun
      tags: [workflows]
      responses:
        '200':
          description: Workflow cancelled
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/WorkflowRun'

  /states:
    post:
      summary: Capture experiment state
      operationId: captureState
      tags: [reproducibility]
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/CaptureStateRequest'
      responses:
        '201':
          description: State captured
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ExperimentState'

  /states/{stateId}/validate:
    parameters:
      - name: stateId
        in: path
        required: true
        schema:
          type: string
          format: uuid

    post:
      summary: Validate reproducibility
      operationId: validateReproducibility
      tags: [reproducibility]
      responses:
        '200':
          description: Validation report
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ValidationReport'

  /states/{stateId}/replay:
    parameters:
      - name: stateId
        in: path
        required: true
        schema:
          type: string
          format: uuid

    post:
      summary: Replay experiment
      operationId: replayExperiment
      tags: [reproducibility]
      responses:
        '200':
          description: Replay result
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ReplayResult'

  /certificates:
    post:
      summary: Generate reproducibility certificate
      operationId: generateCertificate
      tags: [reproducibility]
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/GenerateCertificateRequest'
      responses:
        '201':
          description: Certificate generated
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ReproducibilityCertificate'

components:
  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
      bearerFormat: JWT

  parameters:
    ExperimentId:
      name: experimentId
      in: path
      required: true
      schema:
        type: string
        format: uuid

    RunId:
      name: runId
      in: path
      required: true
      schema:
        type: string
        format: uuid

    PageOffset:
      name: offset
      in: query
      schema:
        type: integer
        minimum: 0
        default: 0

    PageLimit:
      name: limit
      in: query
      schema:
        type: integer
        minimum: 1
        maximum: 100
        default: 20

  responses:
    BadRequest:
      description: Invalid request
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/Error'

    Unauthorized:
      description: Authentication required
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/Error'

    NotFound:
      description: Resource not found
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/Error'

    Conflict:
      description: Resource conflict
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/Error'

    UnprocessableEntity:
      description: Validation failed
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ValidationError'

  schemas:
    Error:
      type: object
      required: [error]
      properties:
        error:
          type: object
          required: [code, message]
          properties:
            code:
              type: string
            message:
              type: string
            details:
              type: object
            request_id:
              type: string

    ValidationError:
      type: object
      required: [error]
      properties:
        error:
          type: object
          required: [code, message, fields]
          properties:
            code:
              type: string
              example: VALIDATION_ERROR
            message:
              type: string
            fields:
              type: array
              items:
                type: object
                properties:
                  field:
                    type: string
                  message:
                    type: string
                  code:
                    type: string

    ExperimentStatus:
      type: string
      enum: [draft, active, paused, completed, archived, failed]

    RunStatus:
      type: string
      enum: [pending, queued, running, completed, failed, cancelled, timed_out]

    ArtifactType:
      type: string
      enum: [model, checkpoint, dataset, visualization, log, config, report, custom]

    Experiment:
      type: object
      required: [id, name, owner_id, status, config, created_at, updated_at]
      properties:
        id:
          type: string
          format: uuid
        name:
          type: string
        description:
          type: string
        hypothesis:
          type: string
        owner_id:
          type: string
          format: uuid
        collaborators:
          type: array
          items:
            type: string
            format: uuid
        tags:
          type: array
          items:
            type: string
        status:
          $ref: '#/components/schemas/ExperimentStatus'
        config:
          type: object
        metadata:
          type: object
        created_at:
          type: string
          format: date-time
        updated_at:
          type: string
          format: date-time
        archived_at:
          type: string
          format: date-time

    ExperimentList:
      type: object
      required: [items, total_count, has_more]
      properties:
        items:
          type: array
          items:
            $ref: '#/components/schemas/Experiment'
        total_count:
          type: integer
        has_more:
          type: boolean
        next_offset:
          type: integer

    CreateExperimentRequest:
      type: object
      required: [name, config]
      properties:
        name:
          type: string
          minLength: 1
          maxLength: 256
        description:
          type: string
        hypothesis:
          type: string
        collaborators:
          type: array
          items:
            type: string
            format: uuid
        tags:
          type: array
          items:
            type: string
        config:
          type: object
        metadata:
          type: object

    UpdateExperimentRequest:
      type: object
      properties:
        name:
          type: string
        description:
          type: string
        hypothesis:
          type: string
        status:
          $ref: '#/components/schemas/ExperimentStatus'
        tags:
          type: array
          items:
            type: string
        config:
          type: object
        metadata:
          type: object

    ExperimentRun:
      type: object
      required: [id, experiment_id, run_number, status, parameters, created_at]
      properties:
        id:
          type: string
          format: uuid
        experiment_id:
          type: string
          format: uuid
        run_number:
          type: integer
        name:
          type: string
        status:
          $ref: '#/components/schemas/RunStatus'
        parameters:
          type: object
        environment:
          type: object
        metrics:
          type: object
        artifacts:
          type: array
          items:
            $ref: '#/components/schemas/ArtifactRef'
        parent_run_id:
          type: string
          format: uuid
        tags:
          type: array
          items:
            type: string
        started_at:
          type: string
          format: date-time
        ended_at:
          type: string
          format: date-time
        created_at:
          type: string
          format: date-time
        created_by:
          type: string
          format: uuid
        error:
          type: object

    RunList:
      type: object
      required: [items, total_count, has_more]
      properties:
        items:
          type: array
          items:
            $ref: '#/components/schemas/ExperimentRun'
        total_count:
          type: integer
        has_more:
          type: boolean

    StartRunRequest:
      type: object
      properties:
        name:
          type: string
        parameters:
          type: object
        tags:
          type: array
          items:
            type: string
        parent_run_id:
          type: string
          format: uuid

    LogMetricsRequest:
      type: object
      required: [metrics]
      properties:
        metrics:
          type: array
          items:
            type: object
            required: [name, value]
            properties:
              name:
                type: string
              value:
                type: number
              step:
                type: integer
              context:
                type: object

    UpdateRunStatusRequest:
      type: object
      required: [status]
      properties:
        status:
          $ref: '#/components/schemas/RunStatus'
        final_metrics:
          type: object
        error:
          type: object
          properties:
            error_type:
              type: string
            message:
              type: string
            stack_trace:
              type: string

    ArtifactRef:
      type: object
      required: [id, name, artifact_type, content_hash, size_bytes, storage_uri, created_at]
      properties:
        id:
          type: string
          format: uuid
        name:
          type: string
        artifact_type:
          $ref: '#/components/schemas/ArtifactType'
        content_hash:
          type: string
        size_bytes:
          type: integer
        storage_uri:
          type: string
        metadata:
          type: object
        created_at:
          type: string
          format: date-time

    RunMetrics:
      type: object
      properties:
        scalars:
          type: object
          additionalProperties:
            type: object
            properties:
              values:
                type: array
                items:
                  type: object
                  properties:
                    value:
                      type: number
                    step:
                      type: integer
                    timestamp:
                      type: string
                      format: date-time
              aggregations:
                type: object

    CompareRunsRequest:
      type: object
      required: [run_ids]
      properties:
        run_ids:
          type: array
          items:
            type: string
            format: uuid
          minItems: 2
        config:
          type: object
          properties:
            same_experiment_only:
              type: boolean
              default: true
            higher_is_better:
              type: boolean
              default: true
            compare_environments:
              type: boolean
              default: false
            run_statistical_tests:
              type: boolean
              default: true

    RunComparison:
      type: object
      properties:
        run_ids:
          type: array
          items:
            type: string
            format: uuid
        metric_comparisons:
          type: object
        parameter_diff:
          type: object
        environment_diff:
          type: object
        statistical_tests:
          type: object
        generated_at:
          type: string
          format: date-time

    BenchmarkSubmitRequest:
      type: object
      required: [model_id, suite_id]
      properties:
        model_id:
          type: string
        model_endpoint:
          type: string
        suite_id:
          type: string
          format: uuid
        parameters:
          type: object
        compare_to_baseline:
          type: boolean
          default: false
        callback_url:
          type: string
          format: uri

    BenchmarkJob:
      type: object
      properties:
        job_id:
          type: string
          format: uuid
        status:
          type: object
          properties:
            state:
              type: string
              enum: [pending, running, completed, failed, cancelled]
            progress:
              type: number
            current_test:
              type: string
            tests_completed:
              type: integer
            tests_total:
              type: integer
        submitted_at:
          type: string
          format: date-time

    BenchmarkResults:
      type: object
      properties:
        job_id:
          type: string
          format: uuid
        model_id:
          type: string
        suite_id:
          type: string
          format: uuid
        results:
          type: array
          items:
            type: object
        aggregated:
          type: object
        baseline_comparison:
          type: object
        completed_at:
          type: string
          format: date-time

    Dataset:
      type: object
      properties:
        id:
          type: string
          format: uuid
        name:
          type: string
        description:
          type: string
        schema:
          type: object
        governance:
          type: object
        owner_id:
          type: string
          format: uuid
        tags:
          type: array
          items:
            type: string
        created_at:
          type: string
          format: date-time
        updated_at:
          type: string
          format: date-time

    DatasetList:
      type: object
      properties:
        items:
          type: array
          items:
            $ref: '#/components/schemas/Dataset'
        total_count:
          type: integer
        has_more:
          type: boolean

    RegisterDatasetRequest:
      type: object
      required: [name, schema]
      properties:
        name:
          type: string
        description:
          type: string
        schema:
          type: object
        governance:
          type: object
        tags:
          type: array
          items:
            type: string

    DatasetVersion:
      type: object
      properties:
        id:
          type: string
          format: uuid
        dataset_id:
          type: string
          format: uuid
        version_number:
          type: integer
        content_hash:
          type: string
        parent_version_id:
          type: string
          format: uuid
        statistics:
          type: object
        created_at:
          type: string
          format: date-time

    Workflow:
      type: object
      properties:
        id:
          type: string
          format: uuid
        name:
          type: string
        description:
          type: string
        version:
          type: string
        steps:
          type: array
          items:
            type: object
        parameters:
          type: array
          items:
            type: object
        created_at:
          type: string
          format: date-time

    CreateWorkflowRequest:
      type: object
      required: [name, steps]
      properties:
        name:
          type: string
        description:
          type: string
        steps:
          type: array
          items:
            type: object
        parameters:
          type: array
          items:
            type: object
        triggers:
          type: array
          items:
            type: object

    WorkflowRun:
      type: object
      properties:
        id:
          type: string
          format: uuid
        workflow_id:
          type: string
          format: uuid
        status:
          type: string
          enum: [pending, running, paused, completed, failed, cancelled]
        parameters:
          type: object
        step_states:
          type: object
        outputs:
          type: object
        started_at:
          type: string
          format: date-time
        ended_at:
          type: string
          format: date-time

    SubmitWorkflowRunRequest:
      type: object
      properties:
        parameters:
          type: object

    ExperimentState:
      type: object
      properties:
        id:
          type: string
          format: uuid
        experiment_id:
          type: string
          format: uuid
        run_id:
          type: string
          format: uuid
        environment:
          type: object
        code_state:
          type: object
        data_state:
          type: object
        configuration:
          type: object
        random_state:
          type: object
        checksum:
          type: string
        captured_at:
          type: string
          format: date-time

    CaptureStateRequest:
      type: object
      required: [experiment_id, run_id]
      properties:
        experiment_id:
          type: string
          format: uuid
        run_id:
          type: string
          format: uuid
        parameters:
          type: object
        datasets:
          type: array
          items:
            type: object

    ValidationReport:
      type: object
      properties:
        state_id:
          type: string
          format: uuid
        is_reproducible:
          type: boolean
        issues:
          type: array
          items:
            type: object
        warnings:
          type: array
          items:
            type: object
        validated_at:
          type: string
          format: date-time

    ReplayResult:
      type: object
      properties:
        state_id:
          type: string
          format: uuid
        environment_differences:
          type: array
          items:
            type: string
        ready_to_execute:
          type: boolean
        setup_commands:
          type: array
          items:
            type: string

    GenerateCertificateRequest:
      type: object
      required: [state_id, run_id]
      properties:
        state_id:
          type: string
          format: uuid
        run_id:
          type: string
          format: uuid

    ReproducibilityCertificate:
      type: object
      properties:
        id:
          type: string
          format: uuid
        state_id:
          type: string
          format: uuid
        experiment_id:
          type: string
          format: uuid
        run_id:
          type: string
          format: uuid
        validation_report:
          $ref: '#/components/schemas/ValidationReport'
        environment_hash:
          type: string
        code_hash:
          type: string
        data_hash:
          type: string
        signature:
          type: string
        issued_at:
          type: string
          format: date-time
```

### 6.3 gRPC Service Definitions

```protobuf
// proto/experiment.proto
syntax = "proto3";

package research_lab.experiment.v1;

import "google/protobuf/timestamp.proto";
import "google/protobuf/struct.proto";

option go_package = "github.com/llm-devops/research-lab/gen/go/experiment/v1";

service ExperimentService {
  // Experiment operations
  rpc CreateExperiment(CreateExperimentRequest) returns (Experiment);
  rpc GetExperiment(GetExperimentRequest) returns (Experiment);
  rpc UpdateExperiment(UpdateExperimentRequest) returns (Experiment);
  rpc ListExperiments(ListExperimentsRequest) returns (ListExperimentsResponse);

  // Run operations
  rpc StartRun(StartRunRequest) returns (ExperimentRun);
  rpc GetRun(GetRunRequest) returns (ExperimentRun);
  rpc UpdateRunStatus(UpdateRunStatusRequest) returns (ExperimentRun);
  rpc ListRuns(ListRunsRequest) returns (ListRunsResponse);

  // Streaming operations
  rpc StreamMetrics(stream StreamMetricsRequest) returns (StreamMetricsResponse);
  rpc WatchRun(WatchRunRequest) returns (stream RunEvent);

  // Artifact operations
  rpc UploadArtifact(stream UploadArtifactRequest) returns (ArtifactRef);
  rpc DownloadArtifact(DownloadArtifactRequest) returns (stream DownloadArtifactResponse);
}

message Experiment {
  string id = 1;
  string name = 2;
  string description = 3;
  string hypothesis = 4;
  string owner_id = 5;
  repeated string collaborators = 6;
  repeated string tags = 7;
  ExperimentStatus status = 8;
  google.protobuf.Struct config = 9;
  google.protobuf.Struct metadata = 10;
  google.protobuf.Timestamp created_at = 11;
  google.protobuf.Timestamp updated_at = 12;
}

enum ExperimentStatus {
  EXPERIMENT_STATUS_UNSPECIFIED = 0;
  EXPERIMENT_STATUS_DRAFT = 1;
  EXPERIMENT_STATUS_ACTIVE = 2;
  EXPERIMENT_STATUS_PAUSED = 3;
  EXPERIMENT_STATUS_COMPLETED = 4;
  EXPERIMENT_STATUS_ARCHIVED = 5;
  EXPERIMENT_STATUS_FAILED = 6;
}

message ExperimentRun {
  string id = 1;
  string experiment_id = 2;
  int64 run_number = 3;
  string name = 4;
  RunStatus status = 5;
  google.protobuf.Struct parameters = 6;
  google.protobuf.Struct environment = 7;
  google.protobuf.Struct metrics = 8;
  repeated ArtifactRef artifacts = 9;
  string parent_run_id = 10;
  repeated string tags = 11;
  google.protobuf.Timestamp started_at = 12;
  google.protobuf.Timestamp ended_at = 13;
  google.protobuf.Timestamp created_at = 14;
  string created_by = 15;
  RunError error = 16;
}

enum RunStatus {
  RUN_STATUS_UNSPECIFIED = 0;
  RUN_STATUS_PENDING = 1;
  RUN_STATUS_QUEUED = 2;
  RUN_STATUS_RUNNING = 3;
  RUN_STATUS_COMPLETED = 4;
  RUN_STATUS_FAILED = 5;
  RUN_STATUS_CANCELLED = 6;
  RUN_STATUS_TIMED_OUT = 7;
}

message RunError {
  string error_type = 1;
  string message = 2;
  string stack_trace = 3;
  google.protobuf.Timestamp occurred_at = 4;
  bool recoverable = 5;
}

message ArtifactRef {
  string id = 1;
  string name = 2;
  ArtifactType artifact_type = 3;
  string content_hash = 4;
  int64 size_bytes = 5;
  string storage_uri = 6;
  map<string, string> metadata = 7;
  google.protobuf.Timestamp created_at = 8;
}

enum ArtifactType {
  ARTIFACT_TYPE_UNSPECIFIED = 0;
  ARTIFACT_TYPE_MODEL = 1;
  ARTIFACT_TYPE_CHECKPOINT = 2;
  ARTIFACT_TYPE_DATASET = 3;
  ARTIFACT_TYPE_VISUALIZATION = 4;
  ARTIFACT_TYPE_LOG = 5;
  ARTIFACT_TYPE_CONFIG = 6;
  ARTIFACT_TYPE_REPORT = 7;
  ARTIFACT_TYPE_CUSTOM = 8;
}

message CreateExperimentRequest {
  string name = 1;
  string description = 2;
  string hypothesis = 3;
  repeated string collaborators = 4;
  repeated string tags = 5;
  google.protobuf.Struct config = 6;
  google.protobuf.Struct metadata = 7;
}

message GetExperimentRequest {
  string id = 1;
}

message UpdateExperimentRequest {
  string id = 1;
  string name = 2;
  string description = 3;
  string hypothesis = 4;
  ExperimentStatus status = 5;
  repeated string tags = 6;
  google.protobuf.Struct config = 7;
  google.protobuf.Struct metadata = 8;
}

message ListExperimentsRequest {
  int32 offset = 1;
  int32 limit = 2;
  repeated ExperimentStatus statuses = 3;
  repeated string tags = 4;
  string owner_id = 5;
}

message ListExperimentsResponse {
  repeated Experiment experiments = 1;
  int64 total_count = 2;
  bool has_more = 3;
}

message StartRunRequest {
  string experiment_id = 1;
  string name = 2;
  google.protobuf.Struct parameters = 3;
  repeated string tags = 4;
  string parent_run_id = 5;
}

message GetRunRequest {
  string id = 1;
}

message UpdateRunStatusRequest {
  string id = 1;
  RunStatus status = 2;
  google.protobuf.Struct final_metrics = 3;
  RunError error = 4;
}

message ListRunsRequest {
  string experiment_id = 1;
  int32 offset = 2;
  int32 limit = 3;
  repeated RunStatus statuses = 4;
}

message ListRunsResponse {
  repeated ExperimentRun runs = 1;
  int64 total_count = 2;
  bool has_more = 3;
}

message StreamMetricsRequest {
  string run_id = 1;
  repeated MetricEntry metrics = 2;
}

message MetricEntry {
  string name = 1;
  double value = 2;
  int64 step = 3;
  map<string, string> context = 4;
}

message StreamMetricsResponse {
  int32 accepted_count = 1;
}

message WatchRunRequest {
  string run_id = 1;
}

message RunEvent {
  oneof event {
    StatusChangeEvent status_change = 1;
    MetricsUpdateEvent metrics_update = 2;
    ArtifactAddedEvent artifact_added = 3;
    ErrorEvent error = 4;
  }
}

message StatusChangeEvent {
  RunStatus old_status = 1;
  RunStatus new_status = 2;
  google.protobuf.Timestamp timestamp = 3;
}

message MetricsUpdateEvent {
  repeated MetricEntry metrics = 1;
  google.protobuf.Timestamp timestamp = 2;
}

message ArtifactAddedEvent {
  ArtifactRef artifact = 1;
  google.protobuf.Timestamp timestamp = 2;
}

message ErrorEvent {
  RunError error = 1;
  google.protobuf.Timestamp timestamp = 2;
}

message UploadArtifactRequest {
  oneof content {
    ArtifactMetadata metadata = 1;
    bytes chunk = 2;
  }
}

message ArtifactMetadata {
  string run_id = 1;
  string name = 2;
  ArtifactType artifact_type = 3;
  map<string, string> metadata = 4;
}

message DownloadArtifactRequest {
  string artifact_id = 1;
}

message DownloadArtifactResponse {
  bytes chunk = 1;
}
```

---

## 7. Security Architecture

### 7.1 Authentication & Authorization

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                           Security Architecture                                  │
└─────────────────────────────────────────────────────────────────────────────────┘

                    ┌───────────────────────────────────────┐
                    │           Identity Provider            │
                    │    (Keycloak / Auth0 / Okta)          │
                    │  • User Management                    │
                    │  • SSO / SAML / OIDC                  │
                    │  • MFA                                │
                    └───────────────────┬───────────────────┘
                                        │
                                        │ JWT Token
                                        ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│                              API Gateway                                         │
│  ┌─────────────────────────────────────────────────────────────────────────┐    │
│  │                        JWT Validation                                    │    │
│  │  • Signature verification                                               │    │
│  │  • Expiration check                                                     │    │
│  │  • Audience validation                                                  │    │
│  │  • Issuer validation                                                    │    │
│  └─────────────────────────────────────────────────────────────────────────┘    │
│                                      │                                           │
│  ┌─────────────────────────────────────────────────────────────────────────┐    │
│  │                        Rate Limiting                                     │    │
│  │  • Per-user limits                                                      │    │
│  │  • Per-endpoint limits                                                  │    │
│  │  • Burst protection                                                     │    │
│  └─────────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────────┘
                                        │
                                        ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│                              Service Layer                                       │
│  ┌─────────────────────────────────────────────────────────────────────────┐    │
│  │                     Authorization (RBAC)                                 │    │
│  │                                                                         │    │
│  │  Roles:                    Permissions:                                 │    │
│  │  ┌─────────────┐          ┌──────────────────────────────────────┐     │    │
│  │  │   Admin     │ ───────► │ experiments:*, runs:*, datasets:*,  │     │    │
│  │  └─────────────┘          │ metrics:*, workflows:*, admin:*     │     │    │
│  │  ┌─────────────┐          └──────────────────────────────────────┘     │    │
│  │  │ Researcher  │ ───────► experiments:crud, runs:crud, datasets:read, │    │
│  │  └─────────────┘          metrics:read, workflows:execute             │    │
│  │  ┌─────────────┐          ┌──────────────────────────────────────┐     │    │
│  │  │  Viewer     │ ───────► │ experiments:read, runs:read,         │     │    │
│  │  └─────────────┘          │ datasets:read, metrics:read          │     │    │
│  │  ┌─────────────┐          └──────────────────────────────────────┘     │    │
│  │  │   Service   │ ───────► integration:*, internal:*                   │    │
│  │  └─────────────┘                                                       │    │
│  └─────────────────────────────────────────────────────────────────────────┘    │
│                                                                                  │
│  ┌─────────────────────────────────────────────────────────────────────────┐    │
│  │                   Resource-Level Authorization                           │    │
│  │  • Owner-based access control                                           │    │
│  │  • Collaborator permissions                                             │    │
│  │  • Team/organization scoping                                            │    │
│  └─────────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### 7.2 Secrets Management

```yaml
# Vault configuration for secrets management
vault:
  address: https://vault.example.com
  auth_method: kubernetes
  kubernetes_role: research-lab

  secret_engines:
    # Database credentials
    database:
      path: database/creds/research-lab
      ttl: 1h
      max_ttl: 24h

    # API keys for external services
    kv:
      path: secret/research-lab
      secrets:
        - name: s3-credentials
          keys: [access_key, secret_key]
        - name: kafka-credentials
          keys: [username, password, ca_cert]
        - name: external-api-keys
          keys: [test_bench_api_key, analytics_hub_api_key, registry_api_key]

    # PKI for mTLS
    pki:
      path: pki/issue/research-lab
      common_name: "*.research-lab.svc.cluster.local"
      ttl: 72h

  policies:
    - name: research-lab-read
      rules: |
        path "secret/data/research-lab/*" {
          capabilities = ["read"]
        }
        path "database/creds/research-lab" {
          capabilities = ["read"]
        }
        path "pki/issue/research-lab" {
          capabilities = ["create", "update"]
        }

# Kubernetes Sealed Secrets for GitOps
sealed_secrets:
  - name: postgresql-credentials
    namespace: research-lab
    data:
      username: ENC[...]
      password: ENC[...]

  - name: redis-credentials
    namespace: research-lab
    data:
      password: ENC[...]

  - name: s3-credentials
    namespace: research-lab
    data:
      access_key: ENC[...]
      secret_key: ENC[...]
```

### 7.3 Network Security

```yaml
# mTLS Configuration with Linkerd
apiVersion: policy.linkerd.io/v1beta1
kind: Server
metadata:
  name: experiment-service
  namespace: research-lab
spec:
  podSelector:
    matchLabels:
      app: experiment-service
  port: 8001
  proxyProtocol: HTTP/2
---
apiVersion: policy.linkerd.io/v1beta1
kind: ServerAuthorization
metadata:
  name: experiment-service-auth
  namespace: research-lab
spec:
  server:
    name: experiment-service
  client:
    meshTLS:
      serviceAccounts:
        - name: api-gateway
        - name: metric-service
        - name: workflow-service
        - name: reproducibility-service
---
# Network encryption at rest
encryption:
  database:
    type: AES-256-GCM
    key_rotation: 90d
    key_storage: vault

  object_storage:
    type: AES-256
    server_side: true
    kms: aws-kms  # or vault-transit

  kafka:
    type: TLS
    client_auth: required
    protocol: SASL_SSL
    mechanism: SCRAM-SHA-512
```

### 7.4 Data Protection

```yaml
data_protection:
  # PII handling
  pii:
    detection:
      enabled: true
      patterns:
        - email
        - phone
        - ssn
        - credit_card
    handling:
      log_sanitization: true
      export_redaction: true

  # Data classification
  classification:
    levels:
      - name: public
        encryption: optional
        retention: unlimited
        access: all_authenticated

      - name: internal
        encryption: required
        retention: 2_years
        access: researchers

      - name: confidential
        encryption: required
        retention: 7_years
        access: authorized_only
        audit: full

      - name: restricted
        encryption: required
        retention: 7_years
        access: need_to_know
        audit: full
        dlp: enabled

  # Data retention
  retention:
    policies:
      - resource: experiments
        default: 2_years
        archive_after: 1_year

      - resource: runs
        default: 2_years
        archive_after: 6_months

      - resource: metrics
        default: 1_year
        downsample_after: 90_days

      - resource: artifacts
        default: 7_years
        tier_after: 90_days

      - resource: audit_logs
        default: 7_years
        immutable: true

  # Backup and DR
  backup:
    postgresql:
      type: continuous
      provider: pg_basebackup
      retention: 30_days
      point_in_time: 7_days

    s3:
      type: cross_region_replication
      destination: us-west-2
      versioning: enabled

    clickhouse:
      type: daily
      retention: 30_days
```

### 7.5 Compliance Framework

```yaml
compliance:
  frameworks:
    - SOC2_Type_II
    - GDPR
    - HIPAA  # if handling health data

  controls:
    access_control:
      - Multi-factor authentication
      - Role-based access control
      - Principle of least privilege
      - Regular access reviews

    audit_logging:
      - All data access logged
      - Admin actions logged
      - Log integrity verified
      - Retention: 7 years

    data_protection:
      - Encryption at rest
      - Encryption in transit
      - Key management via Vault
      - Data classification

    incident_response:
      - Automated alerting
      - Runbook automation
      - Post-incident review
      - SLA: 1 hour acknowledgment

    change_management:
      - GitOps workflow
      - Peer review required
      - Automated testing
      - Rollback capability

  audit_trail:
    events:
      - user_login
      - user_logout
      - resource_created
      - resource_updated
      - resource_deleted
      - permission_changed
      - export_requested
      - admin_action

    storage:
      type: immutable
      encryption: true
      retention: 7_years
```

---

## 8. Observability Architecture

### 8.1 Metrics Collection

```yaml
# Prometheus configuration
prometheus:
  global:
    scrape_interval: 15s
    evaluation_interval: 15s

  scrape_configs:
    # Service discovery for Kubernetes pods
    - job_name: 'research-lab-services'
      kubernetes_sd_configs:
        - role: pod
          namespaces:
            names: ['research-lab']
      relabel_configs:
        - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_scrape]
          action: keep
          regex: true
        - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_path]
          action: replace
          target_label: __metrics_path__
          regex: (.+)
        - source_labels: [__address__, __meta_kubernetes_pod_annotation_prometheus_io_port]
          action: replace
          regex: ([^:]+)(?::\d+)?;(\d+)
          replacement: $1:$2
          target_label: __address__
        - source_labels: [__meta_kubernetes_namespace]
          action: replace
          target_label: namespace
        - source_labels: [__meta_kubernetes_pod_name]
          action: replace
          target_label: pod
        - source_labels: [__meta_kubernetes_pod_label_app]
          action: replace
          target_label: app

  recording_rules:
    - name: research_lab_rules
      rules:
        # Request rate
        - record: research_lab:http_requests:rate5m
          expr: sum(rate(http_requests_total{namespace="research-lab"}[5m])) by (service, method, status)

        # Request latency percentiles
        - record: research_lab:http_request_duration:p50
          expr: histogram_quantile(0.50, sum(rate(http_request_duration_seconds_bucket{namespace="research-lab"}[5m])) by (service, le))

        - record: research_lab:http_request_duration:p95
          expr: histogram_quantile(0.95, sum(rate(http_request_duration_seconds_bucket{namespace="research-lab"}[5m])) by (service, le))

        - record: research_lab:http_request_duration:p99
          expr: histogram_quantile(0.99, sum(rate(http_request_duration_seconds_bucket{namespace="research-lab"}[5m])) by (service, le))

        # Error rate
        - record: research_lab:http_errors:rate5m
          expr: sum(rate(http_requests_total{namespace="research-lab", status=~"5.."}[5m])) by (service) / sum(rate(http_requests_total{namespace="research-lab"}[5m])) by (service)

        # Active experiments
        - record: research_lab:experiments:active
          expr: count(experiments_status{status="active"})

        # Running benchmarks
        - record: research_lab:benchmarks:running
          expr: count(benchmark_jobs_status{status="running"})

  alerting_rules:
    - name: research_lab_alerts
      rules:
        - alert: HighErrorRate
          expr: research_lab:http_errors:rate5m > 0.05
          for: 5m
          labels:
            severity: critical
          annotations:
            summary: "High error rate in {{ $labels.service }}"
            description: "Error rate is {{ $value | humanizePercentage }} for service {{ $labels.service }}"

        - alert: HighLatency
          expr: research_lab:http_request_duration:p95 > 2
          for: 5m
          labels:
            severity: warning
          annotations:
            summary: "High latency in {{ $labels.service }}"
            description: "P95 latency is {{ $value | humanizeDuration }} for service {{ $labels.service }}"

        - alert: ServiceDown
          expr: up{namespace="research-lab"} == 0
          for: 1m
          labels:
            severity: critical
          annotations:
            summary: "Service {{ $labels.service }} is down"

        - alert: DatabaseConnectionPoolExhausted
          expr: db_pool_connections_available{namespace="research-lab"} == 0
          for: 2m
          labels:
            severity: critical
          annotations:
            summary: "Database connection pool exhausted for {{ $labels.service }}"

        - alert: HighMemoryUsage
          expr: container_memory_usage_bytes{namespace="research-lab"} / container_spec_memory_limit_bytes > 0.9
          for: 5m
          labels:
            severity: warning
          annotations:
            summary: "High memory usage in {{ $labels.pod }}"
```

### 8.2 Application Metrics

```rust
//! Application metrics implementation using prometheus-client

use prometheus_client::{
    encoding::text::encode,
    metrics::{counter::Counter, gauge::Gauge, histogram::Histogram},
    registry::Registry,
};
use std::sync::Arc;

/// Service metrics collection
pub struct ServiceMetrics {
    pub registry: Registry,

    // HTTP metrics
    pub http_requests_total: Counter,
    pub http_request_duration: Histogram,
    pub http_requests_in_flight: Gauge,

    // Business metrics
    pub experiments_created: Counter,
    pub experiments_active: Gauge,
    pub runs_started: Counter,
    pub runs_completed: Counter,
    pub runs_failed: Counter,
    pub metrics_logged: Counter,
    pub artifacts_uploaded: Counter,
    pub artifacts_size_bytes: Counter,

    // Benchmark metrics
    pub benchmarks_submitted: Counter,
    pub benchmarks_completed: Counter,
    pub benchmark_duration: Histogram,

    // Dataset metrics
    pub datasets_registered: Counter,
    pub dataset_versions_created: Counter,
    pub dataset_size_bytes: Gauge,

    // Workflow metrics
    pub workflows_submitted: Counter,
    pub workflows_completed: Counter,
    pub workflow_steps_executed: Counter,
    pub workflow_duration: Histogram,

    // Infrastructure metrics
    pub db_query_duration: Histogram,
    pub db_connections_active: Gauge,
    pub cache_hits: Counter,
    pub cache_misses: Counter,
    pub s3_operations: Counter,
    pub kafka_messages_produced: Counter,
    pub kafka_messages_consumed: Counter,
}

impl ServiceMetrics {
    pub fn new(service_name: &str) -> Self {
        let mut registry = Registry::default();

        // HTTP metrics with labels
        let http_requests_total = Counter::default();
        registry.register(
            "http_requests_total",
            "Total HTTP requests",
            http_requests_total.clone(),
        );

        let http_request_duration = Histogram::new([
            0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
        ].into_iter());
        registry.register(
            "http_request_duration_seconds",
            "HTTP request duration in seconds",
            http_request_duration.clone(),
        );

        // ... register all other metrics

        Self {
            registry,
            http_requests_total,
            http_request_duration,
            http_requests_in_flight: Gauge::default(),
            experiments_created: Counter::default(),
            experiments_active: Gauge::default(),
            runs_started: Counter::default(),
            runs_completed: Counter::default(),
            runs_failed: Counter::default(),
            metrics_logged: Counter::default(),
            artifacts_uploaded: Counter::default(),
            artifacts_size_bytes: Counter::default(),
            benchmarks_submitted: Counter::default(),
            benchmarks_completed: Counter::default(),
            benchmark_duration: Histogram::new([1.0, 5.0, 10.0, 30.0, 60.0, 300.0, 600.0, 1800.0].into_iter()),
            datasets_registered: Counter::default(),
            dataset_versions_created: Counter::default(),
            dataset_size_bytes: Gauge::default(),
            workflows_submitted: Counter::default(),
            workflows_completed: Counter::default(),
            workflow_steps_executed: Counter::default(),
            workflow_duration: Histogram::new([1.0, 5.0, 10.0, 30.0, 60.0, 300.0, 600.0, 1800.0, 3600.0].into_iter()),
            db_query_duration: Histogram::new([0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0].into_iter()),
            db_connections_active: Gauge::default(),
            cache_hits: Counter::default(),
            cache_misses: Counter::default(),
            s3_operations: Counter::default(),
            kafka_messages_produced: Counter::default(),
            kafka_messages_consumed: Counter::default(),
        }
    }

    pub fn encode(&self) -> String {
        let mut buffer = String::new();
        encode(&mut buffer, &self.registry).unwrap();
        buffer
    }
}

/// Axum middleware for HTTP metrics
pub async fn metrics_middleware<B>(
    State(metrics): State<Arc<ServiceMetrics>>,
    req: Request<B>,
    next: Next<B>,
) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    metrics.http_requests_in_flight.inc();
    let start = std::time::Instant::now();

    let response = next.run(req).await;

    let duration = start.elapsed().as_secs_f64();
    let status = response.status().as_u16();

    metrics.http_requests_total.inc();
    metrics.http_request_duration.observe(duration);
    metrics.http_requests_in_flight.dec();

    response
}
```

### 8.3 Distributed Tracing

```yaml
# OpenTelemetry configuration
opentelemetry:
  service_name: research-lab

  exporters:
    otlp:
      endpoint: jaeger-collector.observability.svc:4317
      protocol: grpc
      tls:
        insecure: false
        ca_file: /etc/ssl/certs/ca-certificates.crt

  processors:
    batch:
      max_queue_size: 2048
      scheduled_delay_millis: 5000
      max_export_batch_size: 512

  sampling:
    type: parent_based
    root:
      type: trace_id_ratio
      ratio: 0.1  # Sample 10% of traces
    remote_parent_sampled: always_on
    remote_parent_not_sampled: always_off

  propagators:
    - tracecontext
    - baggage
    - b3

  resource:
    attributes:
      - key: service.namespace
        value: research-lab
      - key: deployment.environment
        value: ${ENVIRONMENT}
      - key: service.version
        value: ${VERSION}
```

```rust
//! Tracing implementation with OpenTelemetry

use opentelemetry::{
    global,
    sdk::{propagation::TraceContextPropagator, trace, Resource},
    trace::{Span, SpanKind, Tracer},
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init_tracing(service_name: &str, otlp_endpoint: &str) -> Result<(), TraceError> {
    // Set up propagator
    global::set_text_map_propagator(TraceContextPropagator::new());

    // Create OTLP exporter
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(otlp_endpoint);

    // Create tracer
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(
            trace::config()
                .with_sampler(trace::Sampler::TraceIdRatioBased(0.1))
                .with_resource(Resource::new(vec![
                    KeyValue::new("service.name", service_name.to_string()),
                    KeyValue::new("service.namespace", "research-lab"),
                ])),
        )
        .install_batch(opentelemetry::runtime::Tokio)?;

    // Set up tracing subscriber
    let telemetry = OpenTelemetryLayer::new(tracer);

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(telemetry)
        .with(tracing_subscriber::fmt::layer())
        .init();

    Ok(())
}

/// Custom span attributes for research lab operations
pub fn create_experiment_span(experiment_id: &str, user_id: &str) -> impl Span {
    let tracer = global::tracer("research-lab");
    tracer
        .span_builder("create_experiment")
        .with_kind(SpanKind::Internal)
        .with_attributes(vec![
            KeyValue::new("experiment.id", experiment_id.to_string()),
            KeyValue::new("user.id", user_id.to_string()),
        ])
        .start(&tracer)
}

pub fn create_run_span(run_id: &str, experiment_id: &str) -> impl Span {
    let tracer = global::tracer("research-lab");
    tracer
        .span_builder("start_run")
        .with_kind(SpanKind::Internal)
        .with_attributes(vec![
            KeyValue::new("run.id", run_id.to_string()),
            KeyValue::new("experiment.id", experiment_id.to_string()),
        ])
        .start(&tracer)
}

pub fn create_benchmark_span(job_id: &str, model_id: &str, suite_id: &str) -> impl Span {
    let tracer = global::tracer("research-lab");
    tracer
        .span_builder("run_benchmark")
        .with_kind(SpanKind::Internal)
        .with_attributes(vec![
            KeyValue::new("benchmark.job_id", job_id.to_string()),
            KeyValue::new("benchmark.model_id", model_id.to_string()),
            KeyValue::new("benchmark.suite_id", suite_id.to_string()),
        ])
        .start(&tracer)
}
```

### 8.4 Logging Configuration

```yaml
# Vector configuration for log collection
vector:
  sources:
    kubernetes_logs:
      type: kubernetes_logs
      namespace: research-lab

  transforms:
    parse_json:
      type: remap
      inputs: [kubernetes_logs]
      source: |
        . = parse_json!(.message)
        .timestamp = parse_timestamp!(.timestamp, format: "%+")
        .kubernetes = del(.kubernetes)
        .file = del(.file)

    add_metadata:
      type: remap
      inputs: [parse_json]
      source: |
        .environment = get_env_var!("ENVIRONMENT")
        .cluster = get_env_var!("CLUSTER_NAME")

    filter_noise:
      type: filter
      inputs: [add_metadata]
      condition: |
        !match(.message, r'^Health check')

    route_by_level:
      type: route
      inputs: [filter_noise]
      route:
        error: .level == "error" || .level == "ERROR"
        warn: .level == "warn" || .level == "WARN"
        info: .level == "info" || .level == "INFO"
        debug: .level == "debug" || .level == "DEBUG"

  sinks:
    loki:
      type: loki
      inputs: [route_by_level.info, route_by_level.warn, route_by_level.error]
      endpoint: http://loki.observability.svc:3100
      labels:
        service: "{{ .kubernetes.pod_labels.app }}"
        namespace: "{{ .kubernetes.namespace }}"
        level: "{{ .level }}"
      encoding:
        codec: json

    error_alerts:
      type: http
      inputs: [route_by_level.error]
      uri: https://alerts.example.com/webhook
      encoding:
        codec: json
      request:
        headers:
          Authorization: "Bearer ${ALERT_WEBHOOK_TOKEN}"
```

```rust
//! Structured logging configuration

use serde::Serialize;
use tracing::{info, error, warn, instrument, Level};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Serialize)]
pub struct LogContext {
    pub request_id: String,
    pub user_id: Option<String>,
    pub experiment_id: Option<String>,
    pub run_id: Option<String>,
}

pub fn init_logging() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = fmt::layer()
        .json()
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true);

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .init();
}

/// Structured logging macros with context
#[macro_export]
macro_rules! log_event {
    ($level:expr, $event:expr, $context:expr, $($field:tt)*) => {
        match $level {
            Level::ERROR => {
                tracing::error!(
                    event = $event,
                    request_id = %$context.request_id,
                    user_id = ?$context.user_id,
                    experiment_id = ?$context.experiment_id,
                    run_id = ?$context.run_id,
                    $($field)*
                );
            }
            Level::WARN => {
                tracing::warn!(
                    event = $event,
                    request_id = %$context.request_id,
                    user_id = ?$context.user_id,
                    $($field)*
                );
            }
            Level::INFO => {
                tracing::info!(
                    event = $event,
                    request_id = %$context.request_id,
                    user_id = ?$context.user_id,
                    $($field)*
                );
            }
            _ => {
                tracing::debug!(
                    event = $event,
                    request_id = %$context.request_id,
                    $($field)*
                );
            }
        }
    };
}
```

### 8.5 Dashboards

```yaml
# Grafana dashboard configuration
dashboards:
  - name: Research Lab Overview
    uid: research-lab-overview
    panels:
      - title: Request Rate
        type: graph
        queries:
          - expr: sum(rate(http_requests_total{namespace="research-lab"}[5m])) by (service)

      - title: Error Rate
        type: graph
        queries:
          - expr: research_lab:http_errors:rate5m

      - title: P95 Latency
        type: graph
        queries:
          - expr: research_lab:http_request_duration:p95

      - title: Active Experiments
        type: stat
        queries:
          - expr: research_lab:experiments:active

      - title: Running Benchmarks
        type: stat
        queries:
          - expr: research_lab:benchmarks:running

      - title: Database Connections
        type: graph
        queries:
          - expr: db_connections_active{namespace="research-lab"}

  - name: Experiment Details
    uid: research-lab-experiments
    variables:
      - name: experiment_id
        type: query
        query: label_values(experiment_runs_total, experiment_id)
    panels:
      - title: Runs by Status
        type: piechart
        queries:
          - expr: count(run_status{experiment_id="$experiment_id"}) by (status)

      - title: Run Duration Distribution
        type: histogram
        queries:
          - expr: histogram_quantile(0.95, sum(rate(run_duration_seconds_bucket{experiment_id="$experiment_id"}[1h])) by (le))

      - title: Metrics Logged
        type: graph
        queries:
          - expr: sum(rate(metrics_logged_total{experiment_id="$experiment_id"}[5m]))

  - name: Benchmark Performance
    uid: research-lab-benchmarks
    panels:
      - title: Benchmark Queue Depth
        type: graph
        queries:
          - expr: benchmark_queue_depth

      - title: Benchmark Duration by Suite
        type: graph
        queries:
          - expr: histogram_quantile(0.95, sum(rate(benchmark_duration_seconds_bucket[1h])) by (suite_id, le))

      - title: Model Comparison
        type: table
        queries:
          - expr: avg(benchmark_score) by (model_id, suite_id)
```

---

## 9. Operational Procedures

### 9.1 Deployment Pipeline

```yaml
# GitHub Actions CI/CD Pipeline
name: CI/CD Pipeline

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

env:
  REGISTRY: ghcr.io
  IMAGE_NAME: ${{ github.repository }}

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable

      - name: Run tests
        run: cargo test --all-features

      - name: Run clippy
        run: cargo clippy --all-features -- -D warnings

      - name: Check formatting
        run: cargo fmt --all -- --check

  build:
    needs: test
    runs-on: ubuntu-latest
    outputs:
      image_tag: ${{ steps.meta.outputs.tags }}
    steps:
      - uses: actions/checkout@v4

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Log in to Container Registry
        uses: docker/login-action@v3
        with:
          registry: ${{ env.REGISTRY }}
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Extract metadata
        id: meta
        uses: docker/metadata-action@v5
        with:
          images: ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}
          tags: |
            type=sha,prefix=
            type=ref,event=branch
            type=semver,pattern={{version}}

      - name: Build and push
        uses: docker/build-push-action@v5
        with:
          context: .
          push: true
          tags: ${{ steps.meta.outputs.tags }}
          labels: ${{ steps.meta.outputs.labels }}
          cache-from: type=gha
          cache-to: type=gha,mode=max

  deploy-staging:
    needs: build
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/develop'
    environment: staging
    steps:
      - uses: actions/checkout@v4

      - name: Deploy to staging
        uses: azure/k8s-deploy@v4
        with:
          namespace: research-lab-staging
          manifests: k8s/staging/
          images: ${{ needs.build.outputs.image_tag }}

      - name: Run integration tests
        run: |
          ./scripts/integration-tests.sh staging

  deploy-production:
    needs: [build, deploy-staging]
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    environment: production
    steps:
      - uses: actions/checkout@v4

      - name: Deploy to production (canary)
        uses: azure/k8s-deploy@v4
        with:
          namespace: research-lab
          manifests: k8s/production/
          images: ${{ needs.build.outputs.image_tag }}
          strategy: canary
          percentage: 10

      - name: Monitor canary
        run: |
          ./scripts/monitor-canary.sh 10m

      - name: Promote canary
        uses: azure/k8s-deploy@v4
        with:
          namespace: research-lab
          manifests: k8s/production/
          images: ${{ needs.build.outputs.image_tag }}
          strategy: canary
          action: promote
```

### 9.2 Runbooks

```yaml
# Incident Response Runbooks
runbooks:
  - name: High Error Rate
    trigger: HighErrorRate alert
    severity: P1
    steps:
      - action: Acknowledge alert
        command: |
          # Acknowledge in PagerDuty/OpsGenie

      - action: Check service health
        command: |
          kubectl get pods -n research-lab -l app=${SERVICE}
          kubectl logs -n research-lab -l app=${SERVICE} --tail=100

      - action: Check dependencies
        command: |
          kubectl exec -n research-lab deploy/${SERVICE} -- \
            curl -s http://localhost:8080/health/ready

      - action: Check database connectivity
        command: |
          kubectl exec -n research-lab deploy/${SERVICE} -- \
            pg_isready -h postgresql.data.svc

      - action: Check recent deployments
        command: |
          kubectl rollout history deployment/${SERVICE} -n research-lab

      - action: Rollback if needed
        command: |
          kubectl rollout undo deployment/${SERVICE} -n research-lab

  - name: Database Connection Pool Exhausted
    trigger: DatabaseConnectionPoolExhausted alert
    severity: P1
    steps:
      - action: Check active connections
        command: |
          kubectl exec -n data postgresql-0 -- \
            psql -c "SELECT count(*) FROM pg_stat_activity WHERE state = 'active';"

      - action: Check for long-running queries
        command: |
          kubectl exec -n data postgresql-0 -- \
            psql -c "SELECT pid, now() - pg_stat_activity.query_start AS duration, query
                     FROM pg_stat_activity
                     WHERE state != 'idle'
                     ORDER BY duration DESC
                     LIMIT 10;"

      - action: Kill long-running queries if necessary
        command: |
          kubectl exec -n data postgresql-0 -- \
            psql -c "SELECT pg_terminate_backend(pid)
                     FROM pg_stat_activity
                     WHERE duration > interval '5 minutes'
                     AND state != 'idle';"

      - action: Scale up replicas
        command: |
          kubectl scale deployment/${SERVICE} -n research-lab --replicas=+2

  - name: Kafka Consumer Lag
    trigger: KafkaConsumerLag alert
    severity: P2
    steps:
      - action: Check consumer lag
        command: |
          kubectl exec -n data kafka-0 -- \
            kafka-consumer-groups.sh --bootstrap-server localhost:9092 \
            --describe --group research-lab-consumers

      - action: Scale consumers
        command: |
          kubectl scale deployment/event-consumer -n research-lab --replicas=+2

      - action: Check for poison messages
        command: |
          kubectl exec -n data kafka-0 -- \
            kafka-console-consumer.sh --bootstrap-server localhost:9092 \
            --topic research-lab.events.dlq --from-beginning --max-messages 10
```

### 9.3 Disaster Recovery

```yaml
disaster_recovery:
  rpo: 1_hour  # Recovery Point Objective
  rto: 4_hours  # Recovery Time Objective

  backup_strategy:
    postgresql:
      type: continuous_archiving
      wal_archiving:
        destination: s3://research-lab-backups/wal/
        retention: 7_days
      base_backups:
        frequency: daily
        destination: s3://research-lab-backups/base/
        retention: 30_days

    clickhouse:
      type: incremental
      frequency: daily
      destination: s3://research-lab-backups/clickhouse/
      retention: 30_days

    s3:
      type: cross_region_replication
      source: us-east-1
      destination: us-west-2
      versioning: enabled

  recovery_procedures:
    - name: Database Recovery
      steps:
        - Stop all services writing to database
        - Identify point-in-time for recovery
        - Restore from latest base backup
        - Apply WAL archives to target time
        - Verify data integrity
        - Resume services

    - name: Full Cluster Recovery
      steps:
        - Provision new Kubernetes cluster
        - Restore Vault secrets
        - Deploy infrastructure components
        - Restore databases from backup
        - Deploy application services
        - Verify end-to-end functionality
        - Update DNS records

  testing:
    frequency: quarterly
    scope:
      - Database point-in-time recovery
      - Full cluster failover
      - Cross-region failover
    documentation: required
```

---

## Document Metadata

| Field | Value |
|-------|-------|
| **Version** | 1.0.0 |
| **Status** | Draft |
| **SPARC Phase** | Architecture (Part 2 of 2) |
| **Created** | 2025-11-28 |
| **Ecosystem** | LLM DevOps |
| **Previous Part** | Architecture Part 1: System Design & Infrastructure |
| **Next Phase** | Refinement |

---

*This architecture document completes the SPARC Architecture phase for LLM-Research-Lab. The next phase will cover Refinement (iteration and optimization based on implementation feedback).*
