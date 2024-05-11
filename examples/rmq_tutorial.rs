use amqprs::connection::OpenConnectionArguments;
use serde::{Deserialize, Serialize};

extern crate rabbit_rpc;

use rabbit_rpc::start_rpc_server;
use rabbit_rpc::RPCClient;


#[derive(Debug, Serialize, Deserialize)]
struct TestMsg {
    number : i32,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    let client_args  = OpenConnectionArguments::new("localhost", 5672, "guest", "guest");
    let server_args = client_args.clone();


    let handle = tokio::spawn(async move {
        let rpc_fn = Box::new(move |msg: TestMsg| {

            fn fibbonacchi( n : i32)->i32 { match n {
                1 | 2 => 1,
                _     => fibbonacchi(n-1) + fibbonacchi(n-2),
            } }

            fibbonacchi(msg.number)
        });
        start_rpc_server(server_args, "test", rpc_fn, true).await.unwrap()
    });

    let mut client = RPCClient::new(&client_args).await.unwrap();
    let out: i32 = client
        .call(
            "test",
            TestMsg {
                number: 5,
            },
        )
        .await
        .unwrap();
    println!("{:?}", out);
    handle.await.unwrap();
}
