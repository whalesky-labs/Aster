use super::*;

pub(super) fn health_response(db: &Db, app_version: &str) -> AppResult<HealthResponse> {
    db.with_conn(|conn| {
        let integrity = repository::integrity_check(conn)?;
        let schema_version = repository::schema_version(conn)?;
        Ok(HealthResponse {
            app_name: "Aster".to_string(),
            app_version: app_version.to_string(),
            schema_version,
            database_ok: integrity == "ok",
            message: if integrity == "ok" {
                "主机数据库健康".to_string()
            } else {
                format!("主机数据库异常：{integrity}")
            },
        })
    })
}

pub(super) fn version_response(db: &Db, app_version: &str) -> AppResult<VersionResponse> {
    db.with_conn(|conn| {
        Ok(VersionResponse {
            app_name: "Aster".to_string(),
            app_version: app_version.to_string(),
            schema_version: repository::schema_version(conn)?,
        })
    })
}

pub(super) fn write_json<T: Serialize>(
    stream: &mut impl Write,
    status: u16,
    body: &T,
) -> AppResult<()> {
    http_transport::write_json(stream, status, body)
}

pub(super) fn write_xlsx(stream: &mut impl Write, body: &[u8], row_count: usize) -> AppResult<()> {
    http_transport::write_xlsx(stream, body, row_count)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct HealthResponse {
    pub(super) app_name: String,
    pub(super) app_version: String,
    pub(super) schema_version: i64,
    pub(super) database_ok: bool,
    pub(super) message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct VersionResponse {
    pub(super) app_name: String,
    pub(super) app_version: String,
    pub(super) schema_version: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DiscoveryResponse {
    pub(super) host_address: String,
    pub(super) host_port: u16,
    pub(super) app_name: String,
    pub(super) app_version: String,
    pub(super) schema_version: i64,
    pub(super) message: String,
}
