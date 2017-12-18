use errors::*;

use hyper::server::{Http, Request, Response};
use hyper;
use native_tls::{TlsAcceptor, Pkcs12};
use tokio_tls::proto;
use tokio_proto::TcpServer;
use tokio_service;
use futures_cpupool::CpuFuture;

use gotham::handler::NewHandlerService;

use gotham::router::Router;
use gotham;

#[derive(Default,Serialize,Deserialize,StateData)]
struct Session {
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

  println!("Listening on {}", addr);
  let srv = TcpServer::new(proto, addr);
  Ok(srv.serve(|| Ok(new_https_service())))
}

fn new_https_service() -> HTTPSService {
  HTTPSService { http: NewHandlerService::new(router::new()) }
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

mod router {
  use gotham::router::Router;
  use gotham::router::route::dispatch::{new_pipeline_set, finalize_pipeline_set, PipelineSet,
                                        PipelineHandleChain, DispatcherImpl};
  use gotham::middleware::pipeline::new_pipeline;
  use gotham::router::tree::TreeBuilder;
  use gotham::router::tree::node::{NodeBuilder, SegmentType};

  use hyper::Method;
  use gotham::router::request::path::NoopPathExtractor;
  use gotham::router::request::query_string::NoopQueryStringExtractor;
  use gotham::router::response::finalizer::ResponseFinalizerBuilder;
  use gotham::handler::NewHandler;
  use gotham::router::route::{Extractors, Route, RouteImpl, Delegation};
  use gotham::router::route::matcher::MethodOnlyRouteMatcher;
  use gotham::middleware::session::{NewSessionMiddleware, MemoryBackend};

  pub fn new() -> Router {
    let ps_builder = new_pipeline_set();
    let (ps_builder, global) = ps_builder.add(new_pipeline()
      .add(session_middleware())
      .add(oauth_config_middleware())
      .build());
    let (ps_builder, req_authn) = ps_builder.add(new_pipeline()
      .add(require_auth_middleware())
      .build());
    let ps = finalize_pipeline_set(ps_builder);

    let bare_pipeline = (global, ());
    let normal_pipeline = (req_authn, bare_pipeline);

    let mut builder = TreeBuilder::new();
    let mut oauth = NodeBuilder::new("oauth", SegmentType::Static);

    builder.add_route(static_route(vec![Method::Get, Method::Head],
                                   || Ok(super::inventory::handler),
                                   normal_pipeline,
                                   ps.clone()));

    oauth.add_route(static_route(vec![Method::Get, Method::Head],
                                 || Ok(super::oauth_receiver::handler),
                                 bare_pipeline,
                                 ps.clone()));

    builder.add_child(oauth);
    let tree = builder.finalize();

    let response_finalizer_builder = ResponseFinalizerBuilder::new();
    let response_finalizer = response_finalizer_builder.finalize();
    Router::new(tree, response_finalizer)
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

  fn session_middleware() -> NewSessionMiddleware<MemoryBackend, super::Session> {
    NewSessionMiddleware::default().with_session_type::<super::Session>()
  }

  fn oauth_config_middleware() -> super::oauth_config::New {
    super::oauth_config::New {}
  }

  fn require_auth_middleware() -> super::require_authn::New {
    super::require_authn::New {}
  }
}

mod oauth_config {
  use gotham::middleware::{NewMiddleware, Middleware};
  use gotham::state::State;
  use hyper::Request;
  use gotham::handler::HandlerFuture;
  use std::io;
  use std::env;
  use state::AppConfig;
  use futures::{future, Future};

  pub struct New {}

  impl NewMiddleware for New {
    type Instance = Ware;
    fn new_middleware(&self) -> io::Result<Self::Instance> {
      Ok(Ware {})
    }
  }

  pub struct Ware {}

  impl Middleware for Ware {
    fn call<Chain>(self, mut state: State, request: Request, chain: Chain) -> Box<HandlerFuture>
      where Chain: FnOnce(State, Request) -> Box<HandlerFuture> + Send + 'static,
            Self: Sized
    {
      let cfg = AppConfig {
        canonical_url: env::var("CANONICAL_URL").unwrap_or_default(),
        oauth_path: env::var("OAUTH_PATH").unwrap_or_default(),
        api_key: env::var("API_KEY").unwrap_or_default(),
        client_id: env::var("CLIENT_ID").unwrap_or_default(),
        client_secret: env::var("CLIENT_SECRET").unwrap_or_default(),
        access_token: "".to_owned(),
        refresh_token: "".to_owned(),
      };

      state.put(cfg);
      Box::new(future::ok(state).and_then(|state| chain(state, request)))
    }
  }
}

mod require_authn {
  use gotham::middleware::{NewMiddleware, Middleware};
  use gotham::state::State;
  use hyper::{Request, Response, StatusCode};
  use hyper::header::*;
  use gotham::handler::HandlerFuture;
  use std::io;
  use state::AppConfig;
  use oauth;
  use errors::*;
  use gotham::handler::IntoHandlerFuture;

  pub struct New {}

  impl NewMiddleware for New {
    type Instance = Ware;
    fn new_middleware(&self) -> io::Result<Self::Instance> {
      Ok(Ware {})
    }
  }

  pub struct Ware {}

  impl Middleware for Ware {
    fn call<Chain>(self, state: State, request: Request, chain: Chain) -> Box<HandlerFuture>
      where Chain: FnOnce(State, Request) -> Box<HandlerFuture> + Send + 'static,
            Self: Sized
    {
      let response = {
        let session = state.borrow::<super::Session>().unwrap();
        if session.access_token == "".to_owned() {
          let cfg = state.borrow::<AppConfig>().unwrap();
          Some(redirect_response(cfg))
        } else {
          None
        }
      };

      match response {
        Some(result) => {
          match result {
            Ok(r) => (state, r).into_handler_future(),
            Err(e) => {
              let res = Response::new()
                .with_status(StatusCode::InternalServerError)
                .with_body(format!("{}", e))
                .with_header(ContentType::plaintext());
              (state, res).into_handler_future()
            }
          }
        }
        None => chain(state, request),
      }
    }
  }

  fn redirect_response(cfg: &AppConfig) -> Result<Response> {
    let url = cfg.oauth_url()?;
    Ok(Response::new()
      .with_status(StatusCode::SeeOther)
      .with_header(Location::new(oauth::authorize_url(url.as_str(), cfg))))
  }
}

mod oauth_receiver {
  use gotham::state::State;
  use hyper::server::{Request, Response};
  use hyper::StatusCode;
  use hyper::header::Location;
  use state::AppConfig;
  use oauth;
  use errors::*;

  pub fn handler(mut gstate: State, req: Request) -> (State, Response) {
    let token_res = {
      get_oauth_stuff(&mut gstate, req)
    };

    match token_res {
      Ok(()) =>
        (gstate, Response::new()
                    .with_status(StatusCode::Found)
                    .with_header(Location::new("/"))),
      Err(_) => {
        (gstate,
         Response::new()
           .with_status(StatusCode::NotFound)
           .with_body("Something went wrong getting the code and state.\n"))
      }
    }
  }

  fn get_oauth_stuff(state: &mut State, req: Request) -> Result<()> {
    let token = {
      let cfg = state.borrow::<AppConfig>().ok_or(format_err!("No app config in state?"))?;
      oauth::extract_token(cfg, req)?
    };
    let session = state.borrow_mut::<super::Session>().ok_or(format_err!("No session?"))?;
    session.access_token = token.access_token;
    session.refresh_token = token.refresh_token.unwrap_or_default();
    Ok(())
  }
}

mod inventory {
  use ::state;
  use ::destiny;
  use errors::*;
  use gotham::state::State;
  use gotham::http::response::create_response;
  use hyper::server::{Request, Response};
  use hyper::StatusCode;
  use mime;

  pub fn handler(gstate: State, _req: Request) -> (State, Response) {
    let res = match body() {
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

  fn body() -> Result<String> {
    let state = state::load()?;
    Ok(format!("{}",
               destiny::api_exchange(state.access_token, state.api_key)?))
  }
}
