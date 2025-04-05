// server.rs をモジュールとして読み込む（サーバー機能）
mod server;

// Cap’n Proto の自動生成された型と RPC クライアントのコード
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

    // コマンドライン引数が "--server" だったらサーバーモード
    if std::env::args().any(|arg| arg == "--server") {
        println!("[server] 起動中...");

        // tokio の LocalSet（spawn_local を使うために必要）
        tokio::task::LocalSet::new().run_until(run_server(socket_path)).await?;
        return Ok(());
    }

    // クライアントモード開始（LocalSet の中で動かす）
    tokio::task::LocalSet::new().run_until(async {
        // Unix ドメインソケットでサーバーに接続
        let stream = UnixStream::connect(socket_path).await?;

        // 読み取り / 書き込みのハーフに分ける（所有権の問題を回避するため）
        let (read_half, write_half) = stream.into_split();

        // Cap’n Proto RPC に渡すための互換ラッパーを適用
        let read_half = read_half.compat();
        let write_half = write_half.compat_write();

        // Cap’n Proto の VatNetwork（通信レイヤー）を構築
        let network = twoparty::VatNetwork::new(
            read_half,
            write_half,
            rpc_twoparty_capnp::Side::Client,
            Default::default(),
        );

        // Cap’n Proto の RPC システムを作成
        let mut rpc_system = RpcSystem::new(Box::new(network), None);

        // サーバーが持っている HelloWorld インターフェースを取得
        let hello_world: hello_world::Client = rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);

        // RPC システムを非同期で動かす（spawn_local は LocalSet の中でしか使えない）
        tokio::task::spawn_local(rpc_system);

        // ここから RPC 呼び出し（sayHello）

        // リクエストを作成
        let mut request = hello_world.say_hello_request();
        {
            let mut req_struct = request.get().init_request();
            req_struct.set_name("Toru"); // name フィールド
            req_struct.set_value(123);   // value フィールド
        }

        // サーバーに送信してレスポンスを待つ
        let response = request.send().promise.await?;

        // サーバーからの返答を取得
        let reply = response.get()?.get_reply()?;
        let message = reply.get_message()?.to_str()?; // Cap’n Proto Text を Rust の文字列に

        // サーバーからの返答を表示
        println!("[client] サーバーからの返答: {}", message);

        Ok(())
    }).await
}
