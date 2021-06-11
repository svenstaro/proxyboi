use actix_web::{client::Client, web, HttpRequest, HttpResponse};
use log::info;

use crate::{
    config::ProxyboiConfig,
    error::ProxyboiError,
    forwarded_header::ForwardedHeader,
    logging::{
        log_incoming_request, log_outgoing_response, log_upstream_request, log_upstream_response,
    },
};

pub async fn forward(
    incoming_request: HttpRequest,
    body: web::Bytes,
    config: web::Data<ProxyboiConfig>,
    client: web::Data<Client>,
) -> Result<HttpResponse, ProxyboiError> {
    let incoming_request_log = log_incoming_request(&incoming_request, config.verbose);

    // Figure out new URL like such:
    // Old URL: http://localhost:8080/foo?bar=1
    // New URL: https://0.0.0.0:8081/foo?bar=1
    // So in effect, we have to change `protocol`, `host`, `port` and keep `path` and `query`.
    let mut new_url = config.upstream.clone();
    new_url.set_path(incoming_request.uri().path());
    new_url.set_query(incoming_request.uri().query());

    let conn_info = &incoming_request.connection_info().clone();
    let protocol = conn_info.scheme();
    let version = incoming_request.version();
    let host = conn_info.host();

    let peer = incoming_request
        .head()
        .peer_addr
        .map(|p| p.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let forwarded = incoming_request
        .headers()
        .get("forwarded")
        .map(|x| x.to_str().unwrap_or(""))
        .unwrap_or("");

    let forwarded_header = ForwardedHeader::from_info(
        &peer,
        &config.listen.ip().to_string(),
        forwarded,
        host,
        protocol,
    );
    let via = if let Some(via) = incoming_request
        .headers()
        .get("via")
        .map(|x| x.to_str().unwrap_or(""))
    {
        format!(
            "{previous_via}, {version:?} proxyboi",
            previous_via = via,
            version = version
        )
    } else {
        format!("{version:?} proxyboi", version = version)
    };

    // The X-Forwarded-For header is much simpler to handle :)
    let x_forwarded_for = incoming_request
        .headers()
        .get("x-forwarded-for")
        .map(|x| x.to_str().unwrap_or(""));
    let x_forwarded_for_appended = if let Some(x_forwarded_for) = x_forwarded_for {
        format!("{}, {}", x_forwarded_for, peer)
    } else {
        peer
    };

    let mut upstream_req = client
        .request_from(new_url.as_str(), incoming_request.head())
        .no_decompress()
        // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Forwarded
        .set_header("forwarded", forwarded_header.to_string())
        // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-Forwarded-Proto
        .set_header("x-forwarded-proto", protocol)
        // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-Forwarded-Host
        .set_header("x-forwarded-host", host)
        // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-Forwarded-For
        .set_header("x-forwarded-for", x_forwarded_for_appended)
        // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Via
        .set_header("via", via);

    // Insert additional headers for upstream server request.
    for additional_header in &config.upstream_headers {
        let (header_name, header_value) = additional_header
            .iter()
            .next()
            .expect("Expected to find a header here but there was none");
        upstream_req = upstream_req.set_header(header_name, header_value.clone());
    }

    let upstream_request_log = log_upstream_request(&upstream_req, config.verbose);

    let mut upstream_resp = upstream_req.send_body(body).await?;

    let upstream_response_log =
        log_upstream_response(&upstream_resp, new_url.as_str(), config.verbose);

    let mut outgoing_resp_builder = HttpResponse::build(upstream_resp.status());
    for (header_name, header_value) in upstream_resp
        .headers()
        .iter()
        // Remove `Connection` as per
        // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Connection#Directives
        .filter(|(h, _)| *h != "connection" && *h != "transfer-encoding")
    {
        outgoing_resp_builder.header(header_name, header_value.clone());
    }

    // Insert additional headers for outgoing response.
    for additional_header in &config.response_headers {
        let (header_name, header_value) = additional_header
            .iter()
            .next()
            .expect("Expected to find a header here but there was none");
        outgoing_resp_builder.header(header_name, header_value.clone());
    }

    let outgoing_resp = outgoing_resp_builder.body(upstream_resp.body().await?);

    let outgoing_response_log = log_outgoing_response(
        &outgoing_resp,
        incoming_request
            .connection_info()
            .realip_remote_addr()
            .unwrap_or("unknown"),
        config.verbose,
    );
    info!(
        "{incoming_req}\n{upstream_req}\n{upstream_resp}\n{outgoing_resp}",
        incoming_req = incoming_request_log,
        upstream_req = upstream_request_log,
        upstream_resp = upstream_response_log,
        outgoing_resp = outgoing_response_log
    );
    Ok(outgoing_resp)
}
