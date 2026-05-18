use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, bail};
use log::info;
use tokio::{
    fs,
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};
use tokio_rustls::{
    TlsAcceptor,
    rustls::{
        ServerConfig as TlsServerConfig,
        pki_types::{CertificateDer, pem::PemObject},
    },
};

use crate::{config::HttpBootConfig, tftp::files::normalize_relative_path};

const MAX_REQUEST_LINE: usize = 8192;
const MAX_HEADER_LINE: usize = 8192;

pub async fn spawn_https_static_server(
    config: &HttpBootConfig,
) -> anyhow::Result<Option<tokio::task::JoinHandle<()>>> {
    if !config.enabled || !config.https.enabled {
        return Ok(None);
    }

    let tls_config = Arc::new(load_tls_config(config).await?);
    let root_dir = Arc::new(config.root_dir.clone());
    let listener = TcpListener::bind(config.https.listen_addr)
        .await
        .with_context(|| {
            format!(
                "failed to bind HTTP Boot HTTPS listener {}",
                config.https.listen_addr
            )
        })?;
    let local_addr = listener.local_addr()?;
    info!(
        "HTTP Boot HTTPS static server listening on https://{} root={}",
        local_addr,
        root_dir.display()
    );

    let acceptor = TlsAcceptor::from(tls_config);
    Ok(Some(tokio::spawn(async move {
        loop {
            let (stream, peer) = match listener.accept().await {
                Ok(value) => value,
                Err(err) => {
                    log::warn!("failed to accept HTTP Boot HTTPS connection: {err:#}");
                    continue;
                }
            };
            let acceptor = acceptor.clone();
            let root_dir = root_dir.clone();
            tokio::spawn(async move {
                if let Err(err) = serve_tls_connection(acceptor, stream, root_dir).await {
                    log::debug!("HTTP Boot HTTPS connection from {peer} failed: {err:#}");
                }
            });
        }
    })))
}

async fn load_tls_config(config: &HttpBootConfig) -> anyhow::Result<TlsServerConfig> {
    let cert_bytes = fs::read(&config.https.cert_path).await.with_context(|| {
        format!(
            "failed to read HTTP Boot HTTPS certificate {}",
            config.https.cert_path.display()
        )
    })?;
    let key_bytes = fs::read(&config.https.key_path).await.with_context(|| {
        format!(
            "failed to read HTTP Boot HTTPS private key {}",
            config.https.key_path.display()
        )
    })?;
    let certs = CertificateDer::pem_slice_iter(&cert_bytes)
        .collect::<Result<Vec<_>, _>>()
        .context("failed to parse HTTP Boot HTTPS certificate PEM")?;
    if certs.is_empty() {
        bail!(
            "HTTP Boot HTTPS certificate {} contains no certificates",
            config.https.cert_path.display()
        );
    }
    let key = tokio_rustls::rustls::pki_types::PrivateKeyDer::from_pem_slice(&key_bytes)
        .context("failed to parse HTTP Boot HTTPS private key PEM")?;

    TlsServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .context("failed to build HTTP Boot HTTPS TLS config")
}

async fn serve_tls_connection(
    acceptor: TlsAcceptor,
    stream: TcpStream,
    root_dir: Arc<PathBuf>,
) -> anyhow::Result<()> {
    let stream = acceptor.accept(stream).await?;
    let mut reader = BufReader::new(stream);
    let mut request_line = String::new();
    read_limited_line(&mut reader, &mut request_line, MAX_REQUEST_LINE).await?;
    let request = parse_request_line(&request_line)?;

    loop {
        let mut header = String::new();
        read_limited_line(&mut reader, &mut header, MAX_HEADER_LINE).await?;
        if header == "\r\n" || header == "\n" || header.is_empty() {
            break;
        }
    }

    let response = match request {
        HttpBootRequest::Get(path) => file_response(&root_dir, path, true).await,
        HttpBootRequest::Head(path) => file_response(&root_dir, path, false).await,
        HttpBootRequest::MethodNotAllowed => http_response(
            "405 Method Not Allowed",
            "text/plain",
            b"method not allowed".to_vec(),
            true,
        ),
        HttpBootRequest::BadRequest => http_response(
            "400 Bad Request",
            "text/plain",
            b"bad request".to_vec(),
            true,
        ),
    };

    let stream = reader.get_mut();
    stream.write_all(&response).await?;
    stream.flush().await?;
    Ok(())
}

async fn read_limited_line<R>(
    reader: &mut BufReader<R>,
    line: &mut String,
    limit: usize,
) -> anyhow::Result<()>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let len = reader.read_line(line).await?;
    if len == 0 {
        bail!("unexpected EOF while reading HTTP request");
    }
    if line.len() > limit {
        bail!("HTTP request line exceeds {limit} bytes");
    }
    Ok(())
}

enum HttpBootRequest<'a> {
    Get(&'a str),
    Head(&'a str),
    MethodNotAllowed,
    BadRequest,
}

fn parse_request_line(line: &str) -> anyhow::Result<HttpBootRequest<'_>> {
    let mut parts = line.trim_end_matches(['\r', '\n']).split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("empty request"))?;
    let target = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing target"))?;
    let version = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing version"))?;
    if !version.starts_with("HTTP/1.") {
        return Ok(HttpBootRequest::BadRequest);
    }
    if parts.next().is_some() {
        return Ok(HttpBootRequest::BadRequest);
    }

    let path = target.split('?').next().unwrap_or(target);
    match method {
        "GET" => Ok(HttpBootRequest::Get(path)),
        "HEAD" => Ok(HttpBootRequest::Head(path)),
        _ => Ok(HttpBootRequest::MethodNotAllowed),
    }
}

async fn file_response(root_dir: &Path, request_path: &str, include_body: bool) -> Vec<u8> {
    let Some(relative_path) = request_path.strip_prefix("/boot/") else {
        return http_response(
            "404 Not Found",
            "text/plain",
            b"not found".to_vec(),
            include_body,
        );
    };
    let relative_path = match normalize_relative_path(relative_path) {
        Ok(path) => path,
        Err(_) => {
            return http_response(
                "400 Bad Request",
                "text/plain",
                b"bad request".to_vec(),
                include_body,
            );
        }
    };
    let disk_path = root_dir.join(relative_path);
    let body = match fs::read(&disk_path).await {
        Ok(body) => body,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return http_response(
                "404 Not Found",
                "text/plain",
                b"not found".to_vec(),
                include_body,
            );
        }
        Err(_) => {
            return http_response(
                "500 Internal Server Error",
                "text/plain",
                b"internal server error".to_vec(),
                include_body,
            );
        }
    };
    let content_type = mime_guess::from_path(&disk_path)
        .first_or_octet_stream()
        .to_string();
    http_response("200 OK", &content_type, body, include_body)
}

fn http_response(status: &str, content_type: &str, body: Vec<u8>, include_body: bool) -> Vec<u8> {
    let headers = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let mut response = headers.into_bytes();
    if include_body {
        response.extend_from_slice(&body);
    }
    response
}
