use super::*;
pub(super) struct ClientRuntimeConfig {
    pub(super) address: String,
    pub(super) port: u16,
    pub(super) token: String,
    pub(super) session_token: Option<String>,
    pub(super) certificate_fingerprint: String,
}

pub(super) fn client_runtime_config(state: &AppState) -> AppResult<ClientRuntimeConfig> {
    let config = crate::services::status_service::get_runtime_config(state)?;
    if config.mode != RuntimeMode::Client {
        return Err(AppError::Validation("当前不是客户端模式".to_string()));
    }
    let address = config
        .host_address
        .ok_or_else(|| AppError::Validation("未配置主机地址".to_string()))?;
    let token = config
        .client_token
        .filter(|token| !token.trim().is_empty())
        .ok_or_else(|| AppError::Validation("未完成主机配对，请先在设置中配对".to_string()))?;
    let session_token = state
        .host_service
        .lock()
        .map_err(|_| AppError::Validation("客户端会话状态异常".to_string()))?
        .client_session_token
        .clone();
    let certificate_fingerprint = state
        .db
        .with_conn(|conn| repository::get_setting(conn, "host_certificate_fingerprint"))?
        .filter(|fingerprint| !fingerprint.trim().is_empty())
        .ok_or_else(|| AppError::Validation("主机证书未固定，请重新配对".to_string()))?;
    Ok(ClientRuntimeConfig {
        address,
        port: config.host_port,
        token,
        session_token,
        certificate_fingerprint,
    })
}

pub(super) fn header_value(request: &str, key: &str) -> Option<String> {
    http_transport::header_value(request, key)
}

pub(super) fn http_get_json<T: for<'de> Deserialize<'de>>(
    config: &ClientRuntimeConfig,
    path: &str,
) -> AppResult<T> {
    let mut stream = secure_transport::connect(
        &config.address,
        config.port,
        Some(&config.certificate_fingerprint),
    )?
    .stream;
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: aster\r\nX-Aster-Client-Token: {}\r\n{}Connection: close\r\n\r\n",
        config.token,
        session_header(config)
    );
    stream.write_all(request.as_bytes())?;
    http_transport::read_json_response(stream)
}

pub(super) fn collect_remote_pages<T: for<'de> Deserialize<'de>>(
    config: &ClientRuntimeConfig,
    path: &str,
) -> AppResult<Vec<T>> {
    crate::application::remote_pagination::collect_all(|cursor| {
        http_get_json(config, &page_path(path, cursor))
    })
}

pub(super) fn http_post_json<T: Serialize, R: for<'de> Deserialize<'de>>(
    config: &ClientRuntimeConfig,
    path: &str,
    body: &T,
) -> AppResult<R> {
    let body = serde_json::to_string(body)
        .map_err(|error| AppError::Validation(format!("JSON 序列化失败：{error}")))?;
    let mut stream = secure_transport::connect(
        &config.address,
        config.port,
        Some(&config.certificate_fingerprint),
    )?
    .stream;
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: aster\r\nX-Aster-Client-Token: {}\r\n{}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        config.token,
        session_header(config),
        body.len(),
        body
    );
    stream.write_all(request.as_bytes())?;
    http_transport::read_json_response(stream)
}

pub(super) fn session_header(config: &ClientRuntimeConfig) -> String {
    config
        .session_token
        .as_deref()
        .map(|token| format!("X-Aster-Session-Token: {token}\r\n"))
        .unwrap_or_default()
}

pub(super) fn http_post_json_for_pairing<T: Serialize, R: for<'de> Deserialize<'de>>(
    address: &str,
    port: u16,
    path: &str,
    body: &T,
    expected_fingerprint: Option<&str>,
) -> AppResult<(R, String)> {
    let body = serde_json::to_string(body)
        .map_err(|error| AppError::Validation(format!("JSON 序列化失败：{error}")))?;
    let connected = secure_transport::connect(address, port, expected_fingerprint)?;
    let fingerprint = connected.fingerprint;
    let mut stream = connected.stream;
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: aster\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(request.as_bytes())?;
    Ok((http_transport::read_json_response(stream)?, fingerprint))
}
