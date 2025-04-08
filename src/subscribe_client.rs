use capnp::capability::Promise;
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};
use tokio::net::UnixStream;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use crate::hello_world_capnp::hello_world;

// クライアント側で EventListener インターフェースを実装する構造体
pub struct EventListenerImpl;

impl hello_world::event_listener::Server for EventListenerImpl {
    fn on_event(
        &mut self,
        params: hello_world::event_listener::OnEventParams,
        _results: hello_world::event_listener::OnEventResults,
    ) -> Promise<(), capnp::Error> {
        let message = pry!(pry!(params.get()).get_message());
        println!("[client] サーバーからのイベント受信: {:?}", message);
        Promise::ok(())
    }
}

// イベントリスナーをサーバーに登録するクライアント関数
pub async fn run_event_subscribe_client(socket_path: &str) -> anyhow::Result<()> {
    let stream = UnixStream::connect(socket_path)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to server: {}", e))?;

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

    // RPC システムを非同期で動かす
    tokio::task::spawn_local(rpc_system);

    // EventListener インスタンスを作成して、クライアントとして渡す
    let event_listener_client = capnp_rpc::new_client(EventListenerImpl);

    // サブスクライブリクエストを送る
    let mut subscribe_request = hello_world.subscribe_events_request();
    subscribe_request.get().set_listener(event_listener_client);

    let response = subscribe_request.send().promise.await;

    match response {
        Ok(_) => println!("[client] サーバーへのイベント登録成功"),
        Err(e) => eprintln!("[client] サーバーへのイベント登録失敗: {:?}", e),
    }

    // サーバーからのイベントを受け取る間、しばらく待機（例えば30秒）
    tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;

    Ok(())
}
