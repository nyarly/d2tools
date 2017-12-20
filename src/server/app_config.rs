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

    debug!("AppConfig: putting config in state");
    state.put(cfg);
    Box::new(future::ok(state).and_then(|state| chain(state, request)))
  }
}
