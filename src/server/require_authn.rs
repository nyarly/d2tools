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
use gotham::middleware::session::SessionData;
use gotham::state::FromState;

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
      debug!("Require Authn: Getting session from state");
      let cfg = state.borrow::<AppConfig>().unwrap();
      let session = SessionData::<super::D2Session>::borrow_from(&state);
      if session.access_token == "".to_owned() {
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
            error!("Require Authn: {}", e);
            let res = Response::new()
              .with_status(StatusCode::InternalServerError)
              .with_body(format!("Require Authn: {}", e))
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
