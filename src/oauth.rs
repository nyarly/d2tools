use state;

use native_tls::{TlsAcceptor, Pkcs12};
use tokio_tls::proto;
use hyper::server::Http;
use tokio_proto::TcpServer;
use tokio_service::Service;
use hyper::{self, Request, Response, StatusCode};
use futures::future::{ok, Future};
use std::io;
use oauth2::{Config, Token};
use rand::{self, Rng};
use base64;
use url;

use errors::*;
use state::AppConfig;

pub fn get() -> Result<()> {
  let cfg = state::load()?;

  println!("Open this URL in your browser:\n{}\n",
           authorize_url(cfg.oauth_url()?.as_str(), &cfg));
  oauth_redirection_server()
}

pub fn authorize_url(url: &str, cfg: &AppConfig) -> String {
  let config = oauth_config(url, cfg);
  config.authorize_url().to_string()
}

fn oauth_redirection_server() -> Result<()> {
  // Create our TLS context through which new connections will be
  // accepted. This is where we pass in the certificate as well to
  // send to clients.
  let der = include_bytes!("identity.p12");
  let cert = Pkcs12::from_der(der, "mypass")?;
  let tls_cx = TlsAcceptor::builder(cert)?.build()?;

  // Wrap up hyper's `Http` protocol in our own `Server` protocol. This
  // will run hyper's protocol and then wrap the result in a TLS stream,
  // performing a TLS handshake with connected clients.
  let proto = proto::Server::new(Http::new(), tls_cx);
  let addr = "127.0.0.1:8080".parse()?;

  println!("Listening on {}", addr);
  let srv = TcpServer::new(proto, addr);
  srv.serve(|| Ok(OauthReceiver));
  Ok(())
}

struct OauthReceiver;

impl Service for OauthReceiver {
  type Request = Request;
  type Response = Response;
  type Error = io::Error;
  type Future = Box<Future<Item = Response, Error = io::Error>>;

  fn call(&self, req: Request) -> Self::Future {
    match get_oauth_stuff(req) {
      Ok(()) =>
        Box::new(ok(Response::new()
                    .with_status(StatusCode::Ok)
                    .with_body("Return to the terminal.\n"))),
      Err(_) => {
        Box::new(ok(Response::new()
          .with_status(StatusCode::Ok)
          .with_body("Something went wrong getting the code and state.\n")))
      }
    }

  }
}

fn oauth_config(url: &str, cfg: &AppConfig) -> Config {
  let auth_url = "https://www.bungie.net/en/oauth/authorize";
  let token_url = "https://www.bungie.net/platform/app/oauth/token/";

  let mut config = Config::new(cfg.client_id.clone(),
                               cfg.client_secret.clone(),
                               auth_url,
                               token_url);

  config = config.set_redirect_url(url);
  // Generate the authorization URL to which we'll redirect the user.
  config.set_state(base64::encode(&rand::thread_rng()
    .gen_iter::<u8>()
    .take(32)
    .collect::<Vec<_>>()))
}

fn get_oauth_stuff(req: Request) -> Result<()> {
  let cfg = state::load()?;

  let token = extract_token(&cfg, req)?;

  let mut state = state::load()?;
  state.access_token = token.access_token;
  state.refresh_token = token.refresh_token.unwrap_or("".into());
  state::save(state)?;
  println!("Token recorded! Re-run d2tools.");
  ::std::process::exit(0)
}

pub fn extract_token(cfg: &AppConfig, req: Request) -> Result<Token> {
  let code = value_from_query(req.uri(), "code")?;
  let state = value_from_query(req.uri(), "state")?;

  println!("Oauth code:\n{}\n", code);
  println!("Echoed state:\n{}\n", state);

  let config = oauth_config(cfg.oauth_url()?.as_str(), &cfg);

  Ok(config.exchange_code(code)?)
}

fn value_from_query(uri: &hyper::Uri, name: &str) -> Result<String> {
  let query_string = uri.query().ok_or(format_err!("No query part"))?;
  let pair = url::form_urlencoded::parse(query_string.as_bytes()).find(|pair| {
      let &(ref key, _) = pair;
      key == name
    })
    .ok_or(format_err!("key not found"))?;
  let (_, value) = pair;
  Ok(value.into_owned())
}
