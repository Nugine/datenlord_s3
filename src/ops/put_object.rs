//! [`PutObject`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_PutObject.html)

use crate::utils::{Apply, RequestExt, ResponseExt};
use crate::{error::S3Result, multipart::Multipart};
use crate::{
    output::{wrap_output, S3Output},
    utils::OrderedHeaders,
};
use crate::{Body, BoxStdError, Request, Response};

use futures::stream::StreamExt;
use std::{collections::HashMap, io, mem};

use crate::dto::{ByteStream, PutObjectError, PutObjectOutput, PutObjectRequest};
use crate::headers::names::{
    CONTENT_MD5, X_AMZ_ACL, X_AMZ_EXPIRATION, X_AMZ_GRANT_FULL_CONTROL, X_AMZ_GRANT_READ,
    X_AMZ_GRANT_READ_ACP, X_AMZ_GRANT_WRITE_ACP, X_AMZ_OBJECT_LOCK_LEGAL_HOLD,
    X_AMZ_OBJECT_LOCK_MODE, X_AMZ_OBJECT_LOCK_RETAIN_UNTIL_DATE, X_AMZ_REQUEST_CHARGED,
    X_AMZ_REQUEST_PAYER, X_AMZ_SERVER_SIDE_ENCRYPTION, X_AMZ_SERVER_SIDE_ENCRYPTION_AWS_KMS_KEY_ID,
    X_AMZ_SERVER_SIDE_ENCRYPTION_CONTEXT, X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM,
    X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY, X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY_MD5,
    X_AMZ_STORAGE_CLASS, X_AMZ_TAGGING, X_AMZ_VERSION_ID, X_AMZ_WEBSITE_REDIRECT_LOCATION,
};
use hyper::header::{
    CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_ENCODING, CONTENT_LANGUAGE, CONTENT_LENGTH,
    CONTENT_TYPE, ETAG, EXPIRES,
};

/// transform stream
fn transform_stream(body: Body) -> ByteStream {
    body.map(|try_chunk| {
        try_chunk.map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Error obtaining chunk: {}", e),
            )
        })
    })
    .apply(ByteStream::new)
}

/// extract from multipart
fn extract_from_multipart(
    input: &mut PutObjectRequest,
    mut multipart: Multipart,
) -> Result<(), BoxStdError> {
    multipart.assign_from_optional_field("acl", &mut input.acl)?;
    multipart.assign_from_optional_field("content-type", &mut input.content_type)?;
    multipart.assign_from_optional_field("expires", &mut input.expires)?;
    multipart.assign_from_optional_field("tagging", &mut input.tagging)?;
    multipart.assign_from_optional_field("x-amz-storage-class", &mut input.storage_class)?;

    let mut metadata: HashMap<String, String> = HashMap::new();
    for &mut (ref mut name, ref mut value) in &mut multipart.fields {
        name.make_ascii_lowercase();
        let meta_prefix = "x-amz-meta-";
        if name.starts_with(meta_prefix) {
            let (_, meta_key) = name.split_at(meta_prefix.len());
            if !meta_key.is_empty() {
                let _ = metadata.insert(meta_key.to_owned(), mem::take(value));
            }
        }
    }
    if !metadata.is_empty() {
        input.metadata = Some(metadata);
    }
    // TODO: how to handle the other fields?

    let file_stream = multipart.file.stream;

    input.body = file_stream
        .apply(Body::wrap_stream)
        .apply(transform_stream)
        .apply(Some);

    Ok(())
}

/// extract operation request
pub fn extract(
    req: &Request,
    body: Body,
    bucket: &str,
    key: &str,
    multipart: Option<Multipart>,
    headers: &OrderedHeaders<'_>,
) -> Result<PutObjectRequest, BoxStdError> {
    let mut input: PutObjectRequest = PutObjectRequest {
        bucket: bucket.into(),
        key: key.into(),
        body: None,
        ..PutObjectRequest::default()
    };

    if let Some(content_length) = req.get_header_str(CONTENT_LENGTH)? {
        input.content_length = content_length.parse::<i64>()?.apply(Some)
    }

    req.assign_from_optional_header(&*X_AMZ_ACL, &mut input.acl)?;
    req.assign_from_optional_header(CACHE_CONTROL, &mut input.cache_control)?;
    req.assign_from_optional_header(CONTENT_DISPOSITION, &mut input.content_disposition)?;
    req.assign_from_optional_header(CONTENT_ENCODING, &mut input.content_encoding)?;
    req.assign_from_optional_header(CONTENT_LANGUAGE, &mut input.content_language)?;
    req.assign_from_optional_header(&*CONTENT_MD5, &mut input.content_md5)?;
    req.assign_from_optional_header(CONTENT_TYPE, &mut input.content_type)?;
    req.assign_from_optional_header(EXPIRES, &mut input.expires)?;
    req.assign_from_optional_header(&*X_AMZ_GRANT_FULL_CONTROL, &mut input.grant_full_control)?;
    req.assign_from_optional_header(&*X_AMZ_GRANT_READ, &mut input.grant_read)?;
    req.assign_from_optional_header(&*X_AMZ_GRANT_READ_ACP, &mut input.grant_read_acp)?;
    req.assign_from_optional_header(&*X_AMZ_GRANT_WRITE_ACP, &mut input.grant_write_acp)?;
    req.assign_from_optional_header(
        &*X_AMZ_SERVER_SIDE_ENCRYPTION,
        &mut input.server_side_encryption,
    )?;
    req.assign_from_optional_header(&*X_AMZ_STORAGE_CLASS, &mut input.storage_class)?;
    req.assign_from_optional_header(
        &*X_AMZ_WEBSITE_REDIRECT_LOCATION,
        &mut input.website_redirect_location,
    )?;
    req.assign_from_optional_header(
        &*X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM,
        &mut input.sse_customer_algorithm,
    )?;
    req.assign_from_optional_header(
        &*X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY,
        &mut input.sse_customer_key,
    )?;
    req.assign_from_optional_header(
        &*X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY_MD5,
        &mut input.sse_customer_key_md5,
    )?;
    req.assign_from_optional_header(
        &*X_AMZ_SERVER_SIDE_ENCRYPTION_AWS_KMS_KEY_ID,
        &mut input.ssekms_key_id,
    )?;
    req.assign_from_optional_header(
        &*X_AMZ_SERVER_SIDE_ENCRYPTION_CONTEXT,
        &mut input.ssekms_encryption_context,
    )?;
    req.assign_from_optional_header(&*X_AMZ_REQUEST_PAYER, &mut input.request_payer)?;
    req.assign_from_optional_header(&*X_AMZ_TAGGING, &mut input.tagging)?;
    req.assign_from_optional_header(&*X_AMZ_OBJECT_LOCK_MODE, &mut input.object_lock_mode)?;
    req.assign_from_optional_header(
        &*X_AMZ_OBJECT_LOCK_RETAIN_UNTIL_DATE,
        &mut input.object_lock_retain_until_date,
    )?;
    req.assign_from_optional_header(
        &*X_AMZ_OBJECT_LOCK_LEGAL_HOLD,
        &mut input.object_lock_legal_hold_status,
    )?;

    let mut metadata: HashMap<String, String> = HashMap::new();
    for &(name, value) in headers.as_ref() {
        let meta_prefix = "x-amz-meta-";
        if name.starts_with(meta_prefix) {
            let (_, meta_key) = name.split_at(meta_prefix.len());
            if !meta_key.is_empty() {
                let _ = metadata.insert(meta_key.to_owned(), value.to_owned());
            }
        }
    }
    if !metadata.is_empty() {
        input.metadata = Some(metadata);
    }

    match multipart {
        None => input.body = body.apply(transform_stream).apply(Some),
        Some(multipart) => extract_from_multipart(&mut input, multipart)?,
    };

    Ok(input)
}

impl S3Output for PutObjectOutput {
    fn try_into_response(self) -> S3Result<Response> {
        wrap_output(|res| {
            res.set_optional_header(|| X_AMZ_EXPIRATION.clone(), self.expiration)?;
            res.set_optional_header(|| ETAG, self.e_tag)?;
            res.set_optional_header(
                || X_AMZ_SERVER_SIDE_ENCRYPTION.clone(),
                self.server_side_encryption,
            )?;
            res.set_optional_header(|| X_AMZ_VERSION_ID.clone(), self.version_id)?;
            res.set_optional_header(
                || X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM.clone(),
                self.sse_customer_algorithm,
            )?;
            res.set_optional_header(
                || X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY_MD5.clone(),
                self.sse_customer_key_md5,
            )?;
            res.set_optional_header(
                || X_AMZ_SERVER_SIDE_ENCRYPTION_AWS_KMS_KEY_ID.clone(),
                self.ssekms_key_id,
            )?;
            res.set_optional_header(
                || X_AMZ_SERVER_SIDE_ENCRYPTION_CONTEXT.clone(),
                self.ssekms_encryption_context,
            )?;
            res.set_optional_header(|| X_AMZ_REQUEST_CHARGED.clone(), self.request_charged)?;
            Ok(())
        })
    }
}

impl S3Output for PutObjectError {
    fn try_into_response(self) -> S3Result<Response> {
        match self {}
    }
}
