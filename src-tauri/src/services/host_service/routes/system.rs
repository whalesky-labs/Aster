use super::super::*;

pub(crate) fn handle_system_routes<S: Read + Write>(
    stream: &mut S,
    context: RouteContext<'_>,
) -> AppResult<bool> {
    let RouteContext {
        runtime,
        db,
        method,
        path,
        body,
        request,
        auth_request,
        app_version,
        peer_ip,
    } = context;
    match (method, path) {
        ("GET", "/api/health") => {
            let response = health_response(&db, app_version)?;
            write_json(stream, 200, &response)?;
        }
        ("GET", "/api/version") => {
            let response = version_response(&db, app_version)?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/pair/start") => {
            let request: PairStartRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("配对请求解析失败：{error}")))?;
            let response = begin_pairing(&runtime, request, peer_ip.to_string())?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/pair/finish") => {
            let request: PairFinishRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("配对请求解析失败：{error}")))?;
            let response = finish_pairing(&runtime, &db, request)?;
            write_json(stream, 200, &response)?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/clients") => {
            authenticate_request_and_load_client(request, &runtime, &db)?;
            let cursor = query_param(path, "cursor");
            let clients = db.with_conn(|conn| {
                require_remote_admin(auth_request, conn)?;
                crate::db::client_connection_repository::list_page(conn, cursor.as_deref())
            })?;
            write_json(stream, 200, &clients)?;
        }
        ("GET", "/api/status") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let response = db.with_conn(|conn| {
                let current = require_remote_permission(auth_request, conn, "view_reports")?;
                let department_scope = remote_department_scope(&current)?;
                crate::services::status_service::build_app_status(
                    conn,
                    app_version,
                    None,
                    department_scope.as_deref(),
                )
            })?;
            write_json(stream, 200, &response)?;
        }
        ("GET", "/api/system-settings") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let response = db.with_conn(|conn| {
                require_remote_admin(auth_request, conn)?;
                crate::services::status_service::system_settings_from_conn(conn, None)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/backups") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let cursor = query_param(path, "cursor");
            let response = db.with_conn(|conn| {
                require_remote_admin(auth_request, conn)?;
                crate::db::backup_repository::list_backup_records_page(conn, cursor.as_deref())
            })?;
            write_json(stream, 200, &response)?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/users") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let cursor = query_param(path, "cursor");
            let response = db.with_conn(|conn| {
                require_remote_admin(auth_request, conn)?;
                crate::db::user_repository::list_users_page(conn, cursor.as_deref())
            })?;
            write_json(stream, 200, &response)?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/roles") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let cursor = query_param(path, "cursor");
            let response = db.with_conn(|conn| {
                require_remote_admin(auth_request, conn)?;
                crate::db::user_repository::list_roles_page(conn, cursor.as_deref())
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/user") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let request: SaveUserRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("用户请求解析失败：{error}")))?;
            let response = db.with_conn(|conn| {
                require_remote_admin(auth_request, conn)?;
                crate::services::user_service::save_user_on_conn(conn, request, "client")
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/user/enabled") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let request: SetUserEnabledRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("用户状态请求解析失败：{error}")))?;
            db.with_conn(|conn| {
                require_remote_admin(auth_request, conn)?;
                crate::services::user_service::set_user_enabled_on_conn(conn, request, "client")
            })?;
            write_json(stream, 200, &())?;
        }
        ("POST", "/api/login") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let response =
                crate::services::remote_session_service::handle_login(&db, auth_request, body)?;
            clear_login_rate_limit(&runtime, peer_ip, request, body)?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/logout") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            crate::services::remote_session_service::handle_logout(&db, request)?;
            write_json(stream, 200, &())?;
        }
        ("POST", "/api/user/password") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let request: ChangePasswordRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("密码请求解析失败：{error}")))?;
            db.with_conn(|conn| {
                let current = remote_current_user(auth_request, conn)?;
                if request.user_id.as_deref() != Some(current.id.as_str()) {
                    require_remote_admin(auth_request, conn)?;
                }
                crate::services::user_service::change_password_on_conn(
                    conn,
                    request,
                    "client",
                    &current.id,
                )
            })?;
            write_json(stream, 200, &())?;
        }
        ("POST", "/api/password-reset/request") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let request: RequestPasswordResetCodeRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("找回密码请求解析失败：{error}")))?;
            let response = db.with_conn(|conn| {
                crate::services::user_service::request_password_reset_code_on_conn(conn, request)
            })?;
            write_json(stream, 200, &response)?;
        }
        ("POST", "/api/password-reset/confirm") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let request: ResetPasswordWithCodeRequest = serde_json::from_str(body)
                .map_err(|error| AppError::Validation(format!("重置密码请求解析失败：{error}")))?;
            db.with_conn(|conn| {
                crate::services::user_service::reset_password_with_code_on_conn(conn, request)
            })?;
            write_json(stream, 200, &())?;
        }
        ("GET", path) if http_transport::route_matches(path, "/api/audit-logs") => {
            authenticate_request_and_touch_client(request, &runtime, &db)?;
            let limit = query_param(path, "limit").and_then(|value| value.parse::<i64>().ok());
            let cursor = query_param(path, "cursor");
            let response = db.with_conn(|conn| {
                require_remote_admin(auth_request, conn)?;
                repository::list_audit_logs_page(conn, limit.unwrap_or(100), cursor.as_deref())
            })?;
            write_json(stream, 200, &response)?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}
