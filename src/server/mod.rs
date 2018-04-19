use errors::*;

use fern;
use log::LogLevelFilter;
use chrono::prelude::*;
use oauth2::Token;

mod router;
mod app_config;
mod require_authn;
mod oauth_receiver;
mod inventory;

#[derive(Default, Serialize, Deserialize, StateData, Clone)]
struct D2Session {
  #[serde(default)]
  pub token: Option<Token>,
}

impl D2Session {
  fn acquire_token(&mut self, t: Token) {
    self.token = Some(t);
  }
}

pub fn start_http() -> Result<()> {
  let addr = "127.0.0.1:8181";

  configure_logging();

  Ok(::gotham::start(addr, router::new()))
}

fn configure_logging() {
  fern::Dispatch::new()
    .level(LogLevelFilter::Debug)
    .level_for("tokio_core::reactor", LogLevelFilter::Error)
    .level_for("tokio_core", LogLevelFilter::Error)
    .level_for(
      "tokio_proto::streaming::pipeline::advanced",
      LogLevelFilter::Error,
    )
    .chain(::std::io::stdout())
    .format(|out, message, record| {
      out.finish(format_args!(
        "[{}] {}[{}] {}",
        Utc::now().format("%Y-%m-%d %H:%M:%S%.9f"),
        record.target(),
        record.level(),
        message
      ))
    })
    .apply()
    .unwrap();
}
