use super::*;
pub(super) fn normalize_host_address(value: &str) -> AppResult<String> {
    let address = value.trim();
    if address.is_empty() {
        return Err(AppError::Validation("主机地址不能为空".to_string()));
    }
    if address.contains("://") || address.contains('/') {
        return Err(AppError::Validation(
            "主机地址只填写 IP 或主机名，不要包含 http://、端口或路径".to_string(),
        ));
    }
    if address.contains(':') && !address.contains('.') {
        return Err(AppError::Validation(
            "主机地址和端口请分开填写；IPv6 地址当前不作为局域网自动发现验收范围".to_string(),
        ));
    }
    Ok(address.to_string())
}

pub(super) fn validate_host_port(port: u16) -> AppResult<()> {
    if port < 1024 {
        return Err(AppError::Validation(
            "主机端口必须在 1024-65535 之间，建议使用默认 17871".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn begin_pairing(
    runtime: &Arc<Mutex<HostServiceRuntime>>,
    request: PairStartRequest,
    client_ip: String,
) -> AppResult<PairStartResponse> {
    let mut runtime = runtime
        .lock()
        .map_err(|_| AppError::Validation("主机配对状态异常".to_string()))?;
    let fingerprint = runtime.certificate_fingerprint.clone();
    runtime.pairing.begin(request, client_ip, &fingerprint)
}

pub(super) fn finish_pairing(
    runtime: &Arc<Mutex<HostServiceRuntime>>,
    db: &Db,
    request: PairFinishRequest,
) -> AppResult<PairFinishResponse> {
    let verified = {
        let mut runtime = runtime
            .lock()
            .map_err(|_| AppError::Validation("主机配对状态异常".to_string()))?;
        let verified = runtime.pairing.finish(request)?;
        runtime.pair_code = runtime.pairing.code().map(str::to_owned);
        verified
    };
    register_paired_client(runtime, db, verified)
}

pub(super) fn register_paired_client(
    runtime: &Arc<Mutex<HostServiceRuntime>>,
    db: &Db,
    verified: VerifiedPairing,
) -> AppResult<PairFinishResponse> {
    let id = Uuid::new_v4().to_string();
    let token = Uuid::new_v4().to_string();
    let now = chrono::Local::now().to_rfc3339();
    let client = ClientConnectionInfo {
        id: id.clone(),
        client_name: verified.client_name,
        client_device_id: verified.client_device_id,
        client_ip: verified.client_ip,
        app_version: verified.app_version,
        status: "paired".to_string(),
        last_seen_at: now,
    };
    {
        let mut runtime = runtime
            .lock()
            .map_err(|_| AppError::Validation("主机配对状态异常".to_string()))?;
        runtime.clients.insert(
            token.clone(),
            ClientConnectionInfo {
                id: token.clone(),
                ..client.clone()
            },
        );
    }
    db.with_conn(|conn| upsert_client_connection(conn, &client, &token_hash(&token)))?;
    Ok(PairFinishResponse {
        token,
        message: "配对成功".to_string(),
    })
}

pub(super) fn status_from_runtime(runtime: &HostServiceRuntime) -> HostServiceStatus {
    HostServiceStatus {
        running: runtime.running,
        bind_address: runtime.bind_address.clone(),
        port: runtime.port,
        pair_code: runtime.pair_code.clone(),
        client_count: runtime.clients.len(),
        message: if runtime.running {
            format!("主机服务运行中：{}:{}", runtime.bind_address, runtime.port)
        } else {
            "主机服务未启动".to_string()
        },
    }
}

pub(super) fn validate_pairing_request(
    pair_code: &str,
    client_name: &str,
    client_device_id: &str,
) -> AppResult<()> {
    let code = pair_code.trim();
    if code.len() != 12 || !code.chars().all(|item| item.is_ascii_digit()) {
        return Err(AppError::Validation("配对码必须是 12 位数字".to_string()));
    }
    if client_name.trim().is_empty() {
        return Err(AppError::Validation("客户端名称不能为空".to_string()));
    }
    if client_device_id.trim().is_empty() {
        return Err(AppError::Validation("设备 ID 不能为空".to_string()));
    }
    Ok(())
}

pub(super) fn authenticate_request(
    request: &str,
    runtime: &Arc<Mutex<HostServiceRuntime>>,
) -> AppResult<()> {
    let token = header_value(request, "X-Aster-Client-Token")
        .ok_or_else(|| AppError::Unauthorized("缺少客户端连接凭据".to_string()))?;
    let mut runtime = runtime
        .lock()
        .map_err(|_| AppError::Validation("主机运行状态异常".to_string()))?;
    let Some(client) = runtime
        .clients
        .values_mut()
        .find(|client| client.id == token)
    else {
        return Err(AppError::Unauthorized(
            "客户端连接凭据无效，请重新配对".to_string(),
        ));
    };
    client.last_seen_at = chrono::Local::now().to_rfc3339();
    client.status = "online".to_string();
    Ok(())
}

pub(super) fn authenticate_request_and_touch_client(
    request: &str,
    runtime: &Arc<Mutex<HostServiceRuntime>>,
    db: &Db,
) -> AppResult<()> {
    let client_device_id = authenticate_request_and_load_client(request, runtime, db)?;
    db.with_conn(|conn| touch_client_connection(conn, &client_device_id, "online"))?;
    Ok(())
}

pub(super) fn authenticate_request_and_load_client(
    request: &str,
    runtime: &Arc<Mutex<HostServiceRuntime>>,
    db: &Db,
) -> AppResult<String> {
    let token = header_value(request, "X-Aster-Client-Token")
        .ok_or_else(|| AppError::Unauthorized("缺少客户端连接凭据".to_string()))?;
    if authenticate_request(request, runtime).is_err() {
        let Some(persisted_client) =
            db.with_conn(|conn| find_client_connection_by_token_hash(conn, &token_hash(&token)))?
        else {
            return Err(AppError::Unauthorized(
                "客户端连接凭据无效，请重新配对".to_string(),
            ));
        };
        let mut runtime = runtime
            .lock()
            .map_err(|_| AppError::Validation("主机运行状态异常".to_string()))?;
        runtime.clients.insert(
            token.clone(),
            ClientConnectionInfo {
                id: token.clone(),
                status: "online".to_string(),
                last_seen_at: chrono::Local::now().to_rfc3339(),
                ..persisted_client
            },
        );
    }
    Ok({
        let runtime = runtime
            .lock()
            .map_err(|_| AppError::Validation("主机运行状态异常".to_string()))?;
        runtime
            .clients
            .values()
            .find(|client| client.id == token)
            .map(|client| client.client_device_id.clone())
            .ok_or_else(|| AppError::Unauthorized("客户端连接凭据无效，请重新配对".to_string()))?
    })
}

pub(super) fn upsert_client_connection(
    conn: &rusqlite::Connection,
    client: &ClientConnectionInfo,
    token_hash: &str,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO client_connections (
           id, client_name, client_device_id, token_hash, client_ip, app_version, status, last_seen_at, updated_at
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, CURRENT_TIMESTAMP)
         ON CONFLICT(client_device_id) DO UPDATE SET
           id = excluded.id,
           client_name = excluded.client_name,
           token_hash = excluded.token_hash,
           client_ip = excluded.client_ip,
           app_version = excluded.app_version,
           status = excluded.status,
           last_seen_at = excluded.last_seen_at,
           updated_at = CURRENT_TIMESTAMP",
        rusqlite::params![
            client.id,
            client.client_name,
            client.client_device_id,
            token_hash,
            client.client_ip,
            client.app_version,
            client.status,
            client.last_seen_at
        ],
    )?;
    Ok(())
}

pub(super) fn find_client_connection_by_token_hash(
    conn: &rusqlite::Connection,
    token_hash: &str,
) -> AppResult<Option<ClientConnectionInfo>> {
    use rusqlite::OptionalExtension;

    Ok(conn
        .query_row(
            "SELECT id, client_name, client_device_id, COALESCE(client_ip, ''),
                    COALESCE(app_version, ''), status, last_seen_at
             FROM client_connections
             WHERE token_hash = ?1
             LIMIT 1",
            rusqlite::params![token_hash],
            |row| {
                Ok(ClientConnectionInfo {
                    id: row.get(0)?,
                    client_name: row.get(1)?,
                    client_device_id: row.get(2)?,
                    client_ip: row.get(3)?,
                    app_version: row.get(4)?,
                    status: row.get(5)?,
                    last_seen_at: row.get(6)?,
                })
            },
        )
        .optional()?)
}

pub(super) fn touch_client_connection(
    conn: &rusqlite::Connection,
    client_device_id: &str,
    status: &str,
) -> AppResult<()> {
    conn.execute(
        "UPDATE client_connections
         SET status = ?2, last_seen_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP
         WHERE client_device_id = ?1",
        rusqlite::params![client_device_id, status],
    )?;
    Ok(())
}

pub(super) fn remove_client_connection_from_conn(
    conn: &rusqlite::Connection,
    client_device_id: &str,
) -> AppResult<ClientConnectionInfo> {
    let client = conn
        .query_row(
            "SELECT id, client_name, client_device_id, COALESCE(client_ip, ''),
                    COALESCE(app_version, ''), status, last_seen_at
             FROM client_connections
             WHERE client_device_id = ?1",
            rusqlite::params![client_device_id],
            |row| {
                Ok(ClientConnectionInfo {
                    id: row.get(0)?,
                    client_name: row.get(1)?,
                    client_device_id: row.get(2)?,
                    client_ip: row.get(3)?,
                    app_version: row.get(4)?,
                    status: row.get(5)?,
                    last_seen_at: row.get(6)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| AppError::Validation("客户端设备不存在".to_string()))?;
    conn.execute(
        "DELETE FROM client_connections WHERE client_device_id = ?1",
        rusqlite::params![client_device_id],
    )?;
    Ok(client)
}

pub(super) fn token_hash(token: &str) -> String {
    crate::services::remote_session_service::token_hash(token)
}
