use destiny;
use errors::*;
use gotham::state::State;
use gotham::http::response::create_response;
use hyper::server::Response;
use hyper::StatusCode;
use mime;
use state::AppConfig;

pub fn handler(gstate: State) -> (State, Response) {
  debug!("Assembling inventory");
  let res = match body(&gstate) {
    Ok(string) => create_response(
      &gstate,
      StatusCode::Ok,
      Some((string.into_bytes(), mime::TEXT_PLAIN)),
    ),
    Err(e) => {
      error!("{}", e);
      create_response(
        &gstate,
        StatusCode::InternalServerError,
        Some((format!("{:?}", e).into_bytes(), mime::TEXT_PLAIN)),
      )
    }
  };

  (gstate, res)
}

fn body(state: &State) -> Result<String> {
  let cfg = state
    .try_borrow::<AppConfig>()
    .ok_or(format_err!("No app config in state?"))?;
  let session = state
    .try_borrow::<super::D2Session>()
    .ok_or(format_err!("No session!"))?;
  let token = session
    .clone()
    .token
    .ok_or(format_err!("Not authenticated"))?
    .access_token;
  Ok(format!(
    "{}",
    destiny::api_exchange(token, cfg.api_key.clone())?
  ))
}
