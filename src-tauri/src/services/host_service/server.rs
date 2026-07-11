use super::*;
pub(super) fn serve(
    listener: TcpListener,
    runtime: Arc<Mutex<HostServiceRuntime>>,
    db: Db,
    app_version: String,
    tls_config: Arc<rustls::ServerConfig>,
) {
    let limiter = ConnectionLimiter::new(64, 8);
    loop {
        let running = runtime
            .lock()
            .map(|runtime| runtime.running)
            .unwrap_or(false);
        if !running {
            break;
        }
        match listener.accept() {
            Ok((stream, addr)) => {
                let source = addr.ip().to_string();
                let Some(permit) = limiter.try_acquire(&source) else {
                    drop(stream);
                    continue;
                };
                let runtime = Arc::clone(&runtime);
                let db = db.clone_handle();
                let version = app_version.clone();
                let tls_config = Arc::clone(&tls_config);
                thread::spawn(move || {
                    let _permit = permit;
                    let _ = handle_connection(stream, runtime, db, &version, tls_config);
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(100));
            }
            Err(_) => break,
        }
    }
}

pub(super) fn serve_discovery(
    port: u16,
    runtime: Arc<Mutex<HostServiceRuntime>>,
    db: Db,
    app_version: String,
) {
    let Ok(socket) = UdpSocket::bind(("0.0.0.0", port)) else {
        return;
    };
    let _ = socket.set_read_timeout(Some(Duration::from_millis(500)));
    let mut buffer = [0_u8; 512];
    loop {
        let running = runtime
            .lock()
            .map(|runtime| runtime.running)
            .unwrap_or(false);
        if !running {
            break;
        }
        match socket.recv_from(&mut buffer) {
            Ok((bytes, peer)) => {
                if &buffer[..bytes] != b"ASTER_DISCOVER_V1" {
                    continue;
                }
                let schema_version = db.with_conn(repository::schema_version).unwrap_or_default();
                let response = DiscoveryResponse {
                    host_address: String::new(),
                    host_port: port,
                    app_name: "Aster".to_string(),
                    app_version: app_version.clone(),
                    schema_version,
                    message: "Aster 主机服务可用".to_string(),
                };
                if let Ok(body) = serde_json::to_vec(&response) {
                    let _ = socket.send_to(&body, peer);
                }
            }
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock
                    || error.kind() == std::io::ErrorKind::TimedOut => {}
            Err(_) => break,
        }
    }
}

fn handle_connection(
    stream: TcpStream,
    runtime: Arc<Mutex<HostServiceRuntime>>,
    db: Db,
    app_version: &str,
    tls_config: Arc<rustls::ServerConfig>,
) -> AppResult<()> {
    let monitor_socket = stream.try_clone()?;
    let peer_ip = stream
        .peer_addr()
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|_| "-".to_string());
    let mut stream = secure_transport::accept(stream, tls_config)?;
    let result = match http_transport::read_request(&mut stream) {
        Ok(request) => {
            let cancelled = Arc::new(AtomicBool::new(false));
            let monitor_done = Arc::new(AtomicBool::new(false));
            let monitor = start_disconnect_monitor(monitor_socket, &cancelled, &monitor_done);
            let result = crate::db::connection::with_query_control(
                Duration::from_secs(30),
                Arc::clone(&cancelled),
                || handle_connection_inner(&mut stream, runtime, db, app_version, peer_ip, request),
            );
            monitor_done.store(true, Ordering::Release);
            let _ = monitor.join();
            result
        }
        Err(error) => Err(error),
    };
    if let Err(error) = result {
        let status = http_transport::error_status(&error);
        let message = http_transport::public_error_message(&error);
        write_json(
            &mut stream,
            status,
            &serde_json::json!({ "message": message }),
        )?;
    }
    Ok(())
}

fn start_disconnect_monitor(
    socket: TcpStream,
    cancelled: &Arc<AtomicBool>,
    done: &Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    let cancelled = Arc::clone(cancelled);
    let done = Arc::clone(done);
    thread::spawn(move || {
        let _ = socket.set_read_timeout(Some(Duration::from_millis(100)));
        let mut byte = [0_u8; 1];
        while !done.load(Ordering::Acquire) {
            match socket.peek(&mut byte) {
                Ok(0) => {
                    cancelled.store(true, Ordering::Release);
                    return;
                }
                Ok(_) => thread::sleep(Duration::from_millis(25)),
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        || error.kind() == std::io::ErrorKind::TimedOut => {}
                Err(_) => {
                    cancelled.store(true, Ordering::Release);
                    return;
                }
            }
        }
    })
}

pub(super) fn handle_connection_inner<S: Read + Write>(
    stream: &mut S,
    runtime: Arc<Mutex<HostServiceRuntime>>,
    db: Db,
    app_version: &str,
    peer_ip: String,
    request: String,
) -> AppResult<()> {
    let (method, path) = http_transport::request_line(&request);
    let body = request.split("\r\n\r\n").nth(1).unwrap_or("");
    let auth_request = request.clone();
    enforce_security_rate_limit(&runtime, &path, &peer_ip, &request, body)?;

    let context = RouteContext {
        runtime,
        db,
        method: &method,
        path: &path,
        body,
        request: &request,
        auth_request: &auth_request,
        app_version,
        peer_ip: &peer_ip,
    };
    let handled = handle_system_routes(stream, context.clone())?
        || handle_stock_routes(stream, context.clone())?
        || handle_master_data_routes(stream, context.clone())?
        || handle_operation_routes(stream, context)?;
    if !handled {
        write_json(
            stream,
            404,
            &serde_json::json!({ "message": "Aster host API not found" }),
        )?;
    }
    Ok(())
}
