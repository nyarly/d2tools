use gotham::state::State;
use hyper::server::{Response};
use hyper::StatusCode;
use hyper::Uri;
use hyper::header::Location;
use state::AppConfig;
use oauth;
use errors::*;
use gotham::middleware::session::SessionData;
use gotham::state::FromState;

pub fn handler(mut state: State) -> (State, Response) {
  let token_res = {
    let uri = state.take();
    get_oauth_stuff(&mut state, &uri)
  };

  match token_res {
    Ok(()) =>
      (state, Response::new()
       .with_status(StatusCode::Found)
       .with_header(Location::new("/"))),
    Err(e) => {
      (state,
       Response::new()
         .with_status(StatusCode::NotFound)
         .with_body(format!("Something went wrong getting the code and state: {}\n", e)))
    }
  }
}

fn get_oauth_stuff(state: &mut State, uri: &Uri) -> Result<()> {
  let token = {
    let cfg = state.try_borrow::<AppConfig>().ok_or(format_err!("No app config in state?"))?;
    oauth::extract_token(cfg, uri)?
  };
  let session = SessionData::<super::D2Session>::borrow_mut_from(state);
  session.access_token = token.access_token;
  session.refresh_token = token.refresh_token.unwrap_or_default();
  Ok(())
}
