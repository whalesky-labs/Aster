use std::io::{Read, Write};

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::{AppError, AppResult};

const MAX_HEADER_BYTES: usize = 16 * 1024;
const MAX_BODY_BYTES: usize = 1024 * 1024;
const MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;
const MAX_BINARY_RESPONSE_BYTES: usize = 256 * 1024 * 1024;
const XLSX_CONTENT_TYPE: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";

#[derive(Debug)]
pub struct BinaryResponse {
    pub body: Vec<u8>,
    pub row_count: usize,
}

pub fn read_request(stream: &mut impl Read) -> AppResult<String> {
    let mut buffer = Vec::new();
    let header_end = read_headers(stream, &mut buffer)?;
    let header_text = String::from_utf8_lossy(&buffer[..header_end]);
    let content_length = header_value(&header_text, "Content-Length")
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|_| AppError::Validation("Content-Length 格式无效".to_string()))?
        .unwrap_or(0);
    if content_length > MAX_BODY_BYTES {
        return Err(AppError::PayloadTooLarge(
            "HTTP 请求正文超过 1 MiB 限制".to_string(),
        ));
    }
    let body_start = header_end + 4;
    let target_length = body_start + content_length;
    read_exact_length(stream, &mut buffer, target_length)?;
    buffer.truncate(target_length);
    String::from_utf8(buffer)
        .map_err(|_| AppError::Validation("HTTP 请求不是有效 UTF-8".to_string()))
}

pub fn read_json_response<T: DeserializeOwned>(stream: impl Read) -> AppResult<T> {
    let mut response = Vec::new();
    stream
        .take((MAX_HEADER_BYTES + MAX_RESPONSE_BYTES + 1) as u64)
        .read_to_end(&mut response)?;
    if response.len() > MAX_HEADER_BYTES + MAX_RESPONSE_BYTES {
        return Err(AppError::PayloadTooLarge(
            "主机响应超过 8 MiB 限制".to_string(),
        ));
    }
    let response = String::from_utf8(response)
        .map_err(|_| AppError::Validation("主机响应不是有效 UTF-8".to_string()))?;
    let (head, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| AppError::Validation("主机响应格式异常".to_string()))?;
    if !head.starts_with("HTTP/1.1 200") {
        return Err(AppError::Validation(format!(
            "主机返回错误：{}",
            response_error_message(body)
        )));
    }
    serde_json::from_str(body)
        .map_err(|error| AppError::Validation(format!("主机响应解析失败：{error}")))
}

pub fn read_xlsx_response(stream: impl Read) -> AppResult<BinaryResponse> {
    let mut response = Vec::new();
    stream
        .take((MAX_HEADER_BYTES + MAX_BINARY_RESPONSE_BYTES + 1) as u64)
        .read_to_end(&mut response)?;
    if response.len() > MAX_HEADER_BYTES + MAX_BINARY_RESPONSE_BYTES {
        return Err(AppError::PayloadTooLarge(
            "库存导出响应超过 256 MiB 限制".to_string(),
        ));
    }
    let header_end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| AppError::Validation("主机响应格式异常".to_string()))?;
    if header_end > MAX_HEADER_BYTES {
        return Err(AppError::RequestHeaderTooLarge(
            "主机响应头超过 16 KiB 限制".to_string(),
        ));
    }
    let head = std::str::from_utf8(&response[..header_end])
        .map_err(|_| AppError::Validation("主机响应头不是有效 UTF-8".to_string()))?
        .to_string();
    let body = response.split_off(header_end + 4);
    if !head.starts_with("HTTP/1.1 200") {
        return Err(AppError::Validation(format!(
            "主机返回错误：{}",
            response_error_message(&String::from_utf8_lossy(&body))
        )));
    }
    let content_type = header_value(&head, "Content-Type").unwrap_or_default();
    if !content_type.eq_ignore_ascii_case(XLSX_CONTENT_TYPE) {
        return Err(AppError::Validation("主机库存导出响应类型异常".to_string()));
    }
    let content_length = header_value(&head, "Content-Length")
        .ok_or_else(|| AppError::Validation("主机库存导出缺少文件长度".to_string()))?
        .parse::<usize>()
        .map_err(|_| AppError::Validation("主机库存导出文件长度无效".to_string()))?;
    if content_length != body.len() {
        return Err(AppError::Validation("主机库存导出文件不完整".to_string()));
    }
    let row_count = header_value(&head, "X-Aster-Row-Count")
        .ok_or_else(|| AppError::Validation("主机库存导出缺少行数".to_string()))?
        .parse::<usize>()
        .map_err(|_| AppError::Validation("主机库存导出行数无效".to_string()))?;
    Ok(BinaryResponse { body, row_count })
}

pub fn write_json<T: Serialize>(stream: &mut impl Write, status: u16, body: &T) -> AppResult<()> {
    let text = serde_json::to_string(body)
        .map_err(|error| AppError::Validation(format!("JSON 序列化失败：{error}")))?;
    if text.len() > MAX_RESPONSE_BYTES {
        return Err(AppError::PayloadTooLarge(
            "HTTP 响应超过 8 MiB 限制，请缩小查询范围".to_string(),
        ));
    }
    let status_text = status_text(status);
    let retry_after = if status == 429 {
        "Retry-After: 900\r\n"
    } else {
        ""
    };
    let response = format!(
        "HTTP/1.1 {status} {status_text}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\n{retry_after}Connection: close\r\n\r\n{}",
        text.len(),
        text
    );
    stream.write_all(response.as_bytes())?;
    Ok(())
}

pub fn write_xlsx(stream: &mut impl Write, body: &[u8], row_count: usize) -> AppResult<()> {
    if body.len() > MAX_BINARY_RESPONSE_BYTES {
        return Err(AppError::PayloadTooLarge(
            "库存导出文件超过 256 MiB 限制".to_string(),
        ));
    }
    let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {XLSX_CONTENT_TYPE}\r\nContent-Length: {}\r\nX-Aster-Row-Count: {row_count}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)?;
    Ok(())
}

pub fn error_status(error: &AppError) -> u16 {
    match error {
        AppError::Unauthorized(_) => 401,
        AppError::Forbidden(_) => 403,
        AppError::PayloadTooLarge(_) => 413,
        AppError::RequestHeaderTooLarge(_) => 431,
        AppError::RateLimited(_) => 429,
        AppError::Timeout(_) => 504,
        AppError::Validation(_) | AppError::InvalidRuntimeMode(_) => 400,
        AppError::Io(error)
            if error.kind() == std::io::ErrorKind::TimedOut
                || error.kind() == std::io::ErrorKind::WouldBlock =>
        {
            408
        }
        AppError::Io(_) | AppError::Database(_) | AppError::ProjectDirectoryUnavailable => 500,
    }
}

pub fn public_error_message(error: &AppError) -> String {
    match error {
        AppError::Io(_) | AppError::Database(_) | AppError::ProjectDirectoryUnavailable => {
            "主机内部错误，请查看主机日志".to_string()
        }
        _ => error.to_string(),
    }
}

fn status_text(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        408 => "Request Timeout",
        413 => "Payload Too Large",
        429 => "Too Many Requests",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "Error",
    }
}

pub fn request_line(request: &str) -> (String, String) {
    let mut parts = request
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace();
    (
        parts.next().unwrap_or_default().to_string(),
        parts.next().unwrap_or_default().to_string(),
    )
}

pub fn route_matches(path: &str, route: &str) -> bool {
    path == route
        || path
            .strip_prefix(route)
            .is_some_and(|suffix| suffix.starts_with('?'))
}

pub fn header_value(request: &str, key: &str) -> Option<String> {
    request.lines().skip(1).find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case(key)
            .then(|| value.trim().to_string())
    })
}

pub fn query_param(path: &str, key: &str) -> Option<String> {
    let query = path.split_once('?')?.1;
    query.split('&').find_map(|pair| {
        let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
        (name == key).then(|| url_decode(value))
    })
}

pub fn push_query_param(params: &mut Vec<String>, key: &str, value: Option<String>) {
    let Some(value) = value else {
        return;
    };
    let value = value.trim();
    if !value.is_empty() {
        params.push(format!("{key}={}", url_encode(value)));
    }
}

pub fn page_path(path: &str, cursor: Option<&str>) -> String {
    let Some(cursor) = cursor else {
        return path.to_string();
    };
    let separator = if path.contains('?') { '&' } else { '?' };
    format!("{path}{separator}cursor={}", url_encode(cursor))
}

pub fn url_encode(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            b' ' => vec!['+'],
            other => format!("%{other:02X}").chars().collect(),
        })
        .collect()
}

fn url_decode(value: &str) -> String {
    let mut bytes = Vec::new();
    let mut chars = value.as_bytes().iter().copied().peekable();
    while let Some(byte) = chars.next() {
        match byte {
            b'+' => bytes.push(b' '),
            b'%' => decode_percent_byte(&mut chars, &mut bytes),
            other => bytes.push(other),
        }
    }
    String::from_utf8_lossy(&bytes).to_string()
}

fn decode_percent_byte(
    chars: &mut std::iter::Peekable<impl Iterator<Item = u8>>,
    bytes: &mut Vec<u8>,
) {
    let Some((high, low)) = chars.next().zip(chars.next()) else {
        bytes.push(b'%');
        return;
    };
    let encoded = [high, low];
    match std::str::from_utf8(&encoded)
        .ok()
        .and_then(|hex| u8::from_str_radix(hex, 16).ok())
    {
        Some(decoded) => bytes.push(decoded),
        None => bytes.extend_from_slice(&[b'%', high, low]),
    }
}

fn read_headers(stream: &mut impl Read, buffer: &mut Vec<u8>) -> AppResult<usize> {
    let mut temporary = [0_u8; 4096];
    loop {
        if let Some(index) = find_header_end(buffer) {
            return Ok(index);
        }
        if buffer.len() >= MAX_HEADER_BYTES {
            return Err(AppError::RequestHeaderTooLarge(
                "HTTP 请求头超过 16 KiB 限制".to_string(),
            ));
        }
        let bytes = stream.read(&mut temporary)?;
        if bytes == 0 {
            return Err(AppError::Validation("HTTP 请求头不完整".to_string()));
        }
        buffer.extend_from_slice(&temporary[..bytes]);
        if find_header_end(buffer).is_none() && buffer.len() > MAX_HEADER_BYTES {
            return Err(AppError::RequestHeaderTooLarge(
                "HTTP 请求头超过 16 KiB 限制".to_string(),
            ));
        }
    }
}

fn read_exact_length(
    stream: &mut impl Read,
    buffer: &mut Vec<u8>,
    target_length: usize,
) -> AppResult<()> {
    let mut temporary = [0_u8; 4096];
    while buffer.len() < target_length {
        let remaining = target_length - buffer.len();
        let read_length = remaining.min(temporary.len());
        let bytes = stream.read(&mut temporary[..read_length])?;
        if bytes == 0 {
            return Err(AppError::Validation("HTTP 请求正文不完整".to_string()));
        }
        buffer.extend_from_slice(&temporary[..bytes]);
    }
    Ok(())
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn response_error_message(body: &str) -> String {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| value.get("message")?.as_str().map(str::to_string))
        .unwrap_or_else(|| body.to_string())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn rejects_oversized_header() {
        let request = format!(
            "GET / HTTP/1.1\r\nX-Large: {}",
            "a".repeat(MAX_HEADER_BYTES)
        );
        assert!(read_request(&mut Cursor::new(request)).is_err());
    }

    #[test]
    fn rejects_oversized_body_before_reading_it() {
        let request = format!(
            "POST / HTTP/1.1\r\nContent-Length: {}\r\n\r\n",
            MAX_BODY_BYTES + 1
        );
        assert!(read_request(&mut Cursor::new(request)).is_err());
    }

    #[test]
    fn rejects_truncated_body() {
        let request = b"POST / HTTP/1.1\r\nContent-Length: 5\r\n\r\nabc";
        assert!(read_request(&mut Cursor::new(request)).is_err());
    }

    #[test]
    fn maps_security_errors_to_stable_http_statuses() {
        assert_eq!(error_status(&AppError::Unauthorized("x".into())), 401);
        assert_eq!(error_status(&AppError::Forbidden("x".into())), 403);
        assert_eq!(error_status(&AppError::PayloadTooLarge("x".into())), 413);
        assert_eq!(error_status(&AppError::RateLimited("x".into())), 429);
    }

    #[test]
    fn rate_limit_response_includes_retry_after() {
        let mut response = Vec::new();
        write_json(
            &mut response,
            429,
            &serde_json::json!({ "message": "wait" }),
        )
        .expect("response");
        let response = String::from_utf8(response).expect("utf8");
        assert!(response.contains("Retry-After: 900\r\n"));
    }

    #[test]
    fn route_matching_rejects_prefix_collisions() {
        assert!(route_matches("/api/users", "/api/users"));
        assert!(route_matches("/api/users?cursor=next", "/api/users"));
        assert!(!route_matches("/api/users-invalid", "/api/users"));
    }

    #[test]
    fn xlsx_response_round_trip_preserves_binary_body_and_row_count() {
        let body = b"PK\x03\x04binary-xlsx";
        let mut response = Vec::new();
        write_xlsx(&mut response, body, 27).unwrap();

        let parsed = read_xlsx_response(Cursor::new(response)).unwrap();

        assert_eq!(parsed.body, body);
        assert_eq!(parsed.row_count, 27);
    }

    #[test]
    fn xlsx_reader_surfaces_json_error_responses() {
        let mut response = Vec::new();
        write_json(
            &mut response,
            403,
            &serde_json::json!({ "message": "需要管理员权限" }),
        )
        .unwrap();

        let error = read_xlsx_response(Cursor::new(response)).unwrap_err();

        assert!(error.to_string().contains("需要管理员权限"));
    }
}
