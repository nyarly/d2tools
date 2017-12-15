use errors::*;

use hyper::server::{self, Http, Request, Response};
use hyper::{self, Method, StatusCode};
use native_tls::{TlsAcceptor, Pkcs12};
use tokio_tls::proto;
use tokio_proto::TcpServer;
use tokio_service;
use serde::Serialize;
use futures_cpupool::CpuFuture;

use gotham::handler::NewHandlerService;
use gotham::router::Router;
use gotham::router::tree::TreeBuilder;
use gotham::router::route::{Extractors, Route, RouteImpl, Delegation};
use gotham::router::route::matcher::MethodOnlyRouteMatcher;
use gotham::router::route::dispatch::{new_pipeline_set, finalize_pipeline_set, PipelineSet,
                                      PipelineHandleChain, DispatcherImpl};
use gotham::router::request::path::NoopPathExtractor;
use gotham::router::request::query_string::NoopQueryStringExtractor;
use gotham::state::State;
use gotham::http::response::create_response;
use gotham::middleware::pipeline::new_pipeline;
use gotham::middleware::session::{NewSessionMiddleware, MemoryBackend};
use gotham::handler::NewHandler;
use gotham::router::response::finalizer::ResponseFinalizerBuilder;

use ::state;
use ::destiny;
use mime;

#[derive(Default,Serialize,Deserialize)]
struct Session();

pub fn start_https() -> Result<()> {
  let der = include_bytes!("../identity.p12");
  let cert = Pkcs12::from_der(der, "mypass")?;
  let tls_cx = TlsAcceptor::builder(cert)?.build()?;

  let proto = proto::Server::new(Http::new(), tls_cx);
  let addr = "127.0.0.1:8080".parse()?;

  println!("Listening on {}", addr);
  let srv = TcpServer::new(proto, addr);
  Ok(srv.serve(|| Ok(new_https_service())))
}

fn new_https_service() -> HTTPSService {
  HTTPSService { http: NewHandlerService::new(router()) }
}

fn router() -> Router {
  let ps_builder = new_pipeline_set();
  let (ps_builder, global) = ps_builder.add(new_pipeline()
    .add(session_middleware())
    .build());
  let ps = finalize_pipeline_set(ps_builder);

  let mut builder = TreeBuilder::new();
  builder.add_route(static_route(vec![Method::Get, Method::Head],
                                 || Ok(inventory),
                                 (global, ()),
                                 ps.clone()));
  let tree = builder.finalize();
  let response_finalizer_builder = ResponseFinalizerBuilder::new();
  let response_finalizer = response_finalizer_builder.finalize();
  Router::new(tree, response_finalizer)
}


struct HTTPSService {
  http: NewHandlerService<Router>,
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

fn inventory(gstate: State, _req: Request) -> (State, Response) {
  let res = match inventory_body() {
    Ok(string) => {
      create_response(&gstate,
                      StatusCode::Ok,
                      Some((string.into_bytes(), mime::TEXT_PLAIN)))
    }
    Err(e) => {
      create_response(&gstate,
                      StatusCode::InternalServerError,
                      Some((format!("{}", e).into_bytes(), mime::TEXT_PLAIN)))
    }
  };

  (gstate, res)
}

fn inventory_body() -> Result<String> {
  let state = state::load()?;
  Ok(format!("{}",
             destiny::api_exchange(state.access_token, state.api_key)?))
}

fn session_middleware() -> NewSessionMiddleware<MemoryBackend, Session> {
  NewSessionMiddleware::default().with_session_type::<Session>()
}

fn static_route<NH, P, C>(methods: Vec<Method>,
                          new_handler: NH,
                          active_pipelines: C,
                          ps: PipelineSet<P>)
                          -> Box<Route + Send + Sync>
  where NH: NewHandler + 'static,
        C: PipelineHandleChain<P> + Send + Sync + 'static,
        P: Send + Sync + 'static
{
  let matcher = MethodOnlyRouteMatcher::new(methods);
  let dispatcher = DispatcherImpl::new(new_handler, active_pipelines, ps);
  let extractors: Extractors<NoopPathExtractor, NoopQueryStringExtractor> = Extractors::new();
  let route = RouteImpl::new(matcher,
                             Box::new(dispatcher),
                             extractors,
                             Delegation::Internal);
  Box::new(route)
}
