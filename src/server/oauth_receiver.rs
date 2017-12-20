use gotham::state::State;
use hyper::server::{Request, Response};
use hyper::StatusCode;
use hyper::header::Location;
use state::AppConfig;
use oauth;
use errors::*;
use gotham::middleware::session::SessionData;
use gotham::state::FromState;

pub fn handler(mut gstate: State, req: Request) -> (State, Response) {
  let token_res = {
    get_oauth_stuff(&mut gstate, req)
  };

  match token_res {
    Ok(()) =>
      (gstate, Response::new()
       .with_status(StatusCode::Found)
       .with_header(Location::new("/"))),
    Err(e) => {
      (gstate,
       Response::new()
         .with_status(StatusCode::NotFound)
         .with_body(format!("Something went wrong getting the code and state: {}\n", e)))
    }
  }
}

fn get_oauth_stuff(state: &mut State, req: Request) -> Result<()> {
  let token = {
    let cfg = state.borrow::<AppConfig>().ok_or(format_err!("No app config in state?"))?;
    oauth::extract_token(cfg, req)?
  };
  let session = SessionData::<super::D2Session>::borrow_mut_from(state);
  session.access_token = token.access_token;
  session.refresh_token = token.refresh_token.unwrap_or_default();
  Ok(())
}
