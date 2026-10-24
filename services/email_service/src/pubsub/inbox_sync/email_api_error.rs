use email_api_client::domain::models::{EmailApiError, RateLimitOrigin};
use models_email::gmail::inbox_sync::{InboxSyncOperation, InboxSyncPubsubMessage};
use models_email::service::pubsub::{DetailedError, FailureReason, ProcessingError};

use crate::pubsub::context::PubSubContext;
use crate::pubsub::rate_limit_backoff::next_retry_delay;

#[cfg(test)]
mod test;

#[derive(Debug, PartialEq, Eq)]
enum RateLimitRoute {
    EnqueueRetry,
    RetryAfterVisibilityTimeout,
}

fn rate_limit_route(retry_worker: bool) -> RateLimitRoute {
    if retry_worker {
        RateLimitRoute::RetryAfterVisibilityTimeout
    } else {
        RateLimitRoute::EnqueueRetry
    }
}

pub(crate) async fn handle_operation_error(
    ctx: &PubSubContext,
    sqs_message: &aws_sdk_sqs::types::Message,
    link_id: uuid::Uuid,
    operation: InboxSyncOperation,
    error: EmailApiError,
) -> ProcessingError {
    let EmailApiError::RateLimited {
        retry_after,
        origin,
    } = error
    else {
        return processing_error(error);
    };
    let receive_count = sqs_worker::receive_count(sqs_message).unwrap_or(1);
    let delay = next_retry_delay(ctx.retry_worker, receive_count, origin, retry_after);

    if rate_limit_route(ctx.retry_worker) == RateLimitRoute::RetryAfterVisibilityTimeout {
        // On failure the queue's own visibility timeout still redelivers it.
        let delayed = ctx
            .sqs_worker
            .change_message_visibility(sqs_message, delay)
            .await
            .is_ok();
        log_deferral(
            link_id,
            origin,
            receive_count,
            true,
            delayed.then_some(delay.as_secs()),
        );
        return processing_error(error);
    }

    let message = InboxSyncPubsubMessage { link_id, operation };
    if let Err(enqueue_error) = ctx
        .sqs_client
        .enqueue_gmail_retry_inbox_sync_notification(message, delay)
        .await
    {
        return ProcessingError::Retryable(DetailedError {
            reason: FailureReason::SqsEnqueueFailed,
            source: enqueue_error.context("Failed to enqueue inbox sync retry message"),
        });
    }
    log_deferral(link_id, origin, receive_count, false, Some(delay.as_secs()));

    non_retryable(error)
}

/// Gate refusals log at DEBUG: they are frequent and cost Gmail nothing.
fn log_deferral(
    link_id: uuid::Uuid,
    origin: RateLimitOrigin,
    receive_count: u32,
    retry_worker: bool,
    delay_seconds: Option<u64>,
) {
    match origin {
        RateLimitOrigin::Provider => tracing::info!(
            %link_id,
            origin = ?origin,
            receive_count,
            retry_worker,
            delay_seconds,
            "Gmail API rate limited, inbox sync message will be retried after a delay"
        ),
        RateLimitOrigin::Local => tracing::debug!(
            %link_id,
            origin = ?origin,
            receive_count,
            retry_worker,
            delay_seconds,
            "Gmail API rate limited, inbox sync message will be retried after a delay"
        ),
    }
}

fn processing_error(error: EmailApiError) -> ProcessingError {
    if error.is_transient() || matches!(error, EmailApiError::RateLimited { .. }) {
        ProcessingError::Retryable(detail(error))
    } else {
        non_retryable(error)
    }
}

fn non_retryable(error: EmailApiError) -> ProcessingError {
    ProcessingError::NonRetryable(detail(error))
}

fn detail(error: EmailApiError) -> DetailedError {
    // Exhaustive: a new EmailApiError variant must force a policy decision
    // here rather than silently falling into a catch-all.
    let reason = match error {
        EmailApiError::RateLimited { .. } => FailureReason::GmailApiRateLimited,
        EmailApiError::AuthRequired => FailureReason::AccessTokenFetchFailed,
        EmailApiError::OutdatedCursor => FailureReason::OutdatedHistoryId,
        EmailApiError::Forbidden
        | EmailApiError::NotFound
        | EmailApiError::Conflict
        | EmailApiError::Transient { .. }
        | EmailApiError::SendRejected { .. }
        | EmailApiError::Permanent { .. } => FailureReason::GmailApiFailed,
    };
    DetailedError {
        reason,
        source: anyhow::Error::new(error),
    }
}
