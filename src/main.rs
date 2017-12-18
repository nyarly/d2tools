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
extern crate gotham;
#[macro_use]
extern crate gotham_derive;
extern crate mime;
extern crate futures_cpupool;

mod state;
mod oauth;
mod destiny;
mod errors;
mod table;
mod server;

fn main() {
  use ::std::io::Write;

  ::std::process::exit(match server::start_https() {
    Ok(_) => 0,
    Err(ref e) => {
      write!(&mut ::std::io::stderr(), "{}\n", e).expect("Error writing to stderr");
      1
    }
  });
}
