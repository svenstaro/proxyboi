use actix_web::client::{ClientRequest, ClientResponse};
use actix_web::{HttpRequest, HttpResponse};
use chrono::prelude::*;
use inflector::Inflector;
use yansi::Paint;

pub fn log_incoming_request(req: &HttpRequest, verbose: bool) -> String {
    let conn_info = req.connection_info();
    let local_time = Local::now();
    let time = local_time.format("[%d/%b/%Y:%H:%M:%S %z]").to_string();

    if verbose {
        let path_query = if req.query_string().is_empty() {
            req.path().to_string()
        } else {
            format!(
                "{path}?{query}",
                path = req.path(),
                query = req.query_string(),
            )
        };
        let method_path_version_line = format!(
            "{method} {path_query} {http}/{version}",
            method = Paint::green(req.method()),
            path_query = Paint::cyan(path_query).underline(),
            http = Paint::blue("HTTP"),
            version = Paint::blue(
                format!("{:?}", req.version())
                    .split('/')
                    .nth(1)
                    .unwrap_or("unknown")
            ),
        );

        let mut headers_vec = vec![];
        for (hk, hv) in req.headers() {
            headers_vec.push(format!(
                "{deco} {key}: {value}",
                deco = Paint::green("│").bold(),
                key = Paint::cyan(Inflector::to_train_case(hk.as_str())),
                value = hv.to_str().unwrap_or("<unprintable>")
            ));
        }
        headers_vec.sort();
        let headers = headers_vec.join("\n");
        let req_info = format!(
            "{deco} {method_path_line}\n{headers}",
            deco = Paint::green("│").bold(),
            method_path_line = method_path_version_line,
            headers = headers
        );
        format!(
            "Connection from {remote} at {time}\n{req_banner} from {remote_pretty}\n{req_info}",
            remote = conn_info.realip_remote_addr().unwrap_or("unknown"),
            remote_pretty =
                Paint::magenta(conn_info.realip_remote_addr().unwrap_or("unknown")).bold(),
            time = time,
            req_banner = Paint::green("┌─Incoming request").bold(),
            req_info = req_info,
        )
    } else {
        format!(
            "Connection from {remote} at {time}",
            remote = conn_info.realip_remote_addr().unwrap_or("unknown"),
            time = time
        )
    }
}

pub fn log_upstream_request(req: &ClientRequest, verbose: bool) -> String {
    if verbose {
        let method_path_version_line = format!(
            "{method} {path_query} {http}/{version}",
            method = Paint::green(req.get_method()),
            path_query = Paint::cyan(req.get_uri()).underline(),
            http = Paint::blue("HTTP"),
            version = Paint::blue(
                format!("{:?}", req.get_version())
                    .split('/')
                    .nth(1)
                    .unwrap_or("unknown")
            ),
        );

        let mut headers_vec = vec![];
        for (hk, hv) in req.headers() {
            headers_vec.push(format!(
                "{deco} {key}: {value}",
                deco = Paint::cyan("│").bold(),
                key = Paint::cyan(Inflector::to_train_case(hk.as_str())),
                value = hv.to_str().unwrap_or("<unprintable>")
            ));
        }
        headers_vec.sort();
        let headers = headers_vec.join("\n");
        let req_info = format!(
            "{deco} {method_path_line}\n{headers}",
            deco = Paint::cyan("│").bold(),
            method_path_line = method_path_version_line,
            headers = headers
        );
        format!(
            "{req_banner} to {uri}\n{req_info}",
            req_banner = Paint::cyan("┌─Upstream request").bold(),
            uri = Paint::yellow(req.get_uri()),
            req_info = req_info,
        )
    } else {
        String::new()
    }
}

pub fn log_upstream_response<T>(
    resp: &ClientResponse<T>,
    upstream_uri: &str,
    verbose: bool,
) -> String {
    if verbose {
        let status_line = format!(
            "{http}/{version} {status_code} {status_text}",
            http = Paint::blue("HTTP"),
            version = Paint::blue(
                format!("{:?}", resp.version())
                    .split('/')
                    .nth(1)
                    .unwrap_or("unknown")
            ),
            status_code = Paint::blue(resp.status().as_u16()),
            status_text = Paint::cyan(resp.status().canonical_reason().unwrap_or("")),
        );

        let mut headers_vec = vec![];
        for (hk, hv) in resp.headers() {
            headers_vec.push(format!(
                "{deco} {key}: {value}",
                deco = Paint::blue("│").bold(),
                key = Paint::cyan(Inflector::to_train_case(hk.as_str())),
                value = hv.to_str().unwrap_or("<unprintable>")
            ));
        }
        headers_vec.sort();
        let headers = headers_vec.join("\n");
        let req_info = format!(
            "{deco} {status_line}\n{headers}",
            deco = Paint::blue("│").bold(),
            status_line = status_line,
            headers = headers
        );
        format!(
            "{req_banner} from {uri}\n{req_info}",
            req_banner = Paint::blue("┌─Upstream response").bold(),
            uri = Paint::yellow(upstream_uri),
            req_info = req_info,
        )
    } else {
        String::new()
    }
}

pub fn log_outgoing_response(resp: &HttpResponse, remote: &str, verbose: bool) -> String {
    if verbose {
        let status_line = format!(
            "{http}/{version} {status_code} {status_text}",
            http = Paint::blue("HTTP"),
            version = Paint::blue(
                format!("{:?}", resp.head().version)
                    .split('/')
                    .nth(1)
                    .unwrap_or("unknown")
            ),
            status_code = Paint::blue(resp.status().as_u16()),
            status_text = Paint::cyan(resp.status().canonical_reason().unwrap_or("")),
        );

        let mut headers_vec = vec![];
        for (hk, hv) in resp.headers() {
            headers_vec.push(format!(
                "{deco} {key}: {value}",
                deco = Paint::red("│").bold(),
                key = Paint::cyan(Inflector::to_train_case(hk.as_str())),
                value = hv.to_str().unwrap_or("<unprintable>")
            ));
        }
        headers_vec.sort();
        let headers = headers_vec.join("\n");
        let req_info = format!(
            "{deco} {status_line}\n{headers}",
            deco = Paint::red("│").bold(),
            status_line = status_line,
            headers = headers
        );
        format!(
            "{req_banner} to {remote}\n{req_info}",
            req_banner = Paint::red("┌─Outgoing response").bold(),
            remote = Paint::magenta(remote).bold(),
            req_info = req_info,
        )
    } else {
        String::new()
    }
}
