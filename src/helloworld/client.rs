extern crate bytes;
extern crate env_logger;
extern crate http;
extern crate futures;
extern crate log;
extern crate prost;
extern crate tokio;
extern crate tower_h2;
extern crate tower_add_origin;
extern crate tower_grpc;
extern crate tower_service;
extern crate tower_util;
use tokio::runtime::{Runtime};
use futures::future::{loop_fn, Loop, FutureResult};

use futures::{Future, Poll, Async};
use tokio::executor::DefaultExecutor;
use tokio::net::tcp::{ConnectFuture, TcpStream};
use tower_grpc::Request;
use tower_h2::client;
use tower_service::Service;
use tower_util::MakeService;
use hello_world::client::Greeter;
use hello_world::HelloRequest;
use stopwatch::{Stopwatch};

pub mod hello_world {
    include!(concat!(env!("OUT_DIR"), "/helloworld.rs"));
}

pub fn main() {
    let _ = ::env_logger::init();

    let mut rt = Runtime::new().unwrap();
    let uri: http::Uri = format!("http://localhost:50051").parse().unwrap();

    let h2_settings = Default::default();
    let mut make_client = client::Connect::new(Dst, h2_settings, rt.executor());

    let conn_l = rt.block_on(make_client.make_service(())).unwrap();
    let conn = tower_add_origin::Builder::new()
            .uri(uri)
            .build(conn_l)
            .unwrap();
    let mut client = Greeter::new(conn);
    let total_reqs = 100;
    let sw = Stopwatch::start_new();
    rt.block_on(loop_fn(0, move |i| {
        client.say_hello(Request::new(HelloRequest {
                name: "What is in a name?".to_string(),
            })).map_err(|e| panic!("gRPC request failed; err={:?}", e))
        .and_then(move |_|{

            if i == total_reqs {
                Ok(Loop::Break(i))
            } else {
                Ok(Loop::Continue(i+1))
            }
        })
    })).unwrap();
    let avg_ms_per_req = sw.elapsed_ms() / total_reqs;
    println!("rps: {} avg_ms: {}", 1000.0 / (avg_ms_per_req as f32), avg_ms_per_req);
}

struct Dst;

impl Service<()> for Dst {
    type Response = TcpStream;
    type Error = ::std::io::Error;
    type Future = Box<Future<Item=TcpStream, Error=::std::io::Error> + Send>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Ok(().into())
    }

    fn call(&mut self, _: ()) -> Self::Future {
        Box::new(TcpStream::connect(&([127, 0, 0, 1], 50051).into())
        .map(|stream| {stream.set_nodelay(true).unwrap(); stream})
        .map_err(|err| err))
    }
}

