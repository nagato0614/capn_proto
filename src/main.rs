mod server;

#[path = "schema/hello_world_capnp.rs"]
mod hello_world_capnp;

use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};
use tokio::net::UnixStream;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use hello_world_capnp::hello_world;
use server::run_server;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let socket_path = "/tmp/capnp-demo.sock";

    // サーバーモード
    if std::env::args().any(|arg| arg == "--server") {
        println!("[server] 起動中...");
        tokio::task::LocalSet::new().run_until(run_server(socket_path)).await?;
        return Ok(());
    }

    // クライアントモード（LocalSetを使ってspawn_localを許可）
    tokio::task::LocalSet::new().run_until(async {
        let stream = UnixStream::connect(socket_path).await?;
        let (read_half, write_half) = stream.into_split();

        let read_half = read_half.compat();
        let write_half = write_half.compat_write();

        let network = twoparty::VatNetwork::new(
            read_half,
            write_half,
            rpc_twoparty_capnp::Side::Client,
            Default::default(),
        );

        let mut rpc_system = RpcSystem::new(Box::new(network), None);
        let hello_world: hello_world::Client = rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);

        tokio::task::spawn_local(rpc_system); // 👈 LocalSet の中なので OK！

        // sayHello を呼び出す
        let mut request = hello_world.say_hello_request();
        {
            let mut req_struct = request.get().init_request();
            req_struct.set_name("Toru");
            req_struct.set_value(123);
        }

        let response = request.send().promise.await?;
        let reply = response.get()?.get_reply()?;
        let message = reply.get_message()?.to_str()?;

        println!("[client] サーバーからの返答: {}", message);

        Ok(())
    }).await
}
