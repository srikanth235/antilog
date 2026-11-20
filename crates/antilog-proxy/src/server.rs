//! One listener for both jobs: a CONNECT is the egress tunnel, anything else is a gateway call.

use std::convert::Infallible;

use axum::body::Body;
use axum::http::{Method, Request, Response};
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tower::ServiceExt;

use crate::{Backend, Shared, Time, Tokens, egress, gateway};

/// Serves until the listener fails. Each connection is its own task.
pub async fn serve<S, C, K>(listener: TcpListener, proxy: Shared<S, C, K>) -> std::io::Result<()>
where
    S: Backend,
    C: Tokens,
    K: Time,
{
    let router = gateway::router(proxy.clone());
    loop {
        let (stream, _) = listener.accept().await?;
        let (proxy, router) = (proxy.clone(), router.clone());
        tokio::spawn(async move {
            let service = service_fn(move |request: Request<Incoming>| {
                let (proxy, router) = (proxy.clone(), router.clone());
                async move {
                    let response: Response<Body> = if request.method() == Method::CONNECT {
                        egress::connect(proxy, request).await
                    } else {
                        match router.oneshot(request.map(Body::new)).await {
                            Ok(response) => response,
                            Err(never) => match never {},
                        }
                    };
                    Ok::<_, Infallible>(response)
                }
            });
            let served = hyper::server::conn::http1::Builder::new()
                .serve_connection(TokioIo::new(stream), service)
                .with_upgrades()
                .await;
            if let Err(error) = served {
                tracing::debug!(%error, "connection ended with an error");
            }
        });
    }
}
