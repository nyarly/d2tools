use errors::*;

use hyper::server::{Http, Request, Response};
use hyper;
use native_tls::{TlsAcceptor, Pkcs12};
use tokio_tls::proto;
use tokio_proto::TcpServer;
use tokio_service;
use futures_cpupool::CpuFuture;

use gotham::service::GothamService;

use gotham::router::Router;
use gotham;
use fern;
use log::LogLevelFilter;
use chrono::prelude::*;

mod router;
mod app_config;
mod require_authn;
mod oauth_receiver;
mod inventory;
mod static_files;

#[derive(Default,Serialize,Deserialize,StateData)]
struct D2Session {
  #[serde(default)]
  pub access_token: String,
  #[serde(default)]
  pub refresh_token: String,
}

pub fn start_https() -> Result<()> {
  let der = include_bytes!("../identity.p12");
  let cert = Pkcs12::from_der(der, "mypass")?;
  let tls_cx = TlsAcceptor::builder(cert)?.build()?;

  let proto = proto::Server::new(Http::new(), tls_cx);
  let addr = "127.0.0.1:8080".parse()?;

  configure_logging();
  info!("Listening on {}", addr);
  let srv = TcpServer::new(proto, addr);
  Ok(srv.serve(|| Ok(new_https_service())))
}

fn new_https_service() -> HTTPSService {
  HTTPSService { http: GothamService::new(router::new()) }
}

struct HTTPSService {
  http: GothamService<Router>,
}

impl tokio_service::Service for HTTPSService {
  type Request = Request;
  type Response = Response;
  type Error = hyper::error::Error;
  type Future = CpuFuture<Self::Response, Self::Error>;

  fn call(&self, req: Self::Request) -> Self::Future {
    self.http.call(req)
  }
}

fn configure_logging() {
  fern::Dispatch::new()
    .level(LogLevelFilter::Debug)
    .level_for("tokio_core::reactor", LogLevelFilter::Error)
    .level_for("tokio_core", LogLevelFilter::Error)
    .level_for("tokio_proto::streaming::pipeline::advanced",
               LogLevelFilter::Error)
    .chain(::std::io::stdout())
    .format(|out, message, record| {
      out.finish(format_args!("[{}] {}[{}] {}",
                              Utc::now().format("%Y-%m-%d %H:%M:%S%.9f"),
                              record.target(),
                              record.level(),
                              message))
    })
    .apply()
    .unwrap();
}
