use crate::proto::{dynamic_update_interface, dynamic_update_interface_grpc};
use crate::zones::AuthZone;
use grpc_helpers::{spawn_service_thread, ServerHandle};
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct DynamicUpdateHandler {
    zones: Arc<RwLock<AuthZone>>,
}

impl DynamicUpdateHandler {
    pub fn new(zones: Arc<RwLock<AuthZone>>) -> Self {
        DynamicUpdateHandler { zones }
    }

    pub fn run(self, ip: String, port: u16) -> ServerHandle {
        let update_service = dynamic_update_interface_grpc::create_dynamic_update_interface(self);
        spawn_service_thread(update_service, ip, port, "dynamic_update_service")
    }
}

impl dynamic_update_interface_grpc::DynamicUpdateInterface for DynamicUpdateHandler {
    fn add_r_rset(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: dynamic_update_interface::AddRRsetRequest,
        sink: ::grpcio::UnarySink<dynamic_update_interface::AddRRsetResponse>,
    ) {
    }

    fn delete_domain(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: dynamic_update_interface::DeleteDomainRequest,
        sink: ::grpcio::UnarySink<dynamic_update_interface::DeleteDomainResponse>,
    ) {
    }

    fn delete_r_rset(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: dynamic_update_interface::DeleteRRsetRequest,
        sink: ::grpcio::UnarySink<dynamic_update_interface::DeleteRRsetResponse>,
    ) {
    }
    fn delete_rdata(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: dynamic_update_interface::DeleteRdataRequest,
        sink: ::grpcio::UnarySink<dynamic_update_interface::DeleteRdataResponse>,
    ) {
    }
    fn update_rdata(
        &mut self,
        ctx: ::grpcio::RpcContext,
        req: dynamic_update_interface::UpdateRdataRequest,
        sink: ::grpcio::UnarySink<dynamic_update_interface::UpdateRdataResponse>,
    ) {
    }
}
