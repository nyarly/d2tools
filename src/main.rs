#[macro_use]
extern crate failure;

#[macro_use]
extern crate serde_derive;

extern crate futures;
extern crate hyper;
extern crate hyper_tls;
extern crate native_tls;
extern crate tokio_service;
extern crate tokio_core;
extern crate tokio_proto;
extern crate tokio_tls;
extern crate url;
extern crate uritemplate;
extern crate oauth2;
extern crate rand;
extern crate base64;
extern crate toml;
extern crate serde;
extern crate serde_json;
extern crate zip;
extern crate rusqlite;
extern crate tokio_retry;

mod state;
mod oauth;
mod destiny;
mod errors;

use errors::*;

fn main() {
  use ::std::io::Write;

  ::std::process::exit(match main_loop() {
    Ok(_) => 0,
    Err(ref e) => {
      write!(&mut ::std::io::stderr(), "{}", e).expect("Error writing to stderr");
      1
    }
  });
}

fn main_loop() -> Result<()> {
  let state = state::load()?;

  if state.access_token == "" {
    oauth::get()?
  }

  destiny::api_exchange(state.access_token, state.api_key)
}
