use capnp::capability::Promise;
use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem, pry};
use tokio::net::UnixListener;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use crate::hello_world_capnp::hello_world;

// サーバー側で実装する HelloWorld のロジック
pub struct HelloWorldImpl;

// Cap’n Proto の RPC サーバー側の処理を実装
impl hello_world::Server for HelloWorldImpl {
    fn say_hello(
        &mut self,
        params: hello_world::SayHelloParams,       // クライアントからのリクエストパラメータ
        mut results: hello_world::SayHelloResults, // クライアントへ返すレスポンス
    ) -> Promise<(), capnp::Error> {
        // リクエストの取得とパース
        let request = pry!(pry!(params.get()).get_request());

        // リクエストの中の name と value を取得
        let name = pry!(request.get_name());
        let value = request.get_value();

        // サーバーで受け取った内容をログ出力
        println!("[server] name='{:?}', value={}", name, value);

        // レスポンスメッセージを作成
        let mut reply = results.get().init_reply();
        let message = format!("Hello {:?}, I got {}", name, value);
        reply.set_message(&message);

        // Promise::ok() で完了を通知
        Promise::ok(())
    }
}

// サーバーを起動する関数
pub async fn run_server(socket_path: &str) -> anyhow::Result<()> {
    // ソケットファイルがすでに存在する場合は削除
    let _ = std::fs::remove_file(socket_path);

    // Unix ドメインソケットで待ち受け開始
    let listener = UnixListener::bind(socket_path)?;

    loop {
        // クライアントからの接続を受け付ける
        let (stream, _) = listener.accept().await?;

        // 読み取り / 書き込みのハーフに分割
        let (read_half, write_half) = stream.into_split();
        let read_half = read_half.compat();
        let write_half = write_half.compat_write();

        // Cap’n Proto の通信ネットワークを構築
        let network = twoparty::VatNetwork::new(
            read_half,
            write_half,
            rpc_twoparty_capnp::Side::Server,
            Default::default(),
        );

        // HelloWorld サービスを Cap’n Proto に登録
        let client = capnp_rpc::new_client::<hello_world::Client, _>(HelloWorldImpl).client;

        // RPC システムを起動してリクエスト処理を開始
        let rpc_system = RpcSystem::new(Box::new(network), Some(client));

        // 非同期タスクとして実行
        tokio::task::spawn_local(rpc_system);
    }
}
