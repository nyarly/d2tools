// for error-chain...
#![recursion_limit="256"]

#[macro_use]
extern crate error_chain;

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

mod errors;
mod state;
mod oauth;
mod destiny;

use errors::*;

quick_main!(main_loop);

fn main_loop() -> Result<()> {
  let state = state::load()?;

  if state.access_token == "" {
    oauth::get()?
  }

  destiny::api_exchange(state.access_token, state.api_key)
}
