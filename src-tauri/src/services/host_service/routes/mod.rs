mod master_data;
mod operations;
mod stock;
mod system;

use super::*;

#[derive(Clone)]
pub(super) struct RouteContext<'a> {
    pub(super) runtime: Arc<Mutex<HostServiceRuntime>>,
    pub(super) db: Db,
    pub(super) method: &'a str,
    pub(super) path: &'a str,
    pub(super) body: &'a str,
    pub(super) request: &'a str,
    pub(super) auth_request: &'a str,
    pub(super) app_version: &'a str,
    pub(super) peer_ip: &'a str,
}

pub(super) use master_data::handle_master_data_routes;
pub(super) use operations::handle_operation_routes;
pub(super) use stock::handle_stock_routes;
pub(super) use system::handle_system_routes;
