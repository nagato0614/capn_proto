@0x9c50f680ea4721cc;

interface HelloWorld {
    struct HelloRequest {
        name @0 :Text;
        value @1 :Int32;  # ← 整数を送るために追加
    }

    struct HelloReply {
        message @0 :Text;
    }

    sayHello @0 (request: HelloRequest) -> (reply: HelloReply);
}