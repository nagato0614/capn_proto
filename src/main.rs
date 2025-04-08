// モジュール読み込み
mod client;
mod server;
mod subscribe_client;

// Cap’n Proto の自動生成された型と RPC クライアントのコード
#[path = "schema/hello_world_capnp.rs"]
mod hello_world_capnp;

use client::run_client;
use server::run_server;
use subscribe_client::run_event_subscribe_client;

/// Server 側の HelloWorld のロジックを実装
async fn server(socket_path: &str) -> Result<(), capnp::Error> {
    tokio::task::LocalSet::new()
        .run_until(run_server(socket_path))
        .await
        .expect("Failed to run server");
    Ok(())
}

/// Client 側の HelloWorld のロジックを実装
async fn client(socket_path: &str) -> Result<(), capnp::Error> {
    tokio::task::LocalSet::new()
        .run_until(run_client(socket_path))
        .await
        .expect("Failed to run client");
    Ok(())
}

/// Subscribe クライアントの処理を実装
async fn subscribe_client(socket_path: &str) -> Result<(), capnp::Error> {
    tokio::task::LocalSet::new()
        .run_until(run_event_subscribe_client(socket_path))
        .await
        .expect("Failed to run subscribe client");
    Ok(())
}

/// Cap’n Proto RPC のサーバーとクライアントを実装
#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let socket_path = "/tmp/capnp-demo.sock";

    // コマンドライン引数のリストを取得
    let args: Vec<String> = std::env::args().skip(1).collect();

    match args.first().map(|s| s.as_str()) {
        Some("--server") => {
            println!("[server] 起動中...");
            server(socket_path).await?;
        }
        Some("--client") => {
            println!("[client] 起動中...");
            client(socket_path).await?;
        }
        Some("--subscribe-client") => {
            println!("[subscribe-client] 起動中...");
            subscribe_client(socket_path).await?;
        }
        _ => {
            eprintln!(
                "Usage: {} --server | --client | --subscribe-client",
                std::env::args().next().unwrap()
            );
            std::process::exit(1);
        }
    }

    Ok(())
}
