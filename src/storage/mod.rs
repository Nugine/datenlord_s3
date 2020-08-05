use anyhow::Result;
use async_trait::async_trait;

use rusoto_s3::{
    CreateBucketOutput, CreateBucketRequest, DeleteBucketRequest, DeleteObjectOutput,
    DeleteObjectRequest, GetObjectOutput, GetObjectRequest, HeadBucketRequest, ListBucketsOutput,
    PutObjectOutput, PutObjectRequest,
};

#[allow(clippy::module_name_repetitions)]
#[async_trait]
pub trait S3Storage {
    async fn get_object(&self, input: GetObjectRequest) -> Result<GetObjectOutput>;
    async fn put_object(&self, input: PutObjectRequest) -> Result<PutObjectOutput>;
    async fn delete_object(&self, input: DeleteObjectRequest) -> Result<DeleteObjectOutput>;

    async fn create_bucket(&self, input: CreateBucketRequest) -> Result<CreateBucketOutput>;
    async fn delete_bucket(&self, input: DeleteBucketRequest) -> Result<()>;
    async fn head_bucket(&self, input: HeadBucketRequest) -> Result<()>;
    async fn list_buckets(&self) -> Result<ListBucketsOutput>;

    // TODO: add more operations
}
