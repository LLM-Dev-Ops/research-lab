use aws_sdk_s3::Client as S3Client;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub s3: S3Client,
}
