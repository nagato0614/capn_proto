use capnp::capability::Promise;
use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem, pry};
use tokio::net::UnixListener;
use tokio_util::compat::{TokioAsyncReadCompatExt, FuturesAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use crate::hello_world_capnp::hello_world;

pub struct HelloWorldImpl;

impl hello_world::Server for HelloWorldImpl {
    fn say_hello(
        &mut self,
        params: hello_world::SayHelloParams,
        mut results: hello_world::SayHelloResults,
    ) -> Promise<(), capnp::Error> {
        let request = pry!(pry!(params.get()).get_request());
        let name = pry!(request.get_name());
        let value = request.get_value();

        println!("[server] name='{:?}', value={}", name, value);

        let mut reply = results.get().init_reply();
        let message = format!("Hello {:?}, I got {}", name, value);
        reply.set_message(&message);

        Promise::ok(())
    }
}

pub async fn run_server(socket_path: &str) -> anyhow::Result<()> {
    let _ = std::fs::remove_file(socket_path);
    let listener = UnixListener::bind(socket_path)?;

    loop {
        let (stream, _) = listener.accept().await?;
        let (read_half, write_half) = stream.into_split();
        let read_half = read_half.compat();
        let write_half = write_half.compat_write();

        let network = twoparty::VatNetwork::new(
            read_half,
            write_half,
            rpc_twoparty_capnp::Side::Server,
            Default::default(),
        );

        let client = capnp_rpc::new_client::<hello_world::Client, _>(HelloWorldImpl).client;

        let rpc_system = RpcSystem::new(Box::new(network), Some(client));
        tokio::task::spawn_local(rpc_system);
    }
}
