use hyper::{self, Request};
use oauth2::{Config, Token};
use rand::{self, Rng};
use base64;
use url;

use errors::*;
use state::AppConfig;

pub fn authorize_url(url: &str, cfg: &AppConfig) -> String {
  let config = oauth_config(url, cfg);
  config.authorize_url().to_string()
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
