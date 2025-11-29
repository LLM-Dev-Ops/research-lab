use async_trait::async_trait;
use crate::error::Result;

#[async_trait]
pub trait Repository<T, ID> {
    async fn find_by_id(&self, id: &ID) -> Result<Option<T>>;
    async fn save(&self, entity: &T) -> Result<T>;
    async fn delete(&self, id: &ID) -> Result<()>;
}

#[async_trait]
pub trait EventPublisher {
    async fn publish<E: Send + Sync>(&self, event: E) -> Result<()>;
}

#[async_trait]
pub trait MetricCalculator {
    type Input;
    type Output;

    async fn calculate(&self, input: Self::Input) -> Result<Self::Output>;
}
