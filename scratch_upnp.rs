use igd_next::search_gateway;

#[tokio::main]
async fn main() {
    match igd_next::aio::search_gateway(Default::default()).await {
        Ok(gateway) => {
            let local_ip = local_ip_address::local_ip().unwrap();
            println!("Found gateway: {:?}", gateway);
            println!("Adding port mapping for {}", local_ip);
            let res = gateway.add_port_mapping(
                igd_next::PortMappingProtocol::TCP,
                7770,
                std::net::SocketAddr::new(local_ip, 7770),
                0,
                "Lake Room",
            ).await;
            println!("Mapped: {:?}", res);
        }
        Err(e) => {
            println!("No gateway: {:?}", e);
        }
    }
}
