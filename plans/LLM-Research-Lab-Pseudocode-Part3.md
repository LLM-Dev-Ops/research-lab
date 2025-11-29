# LLM-Research-Lab Pseudocode - Part 3: Integration APIs & Workflow Orchestration

> **SPARC Phase 2: Pseudocode (3 of 3)**
> Part of the LLM DevOps Ecosystem

---

## 5. Integration APIs

### 5.1 LLM-Test-Bench Client

```rust
//! Integration client for LLM-Test-Bench
//! Provides standardized benchmarking infrastructure integration

use async_trait::async_trait;
use std::sync::Arc;

/// Test-Bench integration client
pub struct TestBenchClient {
    inner: Arc<ResilientClient<TestBenchInner>>,
    config: TestBenchConfig,
}

#[derive(Debug, Clone)]
pub struct TestBenchConfig {
    pub base_url: String,
    pub api_key: String,
    pub timeout: std::time::Duration,
    pub retry_policy: RetryPolicy,
    pub circuit_breaker: CircuitBreakerConfig,
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout: std::time::Duration,
    pub half_open_max_calls: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout: std::time::Duration::from_secs(30),
            half_open_max_calls: 3,
        }
    }
}

/// Resilient client wrapper with circuit breaker and retry
pub struct ResilientClient<T> {
    inner: Arc<T>,
    circuit_breaker: Arc<CircuitBreaker>,
    retry_policy: RetryPolicy,
}

/// Circuit breaker implementation
pub struct CircuitBreaker {
    state: tokio::sync::RwLock<CircuitState>,
    config: CircuitBreakerConfig,
    failure_count: std::sync::atomic::AtomicU32,
    success_count: std::sync::atomic::AtomicU32,
    last_failure: tokio::sync::RwLock<Option<std::time::Instant>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: tokio::sync::RwLock::new(CircuitState::Closed),
            config,
            failure_count: std::sync::atomic::AtomicU32::new(0),
            success_count: std::sync::atomic::AtomicU32::new(0),
            last_failure: tokio::sync::RwLock::new(None),
        }
    }

    pub async fn can_execute(&self) -> bool {
        let state = *self.state.read().await;
        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if timeout has elapsed
                let last_failure = self.last_failure.read().await;
                if let Some(time) = *last_failure {
                    if time.elapsed() >= self.config.timeout {
                        // Transition to half-open
                        *self.state.write().await = CircuitState::HalfOpen;
                        self.success_count.store(0, std::sync::atomic::Ordering::SeqCst);
                        true
                    } else {
                        false
                    }
                } else {
                    true
                }
            }
            CircuitState::HalfOpen => {
                // Allow limited calls
                let current = self.success_count.load(std::sync::atomic::Ordering::SeqCst);
                current < self.config.half_open_max_calls
            }
        }
    }

    pub async fn record_success(&self) {
        let state = *self.state.read().await;
        match state {
            CircuitState::Closed => {
                self.failure_count.store(0, std::sync::atomic::Ordering::SeqCst);
            }
            CircuitState::HalfOpen => {
                let count = self.success_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                if count >= self.config.success_threshold {
                    *self.state.write().await = CircuitState::Closed;
                    self.failure_count.store(0, std::sync::atomic::Ordering::SeqCst);
                }
            }
            CircuitState::Open => {}
        }
    }

    pub async fn record_failure(&self) {
        let state = *self.state.read().await;
        match state {
            CircuitState::Closed => {
                let count = self.failure_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                if count >= self.config.failure_threshold {
                    *self.state.write().await = CircuitState::Open;
                    *self.last_failure.write().await = Some(std::time::Instant::now());
                }
            }
            CircuitState::HalfOpen => {
                *self.state.write().await = CircuitState::Open;
                *self.last_failure.write().await = Some(std::time::Instant::now());
            }
            CircuitState::Open => {}
        }
    }
}

impl<T> ResilientClient<T> {
    pub fn new(inner: Arc<T>, retry_policy: RetryPolicy, circuit_config: CircuitBreakerConfig) -> Self {
        Self {
            inner,
            circuit_breaker: Arc::new(CircuitBreaker::new(circuit_config)),
            retry_policy,
        }
    }

    pub async fn execute<F, Fut, R, E>(&self, operation: F) -> Result<R, IntegrationError>
    where
        F: Fn(Arc<T>) -> Fut + Clone,
        Fut: std::future::Future<Output = Result<R, E>>,
        E: std::error::Error + Send + Sync + 'static,
    {
        let mut attempts = 0;
        let mut last_error = None;

        while attempts <= self.retry_policy.max_retries {
            // Check circuit breaker
            if !self.circuit_breaker.can_execute().await {
                return Err(IntegrationError::CircuitOpen);
            }

            // Execute operation
            match operation(self.inner.clone()).await {
                Ok(result) => {
                    self.circuit_breaker.record_success().await;
                    return Ok(result);
                }
                Err(e) => {
                    self.circuit_breaker.record_failure().await;
                    last_error = Some(IntegrationError::Request(Box::new(e)));
                    attempts += 1;

                    if attempts <= self.retry_policy.max_retries {
                        let delay = self.calculate_delay(attempts);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or(IntegrationError::Unknown))
    }

    fn calculate_delay(&self, attempt: u32) -> std::time::Duration {
        let base_delay = self.retry_policy.initial_delay.as_millis() as f64;
        let delay = base_delay * self.retry_policy.exponential_base.powi(attempt as i32 - 1);
        let delay = std::cmp::min(
            delay as u64,
            self.retry_policy.max_delay.as_millis() as u64,
        );
        std::time::Duration::from_millis(delay)
    }
}

/// Test-Bench API trait
#[async_trait]
pub trait TestBenchApi: Send + Sync {
    async fn submit_benchmark(&self, request: BenchmarkSubmitRequest) -> Result<BenchmarkJob, IntegrationError>;
    async fn get_job_status(&self, job_id: &str) -> Result<JobStatus, IntegrationError>;
    async fn get_results(&self, job_id: &str) -> Result<BenchmarkJobResults, IntegrationError>;
    async fn cancel_job(&self, job_id: &str) -> Result<(), IntegrationError>;
    async fn list_benchmark_suites(&self) -> Result<Vec<BenchmarkSuiteInfo>, IntegrationError>;
    async fn get_baseline(&self, model_id: &str, suite_id: &str) -> Result<BaselineResults, IntegrationError>;
}

struct TestBenchInner {
    http_client: reqwest::Client,
    base_url: String,
    api_key: String,
}

#[async_trait]
impl TestBenchApi for TestBenchInner {
    async fn submit_benchmark(&self, request: BenchmarkSubmitRequest) -> Result<BenchmarkJob, IntegrationError> {
        let response = self.http_client
            .post(&format!("{}/api/v1/benchmarks", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(IntegrationError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            })
        }
    }

    async fn get_job_status(&self, job_id: &str) -> Result<JobStatus, IntegrationError> {
        let response = self.http_client
            .get(&format!("{}/api/v1/benchmarks/{}/status", self.base_url, job_id))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(IntegrationError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            })
        }
    }

    async fn get_results(&self, job_id: &str) -> Result<BenchmarkJobResults, IntegrationError> {
        let response = self.http_client
            .get(&format!("{}/api/v1/benchmarks/{}/results", self.base_url, job_id))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(IntegrationError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            })
        }
    }

    async fn cancel_job(&self, job_id: &str) -> Result<(), IntegrationError> {
        let response = self.http_client
            .delete(&format!("{}/api/v1/benchmarks/{}", self.base_url, job_id))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(IntegrationError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            })
        }
    }

    async fn list_benchmark_suites(&self) -> Result<Vec<BenchmarkSuiteInfo>, IntegrationError> {
        let response = self.http_client
            .get(&format!("{}/api/v1/suites", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(IntegrationError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            })
        }
    }

    async fn get_baseline(&self, model_id: &str, suite_id: &str) -> Result<BaselineResults, IntegrationError> {
        let response = self.http_client
            .get(&format!(
                "{}/api/v1/baselines/{}/{}",
                self.base_url, model_id, suite_id
            ))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(IntegrationError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            })
        }
    }
}

impl TestBenchClient {
    pub fn new(config: TestBenchConfig) -> Self {
        let inner = Arc::new(TestBenchInner {
            http_client: reqwest::Client::builder()
                .timeout(config.timeout)
                .build()
                .expect("Failed to build HTTP client"),
            base_url: config.base_url.clone(),
            api_key: config.api_key.clone(),
        });

        let resilient = Arc::new(ResilientClient::new(
            inner,
            config.retry_policy.clone(),
            config.circuit_breaker.clone(),
        ));

        Self {
            inner: resilient,
            config,
        }
    }

    pub async fn submit_benchmark(&self, request: BenchmarkSubmitRequest) -> Result<BenchmarkJob, IntegrationError> {
        self.inner.execute(|client| async move {
            client.submit_benchmark(request.clone()).await
        }).await
    }

    pub async fn wait_for_completion(
        &self,
        job_id: &str,
        poll_interval: std::time::Duration,
        timeout: Option<std::time::Duration>,
    ) -> Result<BenchmarkJobResults, IntegrationError> {
        let start = std::time::Instant::now();

        loop {
            let status = self.inner.execute(|client| {
                let job_id = job_id.to_string();
                async move { client.get_job_status(&job_id).await }
            }).await?;

            match status.state {
                JobState::Completed => {
                    return self.inner.execute(|client| {
                        let job_id = job_id.to_string();
                        async move { client.get_results(&job_id).await }
                    }).await;
                }
                JobState::Failed => {
                    return Err(IntegrationError::JobFailed {
                        job_id: job_id.to_string(),
                        error: status.error.unwrap_or_default(),
                    });
                }
                JobState::Cancelled => {
                    return Err(IntegrationError::JobCancelled {
                        job_id: job_id.to_string(),
                    });
                }
                _ => {
                    if let Some(timeout) = timeout {
                        if start.elapsed() >= timeout {
                            return Err(IntegrationError::Timeout);
                        }
                    }
                    tokio::time::sleep(poll_interval).await;
                }
            }
        }
    }
}

/// Request/Response types for Test-Bench
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSubmitRequest {
    pub model_id: String,
    pub model_endpoint: Option<String>,
    pub suite_id: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub compare_to_baseline: bool,
    pub callback_url: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkJob {
    pub job_id: String,
    pub status: JobStatus,
    pub submitted_at: DateTime<Utc>,
    pub estimated_completion: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStatus {
    pub state: JobState,
    pub progress: f64,
    pub current_test: Option<String>,
    pub tests_completed: u32,
    pub tests_total: u32,
    pub error: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkJobResults {
    pub job_id: String,
    pub model_id: String,
    pub suite_id: String,
    pub results: Vec<TestResult>,
    pub aggregated: AggregatedBenchmarkResults,
    pub baseline_comparison: Option<BaselineComparison>,
    pub completed_at: DateTime<Utc>,
    pub duration_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub test_id: String,
    pub test_name: String,
    pub passed: bool,
    pub score: f64,
    pub latency_ms: u64,
    pub tokens_used: u32,
    pub details: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedBenchmarkResults {
    pub overall_score: f64,
    pub pass_rate: f64,
    pub average_latency_ms: f64,
    pub total_tokens: u64,
    pub category_scores: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineComparison {
    pub baseline_model: String,
    pub score_diff: f64,
    pub latency_diff_percent: f64,
    pub improvements: Vec<String>,
    pub regressions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSuiteInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub test_count: u32,
    pub categories: Vec<String>,
    pub estimated_duration_minutes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineResults {
    pub model_id: String,
    pub suite_id: String,
    pub results: AggregatedBenchmarkResults,
    pub recorded_at: DateTime<Utc>,
}
```

### 5.2 LLM-Analytics-Hub Client

```rust
//! Integration client for LLM-Analytics-Hub
//! Provides visualization and analytical reporting integration

/// Analytics Hub client
pub struct AnalyticsHubClient {
    inner: Arc<ResilientClient<AnalyticsHubInner>>,
    metrics_buffer: Arc<MetricsBuffer>,
    config: AnalyticsHubConfig,
}

#[derive(Debug, Clone)]
pub struct AnalyticsHubConfig {
    pub base_url: String,
    pub api_key: String,
    pub buffer_size: usize,
    pub flush_interval: std::time::Duration,
    pub timeout: std::time::Duration,
    pub retry_policy: RetryPolicy,
}

/// Metrics buffer for batching
struct MetricsBuffer {
    buffer: tokio::sync::Mutex<Vec<MetricDataPoint>>,
    max_size: usize,
}

impl MetricsBuffer {
    fn new(max_size: usize) -> Self {
        Self {
            buffer: tokio::sync::Mutex::new(Vec::with_capacity(max_size)),
            max_size,
        }
    }

    async fn add(&self, point: MetricDataPoint) -> Option<Vec<MetricDataPoint>> {
        let mut buffer = self.buffer.lock().await;
        buffer.push(point);

        if buffer.len() >= self.max_size {
            let batch = std::mem::replace(&mut *buffer, Vec::with_capacity(self.max_size));
            Some(batch)
        } else {
            None
        }
    }

    async fn flush(&self) -> Vec<MetricDataPoint> {
        let mut buffer = self.buffer.lock().await;
        std::mem::replace(&mut *buffer, Vec::with_capacity(self.max_size))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDataPoint {
    pub metric_name: String,
    pub value: f64,
    pub timestamp: DateTime<Utc>,
    pub tags: HashMap<String, String>,
    pub dimensions: HashMap<String, String>,
}

/// Analytics Hub API trait
#[async_trait]
pub trait AnalyticsHubApi: Send + Sync {
    async fn push_metrics(&self, metrics: Vec<MetricDataPoint>) -> Result<(), IntegrationError>;
    async fn query(&self, query: AnalyticsQuery) -> Result<QueryResults, IntegrationError>;
    async fn create_dashboard(&self, config: DashboardConfig) -> Result<Dashboard, IntegrationError>;
    async fn get_dashboard(&self, id: &str) -> Result<Dashboard, IntegrationError>;
    async fn create_alert(&self, config: AlertConfig) -> Result<Alert, IntegrationError>;
    async fn get_anomalies(&self, query: AnomalyQuery) -> Result<Vec<Anomaly>, IntegrationError>;
}

struct AnalyticsHubInner {
    http_client: reqwest::Client,
    base_url: String,
    api_key: String,
}

#[async_trait]
impl AnalyticsHubApi for AnalyticsHubInner {
    async fn push_metrics(&self, metrics: Vec<MetricDataPoint>) -> Result<(), IntegrationError> {
        let response = self.http_client
            .post(&format!("{}/api/v1/metrics", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&metrics)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(IntegrationError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            })
        }
    }

    async fn query(&self, query: AnalyticsQuery) -> Result<QueryResults, IntegrationError> {
        let response = self.http_client
            .post(&format!("{}/api/v1/query", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&query)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(IntegrationError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            })
        }
    }

    async fn create_dashboard(&self, config: DashboardConfig) -> Result<Dashboard, IntegrationError> {
        let response = self.http_client
            .post(&format!("{}/api/v1/dashboards", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&config)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(IntegrationError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            })
        }
    }

    async fn get_dashboard(&self, id: &str) -> Result<Dashboard, IntegrationError> {
        let response = self.http_client
            .get(&format!("{}/api/v1/dashboards/{}", self.base_url, id))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(IntegrationError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            })
        }
    }

    async fn create_alert(&self, config: AlertConfig) -> Result<Alert, IntegrationError> {
        let response = self.http_client
            .post(&format!("{}/api/v1/alerts", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&config)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(IntegrationError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            })
        }
    }

    async fn get_anomalies(&self, query: AnomalyQuery) -> Result<Vec<Anomaly>, IntegrationError> {
        let response = self.http_client
            .post(&format!("{}/api/v1/anomalies", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&query)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(IntegrationError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            })
        }
    }
}

impl AnalyticsHubClient {
    pub fn new(config: AnalyticsHubConfig) -> Self {
        let inner = Arc::new(AnalyticsHubInner {
            http_client: reqwest::Client::builder()
                .timeout(config.timeout)
                .build()
                .expect("Failed to build HTTP client"),
            base_url: config.base_url.clone(),
            api_key: config.api_key.clone(),
        });

        let resilient = Arc::new(ResilientClient::new(
            inner,
            config.retry_policy.clone(),
            CircuitBreakerConfig::default(),
        ));

        let client = Self {
            inner: resilient,
            metrics_buffer: Arc::new(MetricsBuffer::new(config.buffer_size)),
            config: config.clone(),
        };

        // Start background flush task
        let buffer = client.metrics_buffer.clone();
        let resilient_clone = client.inner.clone();
        let interval = config.flush_interval;

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                let batch = buffer.flush().await;
                if !batch.is_empty() {
                    if let Err(e) = resilient_clone.execute(|client| {
                        let batch = batch.clone();
                        async move { client.push_metrics(batch).await }
                    }).await {
                        tracing::error!("Failed to flush metrics: {}", e);
                    }
                }
            }
        });

        client
    }

    /// Push a single metric (buffered)
    pub async fn push_metric(&self, point: MetricDataPoint) -> Result<(), IntegrationError> {
        if let Some(batch) = self.metrics_buffer.add(point).await {
            self.inner.execute(|client| {
                let batch = batch.clone();
                async move { client.push_metrics(batch).await }
            }).await?;
        }
        Ok(())
    }

    /// Push metrics immediately (unbuffered)
    pub async fn push_metrics_immediate(&self, metrics: Vec<MetricDataPoint>) -> Result<(), IntegrationError> {
        self.inner.execute(|client| {
            let metrics = metrics.clone();
            async move { client.push_metrics(metrics).await }
        }).await
    }

    /// Query historical metrics
    pub async fn query(&self, query: AnalyticsQuery) -> Result<QueryResults, IntegrationError> {
        self.inner.execute(|client| {
            let query = query.clone();
            async move { client.query(query).await }
        }).await
    }

    /// Create experiment dashboard
    pub async fn create_experiment_dashboard(
        &self,
        experiment_id: ExperimentId,
        metrics: Vec<String>,
    ) -> Result<Dashboard, IntegrationError> {
        let config = DashboardConfig {
            name: format!("Experiment {} Dashboard", experiment_id.0),
            description: Some("Auto-generated experiment dashboard".to_string()),
            panels: metrics
                .iter()
                .enumerate()
                .map(|(i, metric)| PanelConfig {
                    id: format!("panel_{}", i),
                    title: metric.clone(),
                    panel_type: PanelType::LineChart,
                    query: AnalyticsQuery {
                        metric_names: vec![metric.clone()],
                        time_range: TimeRange::Relative {
                            duration: std::time::Duration::from_secs(3600),
                        },
                        filters: vec![Filter {
                            field: "experiment_id".to_string(),
                            operator: FilterOperator::Equals,
                            value: experiment_id.0.to_string(),
                        }],
                        aggregation: Some(AggregationType::Mean),
                        group_by: vec!["run_id".to_string()],
                        order_by: None,
                        limit: None,
                    },
                    position: PanelPosition {
                        x: (i % 2) as u32 * 6,
                        y: (i / 2) as u32 * 4,
                        width: 6,
                        height: 4,
                    },
                })
                .collect(),
            refresh_interval: Some(std::time::Duration::from_secs(30)),
            tags: vec!["experiment".to_string(), experiment_id.0.to_string()],
        };

        self.inner.execute(|client| {
            let config = config.clone();
            async move { client.create_dashboard(config).await }
        }).await
    }
}

/// Analytics query types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsQuery {
    pub metric_names: Vec<String>,
    pub time_range: TimeRange,
    pub filters: Vec<Filter>,
    pub aggregation: Option<AggregationType>,
    pub group_by: Vec<String>,
    pub order_by: Option<OrderBy>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeRange {
    Absolute {
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    },
    Relative {
        duration: std::time::Duration,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
    pub field: String,
    pub operator: FilterOperator,
    pub value: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterOperator {
    Equals,
    NotEquals,
    Contains,
    GreaterThan,
    LessThan,
    In,
    NotIn,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggregationType {
    Sum,
    Mean,
    Median,
    Min,
    Max,
    Count,
    Percentile(u8),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBy {
    pub field: String,
    pub direction: SortDirection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResults {
    pub series: Vec<TimeSeries>,
    pub metadata: QueryMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeries {
    pub metric_name: String,
    pub tags: HashMap<String, String>,
    pub points: Vec<DataPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMetadata {
    pub execution_time_ms: u64,
    pub rows_scanned: u64,
    pub result_count: u64,
}

/// Dashboard types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    pub name: String,
    pub description: Option<String>,
    pub panels: Vec<PanelConfig>,
    pub refresh_interval: Option<std::time::Duration>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelConfig {
    pub id: String,
    pub title: String,
    pub panel_type: PanelType,
    pub query: AnalyticsQuery,
    pub position: PanelPosition,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelType {
    LineChart,
    BarChart,
    Gauge,
    Table,
    Heatmap,
    Scatter,
    Pie,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PanelPosition {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dashboard {
    pub id: String,
    pub name: String,
    pub url: String,
    pub panels: Vec<PanelConfig>,
    pub created_at: DateTime<Utc>,
}

/// Alert types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    pub name: String,
    pub description: Option<String>,
    pub condition: AlertCondition,
    pub channels: Vec<NotificationChannel>,
    pub cooldown: std::time::Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertCondition {
    pub metric_name: String,
    pub operator: ComparisonOperator,
    pub threshold: f64,
    pub duration: std::time::Duration,
    pub filters: Vec<Filter>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonOperator {
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Equal,
    NotEqual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationChannel {
    Email { addresses: Vec<String> },
    Slack { webhook_url: String },
    PagerDuty { routing_key: String },
    Webhook { url: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub name: String,
    pub status: AlertStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertStatus {
    Active,
    Silenced,
    Disabled,
}

/// Anomaly detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyQuery {
    pub metric_name: String,
    pub time_range: TimeRange,
    pub sensitivity: f64,
    pub filters: Vec<Filter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub timestamp: DateTime<Utc>,
    pub metric_name: String,
    pub expected_value: f64,
    pub actual_value: f64,
    pub deviation: f64,
    pub severity: AnomalySeverity,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnomalySeverity {
    Low,
    Medium,
    High,
    Critical,
}
```

### 5.3 LLM-Registry Client

```rust
//! Integration client for LLM-Registry
//! Provides artifact and model versioning integration

/// Registry client
pub struct RegistryClient {
    inner: Arc<ResilientClient<RegistryInner>>,
    config: RegistryConfig,
}

#[derive(Debug, Clone)]
pub struct RegistryConfig {
    pub base_url: String,
    pub api_key: String,
    pub timeout: std::time::Duration,
    pub retry_policy: RetryPolicy,
    pub chunk_size: usize,
}

#[async_trait]
pub trait RegistryApi: Send + Sync {
    async fn register_model(&self, request: RegisterModelRequest) -> Result<ModelEntry, IntegrationError>;
    async fn get_model(&self, model_id: &str, version: Option<&str>) -> Result<ModelEntry, IntegrationError>;
    async fn list_models(&self, filter: ModelFilter) -> Result<Vec<ModelEntry>, IntegrationError>;
    async fn upload_artifact(&self, artifact: ArtifactUpload) -> Result<ArtifactEntry, IntegrationError>;
    async fn download_artifact(&self, artifact_id: &str) -> Result<Vec<u8>, IntegrationError>;
    async fn register_metric_definition(&self, metric: MetricDefinition) -> Result<(), IntegrationError>;
    async fn get_metric_definition(&self, name: &str, version: &str) -> Result<MetricDefinition, IntegrationError>;
}

struct RegistryInner {
    http_client: reqwest::Client,
    base_url: String,
    api_key: String,
    chunk_size: usize,
}

impl RegistryClient {
    pub fn new(config: RegistryConfig) -> Self {
        let inner = Arc::new(RegistryInner {
            http_client: reqwest::Client::builder()
                .timeout(config.timeout)
                .build()
                .expect("Failed to build HTTP client"),
            base_url: config.base_url.clone(),
            api_key: config.api_key.clone(),
            chunk_size: config.chunk_size,
        });

        let resilient = Arc::new(ResilientClient::new(
            inner,
            config.retry_policy.clone(),
            CircuitBreakerConfig::default(),
        ));

        Self {
            inner: resilient,
            config,
        }
    }

    pub async fn register_model(&self, request: RegisterModelRequest) -> Result<ModelEntry, IntegrationError> {
        self.inner.execute(|client| {
            let request = request.clone();
            async move { client.register_model(request).await }
        }).await
    }

    pub async fn get_model(&self, model_id: &str, version: Option<&str>) -> Result<ModelEntry, IntegrationError> {
        self.inner.execute(|client| {
            let model_id = model_id.to_string();
            let version = version.map(|s| s.to_string());
            async move { client.get_model(&model_id, version.as_deref()).await }
        }).await
    }

    pub async fn upload_experiment_artifact(
        &self,
        experiment_id: ExperimentId,
        run_id: RunId,
        artifact: ArtifactUpload,
    ) -> Result<ArtifactEntry, IntegrationError> {
        let mut upload = artifact;
        upload.metadata.insert("experiment_id".to_string(), experiment_id.0.to_string());
        upload.metadata.insert("run_id".to_string(), run_id.0.to_string());

        self.inner.execute(|client| {
            let upload = upload.clone();
            async move { client.upload_artifact(upload).await }
        }).await
    }
}

#[async_trait]
impl RegistryApi for RegistryInner {
    async fn register_model(&self, request: RegisterModelRequest) -> Result<ModelEntry, IntegrationError> {
        let response = self.http_client
            .post(&format!("{}/api/v1/models", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(IntegrationError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            })
        }
    }

    async fn get_model(&self, model_id: &str, version: Option<&str>) -> Result<ModelEntry, IntegrationError> {
        let url = match version {
            Some(v) => format!("{}/api/v1/models/{}/versions/{}", self.base_url, model_id, v),
            None => format!("{}/api/v1/models/{}/latest", self.base_url, model_id),
        };

        let response = self.http_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(IntegrationError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            })
        }
    }

    async fn list_models(&self, filter: ModelFilter) -> Result<Vec<ModelEntry>, IntegrationError> {
        let response = self.http_client
            .get(&format!("{}/api/v1/models", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .query(&filter)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(IntegrationError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            })
        }
    }

    async fn upload_artifact(&self, artifact: ArtifactUpload) -> Result<ArtifactEntry, IntegrationError> {
        // Multipart upload for large artifacts
        let form = reqwest::multipart::Form::new()
            .text("name", artifact.name.clone())
            .text("artifact_type", format!("{:?}", artifact.artifact_type))
            .text("metadata", serde_json::to_string(&artifact.metadata).unwrap_or_default())
            .part("file", reqwest::multipart::Part::bytes(artifact.data.clone())
                .file_name(artifact.name.clone()));

        let response = self.http_client
            .post(&format!("{}/api/v1/artifacts", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(IntegrationError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            })
        }
    }

    async fn download_artifact(&self, artifact_id: &str) -> Result<Vec<u8>, IntegrationError> {
        let response = self.http_client
            .get(&format!("{}/api/v1/artifacts/{}/download", self.base_url, artifact_id))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.bytes().await?.to_vec())
        } else {
            Err(IntegrationError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            })
        }
    }

    async fn register_metric_definition(&self, metric: MetricDefinition) -> Result<(), IntegrationError> {
        let response = self.http_client
            .post(&format!("{}/api/v1/metrics", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&metric)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(IntegrationError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            })
        }
    }

    async fn get_metric_definition(&self, name: &str, version: &str) -> Result<MetricDefinition, IntegrationError> {
        let response = self.http_client
            .get(&format!("{}/api/v1/metrics/{}/versions/{}", self.base_url, name, version))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(IntegrationError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            })
        }
    }
}

/// Registry types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterModelRequest {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub model_type: ModelType,
    pub artifact_uri: Option<String>,
    pub metrics: HashMap<String, f64>,
    pub parameters: HashMap<String, serde_json::Value>,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub model_type: ModelType,
    pub artifact_uri: Option<String>,
    pub metrics: HashMap<String, f64>,
    pub parameters: HashMap<String, serde_json::Value>,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub stage: ModelStage,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelType {
    LLM,
    Embedding,
    Classification,
    Regression,
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelStage {
    Development,
    Staging,
    Production,
    Archived,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelFilter {
    pub name: Option<String>,
    pub model_type: Option<ModelType>,
    pub stage: Option<ModelStage>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct ArtifactUpload {
    pub name: String,
    pub artifact_type: ArtifactType,
    pub data: Vec<u8>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactEntry {
    pub id: String,
    pub name: String,
    pub artifact_type: ArtifactType,
    pub size_bytes: u64,
    pub content_hash: String,
    pub storage_uri: String,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
}
```

### 5.4 Integration Error Types

```rust
/// Integration error types
#[derive(Debug, thiserror::Error)]
pub enum IntegrationError {
    #[error("HTTP request error: {0}")]
    Request(#[from] Box<dyn std::error::Error + Send + Sync>),

    #[error("API error (status {status}): {message}")]
    ApiError { status: u16, message: String },

    #[error("Circuit breaker open")]
    CircuitOpen,

    #[error("Request timeout")]
    Timeout,

    #[error("Job failed: {job_id} - {error}")]
    JobFailed { job_id: String, error: String },

    #[error("Job cancelled: {job_id}")]
    JobCancelled { job_id: String },

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Unknown error")]
    Unknown,
}

impl From<reqwest::Error> for IntegrationError {
    fn from(err: reqwest::Error) -> Self {
        IntegrationError::Request(Box::new(err))
    }
}
```

---

## 6. Reproducibility Engine

### 6.1 Reproducibility Core

```rust
//! Reproducibility engine for experiment state capture and replay
//! Ensures experiments can be reliably reproduced

/// Reproducibility engine
pub struct ReproducibilityEngine {
    state_store: Arc<dyn StateStore>,
    environment_capture: Arc<EnvironmentCapture>,
    code_snapshot: Arc<CodeSnapshot>,
    config: ReproducibilityConfig,
}

#[derive(Debug, Clone)]
pub struct ReproducibilityConfig {
    pub capture_code: bool,
    pub capture_data_hashes: bool,
    pub capture_system_state: bool,
    pub strict_mode: bool,
    pub hash_algorithm: HashAlgorithm,
}

#[derive(Debug, Clone, Copy)]
pub enum HashAlgorithm {
    Sha256,
    Blake3,
}

impl Default for ReproducibilityConfig {
    fn default() -> Self {
        Self {
            capture_code: true,
            capture_data_hashes: true,
            capture_system_state: true,
            strict_mode: true,
            hash_algorithm: HashAlgorithm::Sha256,
        }
    }
}

/// Complete experiment state for reproduction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentState {
    pub id: Uuid,
    pub experiment_id: ExperimentId,
    pub run_id: RunId,
    pub captured_at: DateTime<Utc>,
    pub environment: EnvironmentSnapshot,
    pub code_state: CodeState,
    pub data_state: DataState,
    pub configuration: ConfigurationState,
    pub random_state: RandomState,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeState {
    pub git_commit: Option<String>,
    pub git_branch: Option<String>,
    pub git_dirty: bool,
    pub code_hash: String,
    pub patches: Vec<CodePatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodePatch {
    pub file_path: String,
    pub diff: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataState {
    pub datasets: Vec<DatasetReference>,
    pub data_hashes: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetReference {
    pub dataset_id: DatasetId,
    pub version_id: DatasetVersionId,
    pub content_hash: String,
    pub row_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationState {
    pub experiment_config: serde_json::Value,
    pub run_parameters: HashMap<String, ParameterValue>,
    pub environment_variables: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RandomState {
    pub global_seed: Option<u64>,
    pub numpy_state: Option<String>,
    pub torch_state: Option<String>,
    pub random_state: Option<String>,
}

impl ReproducibilityEngine {
    pub fn new(
        state_store: Arc<dyn StateStore>,
        environment_capture: Arc<EnvironmentCapture>,
        code_snapshot: Arc<CodeSnapshot>,
        config: ReproducibilityConfig,
    ) -> Self {
        Self {
            state_store,
            environment_capture,
            code_snapshot,
            config,
        }
    }

    /// Capture complete experiment state
    pub async fn capture_state(
        &self,
        experiment_id: ExperimentId,
        run_id: RunId,
        run_parameters: HashMap<String, ParameterValue>,
        datasets: Vec<DatasetReference>,
    ) -> Result<ExperimentState, ReproducibilityError> {
        // Capture environment
        let environment = self.environment_capture.capture().await?;

        // Capture code state
        let code_state = if self.config.capture_code {
            self.code_snapshot.capture().await?
        } else {
            CodeState {
                git_commit: None,
                git_branch: None,
                git_dirty: false,
                code_hash: String::new(),
                patches: Vec::new(),
            }
        };

        // Capture data state
        let data_state = DataState {
            datasets,
            data_hashes: if self.config.capture_data_hashes {
                self.compute_data_hashes().await?
            } else {
                HashMap::new()
            },
        };

        // Capture configuration
        let configuration = ConfigurationState {
            experiment_config: serde_json::json!({}),
            run_parameters,
            environment_variables: environment.environment_variables.clone(),
        };

        // Capture random state
        let random_state = self.capture_random_state()?;

        let state = ExperimentState {
            id: Uuid::new_v4(),
            experiment_id,
            run_id,
            captured_at: Utc::now(),
            environment,
            code_state,
            data_state,
            configuration,
            random_state,
            checksum: String::new(),
        };

        // Compute checksum
        let checksum = self.compute_state_checksum(&state)?;
        let state = ExperimentState { checksum, ..state };

        // Store state
        self.state_store.save(&state).await?;

        Ok(state)
    }

    /// Validate reproducibility of a configuration
    pub async fn validate_reproducibility(
        &self,
        state: &ExperimentState,
    ) -> Result<ValidationReport, ReproducibilityError> {
        let mut issues = Vec::new();
        let mut warnings = Vec::new();

        // Check code state
        if state.code_state.git_dirty {
            if self.config.strict_mode {
                issues.push(ValidationIssue {
                    severity: IssueSeverity::Error,
                    category: IssueCategory::Code,
                    message: "Repository has uncommitted changes".to_string(),
                    suggestion: Some("Commit all changes before running experiment".to_string()),
                });
            } else {
                warnings.push(ValidationIssue {
                    severity: IssueSeverity::Warning,
                    category: IssueCategory::Code,
                    message: "Repository has uncommitted changes".to_string(),
                    suggestion: Some("Consider committing changes for full reproducibility".to_string()),
                });
            }
        }

        // Check environment
        if state.environment.dependencies.packages.is_empty() {
            warnings.push(ValidationIssue {
                severity: IssueSeverity::Warning,
                category: IssueCategory::Environment,
                message: "No dependency manifest captured".to_string(),
                suggestion: Some("Ensure Cargo.lock or requirements.txt exists".to_string()),
            });
        }

        // Check random state
        if state.random_state.global_seed.is_none() {
            warnings.push(ValidationIssue {
                severity: IssueSeverity::Warning,
                category: IssueCategory::Randomness,
                message: "No global random seed set".to_string(),
                suggestion: Some("Set a global seed for deterministic results".to_string()),
            });
        }

        // Check data integrity
        for dataset in &state.data_state.datasets {
            if dataset.content_hash.is_empty() {
                issues.push(ValidationIssue {
                    severity: IssueSeverity::Error,
                    category: IssueCategory::Data,
                    message: format!("Dataset {} has no content hash", dataset.dataset_id.0),
                    suggestion: Some("Re-register dataset with content hashing enabled".to_string()),
                });
            }
        }

        let is_reproducible = issues.is_empty();

        Ok(ValidationReport {
            state_id: state.id,
            is_reproducible,
            issues,
            warnings,
            validated_at: Utc::now(),
        })
    }

    /// Replay an experiment from captured state
    pub async fn replay_experiment(
        &self,
        state: &ExperimentState,
    ) -> Result<ReplayResult, ReproducibilityError> {
        // Validate state checksum
        let computed_checksum = self.compute_state_checksum(state)?;
        if computed_checksum != state.checksum {
            return Err(ReproducibilityError::ChecksumMismatch {
                expected: state.checksum.clone(),
                actual: computed_checksum,
            });
        }

        // Restore code state
        if self.config.capture_code {
            self.restore_code_state(&state.code_state).await?;
        }

        // Restore random state
        self.restore_random_state(&state.random_state)?;

        // Verify data availability
        for dataset in &state.data_state.datasets {
            self.verify_dataset_available(dataset).await?;
        }

        // Verify environment compatibility
        let current_env = self.environment_capture.capture().await?;
        let env_diff = self.compare_environments(&state.environment, &current_env);

        Ok(ReplayResult {
            state_id: state.id,
            environment_differences: env_diff,
            ready_to_execute: true,
            setup_commands: self.generate_setup_commands(state),
            replayed_at: Utc::now(),
        })
    }

    /// Compare two experiment states
    pub async fn compare_states(
        &self,
        state_a: &ExperimentState,
        state_b: &ExperimentState,
    ) -> Result<StateComparison, ReproducibilityError> {
        let mut differences = Vec::new();

        // Compare code
        if state_a.code_state.code_hash != state_b.code_state.code_hash {
            differences.push(StateDifference {
                category: DifferenceCategory::Code,
                field: "code_hash".to_string(),
                value_a: state_a.code_state.code_hash.clone(),
                value_b: state_b.code_state.code_hash.clone(),
                impact: DifferenceImpact::High,
            });
        }

        // Compare parameters
        for (key, value_a) in &state_a.configuration.run_parameters {
            if let Some(value_b) = state_b.configuration.run_parameters.get(key) {
                if value_a != value_b {
                    differences.push(StateDifference {
                        category: DifferenceCategory::Configuration,
                        field: key.clone(),
                        value_a: format!("{:?}", value_a),
                        value_b: format!("{:?}", value_b),
                        impact: DifferenceImpact::High,
                    });
                }
            } else {
                differences.push(StateDifference {
                    category: DifferenceCategory::Configuration,
                    field: key.clone(),
                    value_a: format!("{:?}", value_a),
                    value_b: "missing".to_string(),
                    impact: DifferenceImpact::High,
                });
            }
        }

        // Compare data
        for dataset_a in &state_a.data_state.datasets {
            let matching = state_b
                .data_state
                .datasets
                .iter()
                .find(|d| d.dataset_id == dataset_a.dataset_id);

            if let Some(dataset_b) = matching {
                if dataset_a.content_hash != dataset_b.content_hash {
                    differences.push(StateDifference {
                        category: DifferenceCategory::Data,
                        field: format!("dataset:{}", dataset_a.dataset_id.0),
                        value_a: dataset_a.content_hash.clone(),
                        value_b: dataset_b.content_hash.clone(),
                        impact: DifferenceImpact::High,
                    });
                }
            }
        }

        // Compare environment
        let env_diffs = self.compare_environments(&state_a.environment, &state_b.environment);
        for diff in env_diffs {
            differences.push(StateDifference {
                category: DifferenceCategory::Environment,
                field: diff.clone(),
                value_a: "state_a".to_string(),
                value_b: "state_b".to_string(),
                impact: DifferenceImpact::Medium,
            });
        }

        let are_equivalent = differences.is_empty();

        Ok(StateComparison {
            state_a_id: state_a.id,
            state_b_id: state_b.id,
            are_equivalent,
            differences,
            compared_at: Utc::now(),
        })
    }

    /// Generate reproducibility certificate
    pub async fn generate_certificate(
        &self,
        state: &ExperimentState,
        run_results: &ExperimentRun,
    ) -> Result<ReproducibilityCertificate, ReproducibilityError> {
        let validation = self.validate_reproducibility(state).await?;

        if !validation.is_reproducible && self.config.strict_mode {
            return Err(ReproducibilityError::ValidationFailed {
                issues: validation.issues,
            });
        }

        let certificate = ReproducibilityCertificate {
            id: Uuid::new_v4(),
            state_id: state.id,
            experiment_id: state.experiment_id,
            run_id: state.run_id,
            validation_report: validation,
            environment_hash: self.hash_environment(&state.environment),
            code_hash: state.code_state.code_hash.clone(),
            data_hash: self.hash_data_state(&state.data_state),
            configuration_hash: self.hash_configuration(&state.configuration),
            results_hash: self.hash_results(run_results),
            issued_at: Utc::now(),
            signature: self.sign_certificate(state, run_results)?,
        };

        Ok(certificate)
    }

    // Private helper methods

    fn compute_state_checksum(&self, state: &ExperimentState) -> Result<String, ReproducibilityError> {
        let mut hasher = match self.config.hash_algorithm {
            HashAlgorithm::Sha256 => {
                use sha2::{Sha256, Digest};
                let state_bytes = serde_json::to_vec(&ExperimentState {
                    checksum: String::new(),
                    ..state.clone()
                })?;
                let mut hasher = Sha256::new();
                hasher.update(&state_bytes);
                hex::encode(hasher.finalize())
            }
            HashAlgorithm::Blake3 => {
                let state_bytes = serde_json::to_vec(&ExperimentState {
                    checksum: String::new(),
                    ..state.clone()
                })?;
                let hash = blake3::hash(&state_bytes);
                hash.to_hex().to_string()
            }
        };

        Ok(hasher)
    }

    fn capture_random_state(&self) -> Result<RandomState, ReproducibilityError> {
        // This would integrate with the actual random state capture
        Ok(RandomState {
            global_seed: None,
            numpy_state: None,
            torch_state: None,
            random_state: None,
        })
    }

    fn restore_random_state(&self, _state: &RandomState) -> Result<(), ReproducibilityError> {
        // Restore random state from captured state
        Ok(())
    }

    async fn restore_code_state(&self, state: &CodeState) -> Result<(), ReproducibilityError> {
        if let Some(ref commit) = state.git_commit {
            // Git checkout
            tokio::process::Command::new("git")
                .args(["checkout", commit])
                .output()
                .await
                .map_err(|e| ReproducibilityError::CodeRestoreFailed {
                    message: e.to_string(),
                })?;

            // Apply patches if any
            for patch in &state.patches {
                tokio::process::Command::new("git")
                    .args(["apply", "-"])
                    .stdin(std::process::Stdio::piped())
                    .spawn()
                    .map_err(|e| ReproducibilityError::CodeRestoreFailed {
                        message: e.to_string(),
                    })?;
            }
        }

        Ok(())
    }

    async fn verify_dataset_available(
        &self,
        dataset: &DatasetReference,
    ) -> Result<(), ReproducibilityError> {
        // Verify dataset is accessible
        // This would check with the data vault
        Ok(())
    }

    async fn compute_data_hashes(&self) -> Result<HashMap<String, String>, ReproducibilityError> {
        // Compute hashes for all referenced data
        Ok(HashMap::new())
    }

    fn compare_environments(
        &self,
        a: &EnvironmentSnapshot,
        b: &EnvironmentSnapshot,
    ) -> Vec<String> {
        let mut differences = Vec::new();

        if a.os.name != b.os.name {
            differences.push(format!("OS: {} vs {}", a.os.name, b.os.name));
        }

        if a.hardware.cpu_model != b.hardware.cpu_model {
            differences.push(format!(
                "CPU: {} vs {}",
                a.hardware.cpu_model, b.hardware.cpu_model
            ));
        }

        // Compare dependencies
        let a_packages: HashMap<_, _> = a
            .dependencies
            .packages
            .iter()
            .map(|p| (&p.name, &p.version))
            .collect();

        for package in &b.dependencies.packages {
            if let Some(version_a) = a_packages.get(&package.name) {
                if *version_a != &package.version {
                    differences.push(format!(
                        "Package {}: {} vs {}",
                        package.name, version_a, package.version
                    ));
                }
            } else {
                differences.push(format!("Package {} added in b", package.name));
            }
        }

        differences
    }

    fn generate_setup_commands(&self, state: &ExperimentState) -> Vec<String> {
        let mut commands = Vec::new();

        // Git checkout
        if let Some(ref commit) = state.code_state.git_commit {
            commands.push(format!("git checkout {}", commit));
        }

        // Install dependencies
        match state.environment.dependencies.format {
            DependencyFormat::CargoLock => {
                commands.push("cargo build --release".to_string());
            }
            DependencyFormat::PipFreeze => {
                commands.push("pip install -r requirements.txt".to_string());
            }
            DependencyFormat::CondaEnvironment => {
                commands.push("conda env create -f environment.yml".to_string());
            }
            _ => {}
        }

        commands
    }

    fn hash_environment(&self, env: &EnvironmentSnapshot) -> String {
        use sha2::{Sha256, Digest};
        let bytes = serde_json::to_vec(env).unwrap_or_default();
        hex::encode(Sha256::digest(&bytes))
    }

    fn hash_data_state(&self, data: &DataState) -> String {
        use sha2::{Sha256, Digest};
        let bytes = serde_json::to_vec(data).unwrap_or_default();
        hex::encode(Sha256::digest(&bytes))
    }

    fn hash_configuration(&self, config: &ConfigurationState) -> String {
        use sha2::{Sha256, Digest};
        let bytes = serde_json::to_vec(config).unwrap_or_default();
        hex::encode(Sha256::digest(&bytes))
    }

    fn hash_results(&self, run: &ExperimentRun) -> String {
        use sha2::{Sha256, Digest};
        let bytes = serde_json::to_vec(&run.metrics).unwrap_or_default();
        hex::encode(Sha256::digest(&bytes))
    }

    fn sign_certificate(
        &self,
        state: &ExperimentState,
        run: &ExperimentRun,
    ) -> Result<String, ReproducibilityError> {
        // Sign with system key
        use sha2::{Sha256, Digest};
        let data = format!(
            "{}:{}:{}:{}",
            state.id, state.checksum, run.id.0, Utc::now().timestamp()
        );
        Ok(hex::encode(Sha256::digest(data.as_bytes())))
    }
}

/// Code snapshot service
pub struct CodeSnapshot {
    repo_path: std::path::PathBuf,
}

impl CodeSnapshot {
    pub fn new(repo_path: std::path::PathBuf) -> Self {
        Self { repo_path }
    }

    pub async fn capture(&self) -> Result<CodeState, ReproducibilityError> {
        // Get git commit
        let commit_output = tokio::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&self.repo_path)
            .output()
            .await?;

        let git_commit = if commit_output.status.success() {
            Some(String::from_utf8_lossy(&commit_output.stdout).trim().to_string())
        } else {
            None
        };

        // Get branch
        let branch_output = tokio::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(&self.repo_path)
            .output()
            .await?;

        let git_branch = if branch_output.status.success() {
            Some(String::from_utf8_lossy(&branch_output.stdout).trim().to_string())
        } else {
            None
        };

        // Check if dirty
        let status_output = tokio::process::Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&self.repo_path)
            .output()
            .await?;

        let git_dirty = !status_output.stdout.is_empty();

        // Get patches for uncommitted changes
        let patches = if git_dirty {
            let diff_output = tokio::process::Command::new("git")
                .args(["diff"])
                .current_dir(&self.repo_path)
                .output()
                .await?;

            vec![CodePatch {
                file_path: "working_tree".to_string(),
                diff: String::from_utf8_lossy(&diff_output.stdout).to_string(),
            }]
        } else {
            Vec::new()
        };

        // Compute code hash
        let code_hash = self.compute_code_hash().await?;

        Ok(CodeState {
            git_commit,
            git_branch,
            git_dirty,
            code_hash,
            patches,
        })
    }

    async fn compute_code_hash(&self) -> Result<String, ReproducibilityError> {
        // Hash all source files
        use sha2::{Sha256, Digest};
        let output = tokio::process::Command::new("git")
            .args(["ls-files", "-s"])
            .current_dir(&self.repo_path)
            .output()
            .await?;

        Ok(hex::encode(Sha256::digest(&output.stdout)))
    }
}

/// State store trait
#[async_trait]
pub trait StateStore: Send + Sync {
    async fn save(&self, state: &ExperimentState) -> Result<(), ReproducibilityError>;
    async fn get(&self, id: Uuid) -> Result<ExperimentState, ReproducibilityError>;
    async fn get_for_run(&self, run_id: RunId) -> Result<ExperimentState, ReproducibilityError>;
    async fn list_for_experiment(&self, experiment_id: ExperimentId) -> Result<Vec<ExperimentState>, ReproducibilityError>;
}

/// Reproducibility types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub state_id: Uuid,
    pub is_reproducible: bool,
    pub issues: Vec<ValidationIssue>,
    pub warnings: Vec<ValidationIssue>,
    pub validated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub severity: IssueSeverity,
    pub category: IssueCategory,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueCategory {
    Code,
    Environment,
    Data,
    Configuration,
    Randomness,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayResult {
    pub state_id: Uuid,
    pub environment_differences: Vec<String>,
    pub ready_to_execute: bool,
    pub setup_commands: Vec<String>,
    pub replayed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateComparison {
    pub state_a_id: Uuid,
    pub state_b_id: Uuid,
    pub are_equivalent: bool,
    pub differences: Vec<StateDifference>,
    pub compared_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDifference {
    pub category: DifferenceCategory,
    pub field: String,
    pub value_a: String,
    pub value_b: String,
    pub impact: DifferenceImpact,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DifferenceCategory {
    Code,
    Environment,
    Data,
    Configuration,
    Randomness,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DifferenceImpact {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReproducibilityCertificate {
    pub id: Uuid,
    pub state_id: Uuid,
    pub experiment_id: ExperimentId,
    pub run_id: RunId,
    pub validation_report: ValidationReport,
    pub environment_hash: String,
    pub code_hash: String,
    pub data_hash: String,
    pub configuration_hash: String,
    pub results_hash: String,
    pub issued_at: DateTime<Utc>,
    pub signature: String,
}

/// Reproducibility errors
#[derive(Debug, thiserror::Error)]
pub enum ReproducibilityError {
    #[error("Checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("Validation failed: {issues:?}")]
    ValidationFailed { issues: Vec<ValidationIssue> },

    #[error("Code restore failed: {message}")]
    CodeRestoreFailed { message: String },

    #[error("State not found: {id}")]
    StateNotFound { id: Uuid },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Environment capture error: {0}")]
    EnvironmentCapture(#[from] TrackerError),
}
```

---

## 7. Workflow Orchestration

### 7.1 Workflow Engine

```rust
//! Workflow orchestration for research pipelines
//! Provides DAG-based workflow execution with checkpointing

/// Workflow orchestrator
pub struct WorkflowOrchestrator {
    workflow_store: Arc<dyn WorkflowStore>,
    run_store: Arc<dyn WorkflowRunStore>,
    executor: Arc<WorkflowExecutor>,
    checkpoint_manager: Arc<CheckpointManager>,
    scheduler: Arc<WorkflowScheduler>,
    event_publisher: Arc<dyn EventPublisher>,
    config: OrchestratorConfig,
}

#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    pub max_concurrent_workflows: usize,
    pub max_concurrent_steps: usize,
    pub default_timeout: std::time::Duration,
    pub checkpoint_interval: std::time::Duration,
    pub retry_policy: RetryPolicy,
}

/// Workflow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: WorkflowId,
    pub name: String,
    pub description: Option<String>,
    pub version: SemanticVersion,
    pub steps: Vec<WorkflowStep>,
    pub parameters: Vec<WorkflowParameter>,
    pub triggers: Vec<WorkflowTrigger>,
    pub default_timeout: Option<std::time::Duration>,
    pub retry_policy: Option<RetryPolicy>,
    pub created_at: DateTime<Utc>,
    pub created_by: UserId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: String,
    pub name: String,
    pub step_type: StepType,
    pub config: StepConfig,
    pub dependencies: Vec<String>,
    pub condition: Option<StepCondition>,
    pub retry: Option<StepRetry>,
    pub timeout: Option<std::time::Duration>,
    pub outputs: Vec<StepOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepType {
    Experiment,
    Benchmark,
    Transform,
    Conditional,
    Parallel,
    Loop,
    SubWorkflow,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepConfig {
    pub parameters: HashMap<String, serde_json::Value>,
    pub inputs: HashMap<String, InputRef>,
    pub resources: Option<ResourceRequirements>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputRef {
    Parameter(String),
    StepOutput { step_id: String, output_name: String },
    Literal(serde_json::Value),
    Expression(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepCondition {
    pub expression: String,
    pub on_false: ConditionAction,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionAction {
    Skip,
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepRetry {
    pub max_attempts: u32,
    pub delay: std::time::Duration,
    pub backoff_multiplier: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepOutput {
    pub name: String,
    pub output_type: OutputType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputType {
    Scalar,
    Artifact,
    Dataset,
    Metrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowParameter {
    pub name: String,
    pub parameter_type: ParameterType,
    pub required: bool,
    pub default: Option<serde_json::Value>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterType {
    String,
    Integer,
    Float,
    Boolean,
    List,
    Object,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowTrigger {
    Manual,
    Schedule { cron: String },
    Event { event_type: String, filter: Option<String> },
    Webhook { path: String },
}

/// Workflow run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRun {
    pub id: WorkflowRunId,
    pub workflow_id: WorkflowId,
    pub workflow_version: SemanticVersion,
    pub status: WorkflowRunStatus,
    pub parameters: HashMap<String, serde_json::Value>,
    pub step_states: HashMap<String, StepState>,
    pub outputs: HashMap<String, serde_json::Value>,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub created_by: UserId,
    pub error: Option<WorkflowError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowRunStatus {
    Pending,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepState {
    pub step_id: String,
    pub status: StepStatus,
    pub attempt: u32,
    pub inputs: HashMap<String, serde_json::Value>,
    pub outputs: HashMap<String, serde_json::Value>,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
    pub logs_uri: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowError {
    pub step_id: Option<String>,
    pub error_type: String,
    pub message: String,
    pub recoverable: bool,
}

impl WorkflowOrchestrator {
    pub fn new(
        workflow_store: Arc<dyn WorkflowStore>,
        run_store: Arc<dyn WorkflowRunStore>,
        executor: Arc<WorkflowExecutor>,
        checkpoint_manager: Arc<CheckpointManager>,
        scheduler: Arc<WorkflowScheduler>,
        event_publisher: Arc<dyn EventPublisher>,
        config: OrchestratorConfig,
    ) -> Self {
        Self {
            workflow_store,
            run_store,
            executor,
            checkpoint_manager,
            scheduler,
            event_publisher,
            config,
        }
    }

    /// Submit a workflow for execution
    pub async fn submit(
        &self,
        workflow_id: WorkflowId,
        parameters: HashMap<String, serde_json::Value>,
        user_id: UserId,
    ) -> Result<WorkflowRun, WorkflowError> {
        // Get workflow definition
        let workflow = self.workflow_store.get(workflow_id).await?;

        // Validate parameters
        self.validate_parameters(&workflow, &parameters)?;

        // Create run
        let run_id = WorkflowRunId(Uuid::new_v4());
        let now = Utc::now();

        let step_states: HashMap<String, StepState> = workflow
            .steps
            .iter()
            .map(|s| {
                (
                    s.id.clone(),
                    StepState {
                        step_id: s.id.clone(),
                        status: StepStatus::Pending,
                        attempt: 0,
                        inputs: HashMap::new(),
                        outputs: HashMap::new(),
                        started_at: None,
                        ended_at: None,
                        error: None,
                        logs_uri: None,
                    },
                )
            })
            .collect();

        let run = WorkflowRun {
            id: run_id,
            workflow_id,
            workflow_version: workflow.version.clone(),
            status: WorkflowRunStatus::Pending,
            parameters,
            step_states,
            outputs: HashMap::new(),
            started_at: None,
            ended_at: None,
            created_at: now,
            created_by: user_id,
            error: None,
        };

        // Save run
        self.run_store.save(&run).await?;

        // Queue for execution
        self.scheduler.enqueue(run_id).await?;

        Ok(run)
    }

    /// Execute a workflow run
    pub async fn execute(&self, run_id: WorkflowRunId) -> Result<WorkflowRun, WorkflowError> {
        let mut run = self.run_store.get(run_id).await?;
        let workflow = self.workflow_store.get(run.workflow_id).await?;

        // Update status
        run.status = WorkflowRunStatus::Running;
        run.started_at = Some(Utc::now());
        self.run_store.save(&run).await?;

        // Build execution DAG
        let dag = self.build_dag(&workflow)?;

        // Execute steps in topological order
        let mut completed_steps = HashSet::new();

        while completed_steps.len() < workflow.steps.len() {
            // Find ready steps
            let ready_steps: Vec<_> = dag
                .iter()
                .filter(|(step_id, deps)| {
                    !completed_steps.contains(*step_id)
                        && deps.iter().all(|d| completed_steps.contains(d))
                })
                .map(|(step_id, _)| step_id.clone())
                .collect();

            if ready_steps.is_empty() && completed_steps.len() < workflow.steps.len() {
                // Deadlock or all remaining steps failed
                break;
            }

            // Execute ready steps in parallel
            let mut handles = Vec::new();

            for step_id in ready_steps.iter().take(self.config.max_concurrent_steps) {
                let step = workflow
                    .steps
                    .iter()
                    .find(|s| &s.id == step_id)
                    .unwrap()
                    .clone();

                let executor = self.executor.clone();
                let run_clone = run.clone();
                let checkpoint_manager = self.checkpoint_manager.clone();
                let step_id = step_id.clone();

                let handle = tokio::spawn(async move {
                    let result = executor
                        .execute_step(&step, &run_clone, &checkpoint_manager)
                        .await;
                    (step_id, result)
                });

                handles.push(handle);
            }

            // Wait for step results
            for handle in handles {
                let (step_id, result) = handle.await.map_err(|e| WorkflowError {
                    step_id: None,
                    error_type: "task_join".to_string(),
                    message: e.to_string(),
                    recoverable: false,
                })?;

                match result {
                    Ok(state) => {
                        run.step_states.insert(step_id.clone(), state);
                        if run.step_states[&step_id].status == StepStatus::Completed
                            || run.step_states[&step_id].status == StepStatus::Skipped
                        {
                            completed_steps.insert(step_id);
                        }
                    }
                    Err(e) => {
                        run.step_states.get_mut(&step_id).unwrap().status = StepStatus::Failed;
                        run.step_states.get_mut(&step_id).unwrap().error = Some(e.message.clone());

                        if !e.recoverable {
                            run.status = WorkflowRunStatus::Failed;
                            run.error = Some(e);
                            run.ended_at = Some(Utc::now());
                            self.run_store.save(&run).await?;
                            return Err(run.error.clone().unwrap());
                        }
                    }
                }

                // Checkpoint
                self.checkpoint_manager.save_checkpoint(&run).await?;
                self.run_store.save(&run).await?;
            }
        }

        // Finalize
        run.status = if completed_steps.len() == workflow.steps.len() {
            WorkflowRunStatus::Completed
        } else {
            WorkflowRunStatus::Failed
        };
        run.ended_at = Some(Utc::now());

        // Collect outputs
        for step in &workflow.steps {
            for output in &step.outputs {
                if let Some(state) = run.step_states.get(&step.id) {
                    if let Some(value) = state.outputs.get(&output.name) {
                        run.outputs
                            .insert(format!("{}.{}", step.id, output.name), value.clone());
                    }
                }
            }
        }

        self.run_store.save(&run).await?;

        Ok(run)
    }

    /// Pause a running workflow
    pub async fn pause(&self, run_id: WorkflowRunId) -> Result<WorkflowRun, WorkflowError> {
        let mut run = self.run_store.get(run_id).await?;

        if run.status != WorkflowRunStatus::Running {
            return Err(WorkflowError {
                step_id: None,
                error_type: "invalid_state".to_string(),
                message: "Workflow is not running".to_string(),
                recoverable: false,
            });
        }

        run.status = WorkflowRunStatus::Paused;
        self.run_store.save(&run).await?;

        Ok(run)
    }

    /// Resume a paused workflow
    pub async fn resume(&self, run_id: WorkflowRunId) -> Result<WorkflowRun, WorkflowError> {
        let run = self.run_store.get(run_id).await?;

        if run.status != WorkflowRunStatus::Paused {
            return Err(WorkflowError {
                step_id: None,
                error_type: "invalid_state".to_string(),
                message: "Workflow is not paused".to_string(),
                recoverable: false,
            });
        }

        // Re-queue for execution
        self.scheduler.enqueue(run_id).await?;

        self.execute(run_id).await
    }

    /// Cancel a workflow
    pub async fn cancel(&self, run_id: WorkflowRunId) -> Result<WorkflowRun, WorkflowError> {
        let mut run = self.run_store.get(run_id).await?;

        if run.status == WorkflowRunStatus::Completed
            || run.status == WorkflowRunStatus::Cancelled
            || run.status == WorkflowRunStatus::Failed
        {
            return Err(WorkflowError {
                step_id: None,
                error_type: "invalid_state".to_string(),
                message: "Workflow already terminated".to_string(),
                recoverable: false,
            });
        }

        run.status = WorkflowRunStatus::Cancelled;
        run.ended_at = Some(Utc::now());

        // Cancel running steps
        for state in run.step_states.values_mut() {
            if state.status == StepStatus::Running {
                state.status = StepStatus::Cancelled;
                state.ended_at = Some(Utc::now());
            }
        }

        self.run_store.save(&run).await?;

        Ok(run)
    }

    /// Get workflow run status
    pub async fn get_status(&self, run_id: WorkflowRunId) -> Result<WorkflowRun, WorkflowError> {
        self.run_store.get(run_id).await
    }

    // Private helpers

    fn validate_parameters(
        &self,
        workflow: &Workflow,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> Result<(), WorkflowError> {
        for param in &workflow.parameters {
            if param.required && !parameters.contains_key(&param.name) && param.default.is_none() {
                return Err(WorkflowError {
                    step_id: None,
                    error_type: "validation".to_string(),
                    message: format!("Missing required parameter: {}", param.name),
                    recoverable: false,
                });
            }
        }
        Ok(())
    }

    fn build_dag(&self, workflow: &Workflow) -> Result<HashMap<String, Vec<String>>, WorkflowError> {
        let mut dag = HashMap::new();

        for step in &workflow.steps {
            dag.insert(step.id.clone(), step.dependencies.clone());
        }

        // Validate no cycles
        self.validate_dag(&dag)?;

        Ok(dag)
    }

    fn validate_dag(&self, dag: &HashMap<String, Vec<String>>) -> Result<(), WorkflowError> {
        // Topological sort to detect cycles
        let mut visited = HashSet::new();
        let mut in_stack = HashSet::new();

        fn visit(
            node: &str,
            dag: &HashMap<String, Vec<String>>,
            visited: &mut HashSet<String>,
            in_stack: &mut HashSet<String>,
        ) -> Result<(), WorkflowError> {
            if in_stack.contains(node) {
                return Err(WorkflowError {
                    step_id: Some(node.to_string()),
                    error_type: "cycle".to_string(),
                    message: "Cycle detected in workflow DAG".to_string(),
                    recoverable: false,
                });
            }

            if visited.contains(node) {
                return Ok(());
            }

            in_stack.insert(node.to_string());

            if let Some(deps) = dag.get(node) {
                for dep in deps {
                    visit(dep, dag, visited, in_stack)?;
                }
            }

            in_stack.remove(node);
            visited.insert(node.to_string());

            Ok(())
        }

        for node in dag.keys() {
            visit(node, dag, &mut visited, &mut in_stack)?;
        }

        Ok(())
    }
}

/// Workflow executor
pub struct WorkflowExecutor {
    experiment_tracker: Arc<ExperimentTracker>,
    benchmark_runner: Arc<BenchmarkRunner>,
    dataset_manager: Arc<DatasetManager>,
}

impl WorkflowExecutor {
    pub async fn execute_step(
        &self,
        step: &WorkflowStep,
        run: &WorkflowRun,
        checkpoint_manager: &CheckpointManager,
    ) -> Result<StepState, WorkflowError> {
        let mut state = StepState {
            step_id: step.id.clone(),
            status: StepStatus::Running,
            attempt: 0,
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            started_at: Some(Utc::now()),
            ended_at: None,
            error: None,
            logs_uri: None,
        };

        // Resolve inputs
        state.inputs = self.resolve_inputs(&step.config.inputs, run)?;

        // Check condition
        if let Some(ref condition) = step.condition {
            let should_run = self.evaluate_condition(&condition.expression, &state.inputs)?;
            if !should_run {
                state.status = StepStatus::Skipped;
                state.ended_at = Some(Utc::now());
                return Ok(state);
            }
        }

        // Execute with retries
        let max_attempts = step.retry.as_ref().map(|r| r.max_attempts).unwrap_or(1);
        let mut last_error = None;

        while state.attempt < max_attempts {
            state.attempt += 1;

            match self.execute_step_type(step, &state.inputs).await {
                Ok(outputs) => {
                    state.outputs = outputs;
                    state.status = StepStatus::Completed;
                    state.ended_at = Some(Utc::now());
                    return Ok(state);
                }
                Err(e) => {
                    last_error = Some(e);
                    if state.attempt < max_attempts {
                        if let Some(ref retry) = step.retry {
                            let delay = retry.delay.mul_f64(
                                retry.backoff_multiplier.powi(state.attempt as i32 - 1),
                            );
                            tokio::time::sleep(delay).await;
                        }
                    }
                }
            }
        }

        state.status = StepStatus::Failed;
        state.error = last_error.map(|e| e.message);
        state.ended_at = Some(Utc::now());

        Err(WorkflowError {
            step_id: Some(step.id.clone()),
            error_type: "execution".to_string(),
            message: state.error.clone().unwrap_or_default(),
            recoverable: false,
        })
    }

    async fn execute_step_type(
        &self,
        step: &WorkflowStep,
        inputs: &HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>, WorkflowError> {
        match &step.step_type {
            StepType::Experiment => self.execute_experiment_step(step, inputs).await,
            StepType::Benchmark => self.execute_benchmark_step(step, inputs).await,
            StepType::Transform => self.execute_transform_step(step, inputs).await,
            StepType::Conditional => self.execute_conditional_step(step, inputs).await,
            StepType::Parallel => self.execute_parallel_step(step, inputs).await,
            StepType::Loop => self.execute_loop_step(step, inputs).await,
            StepType::SubWorkflow => self.execute_subworkflow_step(step, inputs).await,
            StepType::Custom(name) => self.execute_custom_step(name, step, inputs).await,
        }
    }

    async fn execute_experiment_step(
        &self,
        _step: &WorkflowStep,
        _inputs: &HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>, WorkflowError> {
        // Execute experiment using tracker
        Ok(HashMap::new())
    }

    async fn execute_benchmark_step(
        &self,
        _step: &WorkflowStep,
        _inputs: &HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>, WorkflowError> {
        // Execute benchmark using runner
        Ok(HashMap::new())
    }

    async fn execute_transform_step(
        &self,
        _step: &WorkflowStep,
        _inputs: &HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>, WorkflowError> {
        // Execute data transformation
        Ok(HashMap::new())
    }

    async fn execute_conditional_step(
        &self,
        _step: &WorkflowStep,
        _inputs: &HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>, WorkflowError> {
        // Execute conditional logic
        Ok(HashMap::new())
    }

    async fn execute_parallel_step(
        &self,
        _step: &WorkflowStep,
        _inputs: &HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>, WorkflowError> {
        // Execute parallel branches
        Ok(HashMap::new())
    }

    async fn execute_loop_step(
        &self,
        _step: &WorkflowStep,
        _inputs: &HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>, WorkflowError> {
        // Execute loop
        Ok(HashMap::new())
    }

    async fn execute_subworkflow_step(
        &self,
        _step: &WorkflowStep,
        _inputs: &HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>, WorkflowError> {
        // Execute sub-workflow
        Ok(HashMap::new())
    }

    async fn execute_custom_step(
        &self,
        _name: &str,
        _step: &WorkflowStep,
        _inputs: &HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>, WorkflowError> {
        // Execute custom step type
        Ok(HashMap::new())
    }

    fn resolve_inputs(
        &self,
        input_refs: &HashMap<String, InputRef>,
        run: &WorkflowRun,
    ) -> Result<HashMap<String, serde_json::Value>, WorkflowError> {
        let mut resolved = HashMap::new();

        for (name, input_ref) in input_refs {
            let value = match input_ref {
                InputRef::Parameter(param_name) => run
                    .parameters
                    .get(param_name)
                    .cloned()
                    .ok_or_else(|| WorkflowError {
                        step_id: None,
                        error_type: "input_resolution".to_string(),
                        message: format!("Parameter not found: {}", param_name),
                        recoverable: false,
                    })?,
                InputRef::StepOutput { step_id, output_name } => run
                    .step_states
                    .get(step_id)
                    .and_then(|s| s.outputs.get(output_name))
                    .cloned()
                    .ok_or_else(|| WorkflowError {
                        step_id: Some(step_id.clone()),
                        error_type: "input_resolution".to_string(),
                        message: format!("Step output not found: {}.{}", step_id, output_name),
                        recoverable: false,
                    })?,
                InputRef::Literal(value) => value.clone(),
                InputRef::Expression(expr) => self.evaluate_expression(expr, run)?,
            };

            resolved.insert(name.clone(), value);
        }

        Ok(resolved)
    }

    fn evaluate_condition(
        &self,
        _expression: &str,
        _inputs: &HashMap<String, serde_json::Value>,
    ) -> Result<bool, WorkflowError> {
        // Evaluate condition expression
        Ok(true)
    }

    fn evaluate_expression(
        &self,
        _expression: &str,
        _run: &WorkflowRun,
    ) -> Result<serde_json::Value, WorkflowError> {
        // Evaluate expression
        Ok(serde_json::Value::Null)
    }
}

/// Checkpoint manager
pub struct CheckpointManager {
    store: Arc<dyn CheckpointStore>,
}

impl CheckpointManager {
    pub async fn save_checkpoint(&self, run: &WorkflowRun) -> Result<(), WorkflowError> {
        let checkpoint = Checkpoint {
            run_id: run.id,
            state: serde_json::to_value(run).map_err(|e| WorkflowError {
                step_id: None,
                error_type: "serialization".to_string(),
                message: e.to_string(),
                recoverable: false,
            })?,
            created_at: Utc::now(),
        };

        self.store.save(&checkpoint).await
    }

    pub async fn load_checkpoint(
        &self,
        run_id: WorkflowRunId,
    ) -> Result<Option<Checkpoint>, WorkflowError> {
        self.store.get(run_id).await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub run_id: WorkflowRunId,
    pub state: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Workflow scheduler
pub struct WorkflowScheduler {
    queue: Arc<tokio::sync::Mutex<std::collections::VecDeque<WorkflowRunId>>>,
}

impl WorkflowScheduler {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(tokio::sync::Mutex::new(std::collections::VecDeque::new())),
        }
    }

    pub async fn enqueue(&self, run_id: WorkflowRunId) -> Result<(), WorkflowError> {
        let mut queue = self.queue.lock().await;
        queue.push_back(run_id);
        Ok(())
    }

    pub async fn dequeue(&self) -> Option<WorkflowRunId> {
        let mut queue = self.queue.lock().await;
        queue.pop_front()
    }
}

/// Storage traits
#[async_trait]
pub trait WorkflowStore: Send + Sync {
    async fn save(&self, workflow: &Workflow) -> Result<(), WorkflowError>;
    async fn get(&self, id: WorkflowId) -> Result<Workflow, WorkflowError>;
    async fn list(&self, filter: WorkflowFilter) -> Result<Vec<Workflow>, WorkflowError>;
}

#[async_trait]
pub trait WorkflowRunStore: Send + Sync {
    async fn save(&self, run: &WorkflowRun) -> Result<(), WorkflowError>;
    async fn get(&self, id: WorkflowRunId) -> Result<WorkflowRun, WorkflowError>;
    async fn list_for_workflow(&self, workflow_id: WorkflowId) -> Result<Vec<WorkflowRun>, WorkflowError>;
}

#[async_trait]
pub trait CheckpointStore: Send + Sync {
    async fn save(&self, checkpoint: &Checkpoint) -> Result<(), WorkflowError>;
    async fn get(&self, run_id: WorkflowRunId) -> Result<Option<Checkpoint>, WorkflowError>;
}

#[derive(Debug, Clone, Default)]
pub struct WorkflowFilter {
    pub name: Option<String>,
    pub created_by: Option<UserId>,
    pub tags: Option<Vec<String>>,
}

use std::collections::HashSet;
```

---

## Document Metadata

| Field | Value |
|-------|-------|
| **Version** | 1.0.0 |
| **Status** | Draft |
| **SPARC Phase** | Pseudocode (Part 3 of 3) |
| **Created** | 2025-11-28 |
| **Ecosystem** | LLM DevOps |
| **Previous Parts** | Part 1: Core Data Models & Experiment Tracking |
|  | Part 2: Metric Benchmarking & Dataset Versioning |
| **Next Phase** | Architecture |

---

*This pseudocode document completes the SPARC Pseudocode phase for LLM-Research-Lab. The next phase will cover system architecture design including deployment topology, data flow diagrams, and infrastructure specifications.*
