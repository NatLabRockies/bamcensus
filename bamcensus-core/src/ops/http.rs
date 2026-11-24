use reqwest::Client;
use reqwest_middleware::ClientBuilder;
pub use reqwest_middleware::ClientWithMiddleware;
use reqwest_retry::RetryTransientMiddleware;
pub use reqwest_retry::{
    policies::{ExponentialBackoff, ExponentialBackoffBuilder},
    Jitter,
};

pub const DEFAULT_MAX_RETRIES: u32 = 3;

/// Retry configuration for ACS, LODES, and TIGER HTTP requests.
#[derive(Default)]
pub enum RetryConfig {
    /// Do not install retry middleware.
    Disabled,
    /// Use the crate's default backoff with three retries (four total attempts).
    #[default]
    Default,
    /// Use caller-specified intervals, jitter, base, and retry count.
    Custom {
        builder: ExponentialBackoffBuilder,
        max_retries: u32,
    },
}
/// Build an HTTP client with disabled, default, or custom retries.
///
/// ```
/// use bamcensus_core::ops::http::{build_client, ExponentialBackoff, RetryConfig};
/// use std::time::Duration;
///
/// let no_retry = build_client(RetryConfig::Disabled);
/// let defaults = build_client(RetryConfig::Default);
/// let custom = build_client(RetryConfig::Custom {
///     builder: ExponentialBackoff::builder()
///         .retry_bounds(Duration::from_millis(500), Duration::from_secs(30))
///         .base(2),
///     max_retries: 5,
/// });
/// ```
pub fn build_client(retries: RetryConfig) -> ClientWithMiddleware {
    build_client_from(Client::new(), retries)
}

/// Preserve an existing client's transport settings and configure retries.
pub fn build_client_from(client: Client, retries: RetryConfig) -> ClientWithMiddleware {
    let builder = ClientBuilder::new(client);
    let policy = match retries {
        RetryConfig::Disabled => return builder.build(),
        RetryConfig::Default => {
            ExponentialBackoff::builder().build_with_max_retries(DEFAULT_MAX_RETRIES)
        }
        RetryConfig::Custom {
            builder,
            max_retries,
        } => builder.build_with_max_retries(max_retries),
    };
    builder
        .with(RetryTransientMiddleware::new_with_policy(policy))
        .build()
}

/// Builds a `ClientWithMiddleware` configured with exponential-backoff retries
/// for transient HTTP and network failures.
pub fn build_retry_client() -> ClientWithMiddleware {
    build_client(RetryConfig::Default)
}

/// Wraps an existing `reqwest::Client` in a `ClientWithMiddleware` configured
/// with exponential-backoff retries for up to `max_retries` retries after
/// the initial attempt. Use [`build_client_from`] to customize the backoff.
pub fn build_retry_client_from(client: Client, max_retries: u32) -> ClientWithMiddleware {
    build_client_from(
        client,
        RetryConfig::Custom {
            builder: ExponentialBackoff::builder(),
            max_retries,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use std::time::Duration;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn retries_transient_server_errors_and_succeeds() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let attempts = Arc::new(AtomicUsize::new(0));
        let server_attempts = attempts.clone();

        let server = tokio::spawn(async move {
            for attempt in 0..3 {
                let (mut socket, _) = listener.accept().await.unwrap();
                server_attempts.fetch_add(1, Ordering::SeqCst);
                let mut reader = BufReader::new(&mut socket);
                let mut line = String::new();
                loop {
                    line.clear();
                    assert!(reader.read_line(&mut line).await.unwrap() > 0);
                    if line == "\r\n" {
                        break;
                    }
                }
                let response = if attempt < 2 {
                    "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                } else {
                    "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok"
                };
                socket.write_all(response.as_bytes()).await.unwrap();
            }
        });

        let client = build_client(RetryConfig::Custom {
            builder: ExponentialBackoff::builder()
                .retry_bounds(Duration::from_millis(1), Duration::from_millis(10)),
            max_retries: 2,
        });

        let resp = client.get(&url).send().await.unwrap();
        assert_eq!(resp.status().as_u16(), 200);
        assert_eq!(resp.text().await.unwrap(), "ok");
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
        server.await.unwrap();
    }

    async fn assert_attempts(config: RetryConfig, status: u16, expected_attempts: usize) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let attempts = Arc::new(AtomicUsize::new(0));
        let server_attempts = attempts.clone();
        let server = tokio::spawn(async move {
            for _ in 0..expected_attempts {
                let (mut socket, _) = listener.accept().await.unwrap();
                server_attempts.fetch_add(1, Ordering::SeqCst);
                let mut reader = BufReader::new(&mut socket);
                let mut line = String::new();
                loop {
                    line.clear();
                    assert!(reader.read_line(&mut line).await.unwrap() > 0);
                    if line == "\r\n" {
                        break;
                    }
                }
                socket
                    .write_all(
                        format!(
                            "HTTP/1.1 {status} Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                        )
                        .as_bytes(),
                    )
                    .await
                    .unwrap();
            }
        });
        let client = build_client(config);
        let result = tokio::time::timeout(Duration::from_secs(15), async {
            let response = client.get(url).send().await.unwrap();
            server.await.unwrap();
            response
        })
        .await
        .unwrap();
        assert_eq!(result.status().as_u16(), status);
        assert_eq!(attempts.load(Ordering::SeqCst), expected_attempts);
    }

    #[tokio::test]
    async fn disabled_retries_return_first_transient_response() {
        assert_attempts(RetryConfig::Disabled, 503, 1).await;
    }

    #[tokio::test]
    async fn default_retries_stop_after_four_attempts() {
        assert_attempts(RetryConfig::Default, 503, 4).await;
    }

    #[tokio::test]
    async fn custom_retry_limit_and_non_transient_responses_are_respected() {
        for (max_retries, status, expected_attempts) in [(1, 503, 2), (0, 503, 1), (3, 404, 1)] {
            assert_attempts(
                RetryConfig::Custom {
                    builder: ExponentialBackoff::builder()
                        .retry_bounds(Duration::from_millis(1), Duration::from_millis(10)),
                    max_retries,
                },
                status,
                expected_attempts,
            )
            .await;
        }
    }
}
