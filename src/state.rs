use std::env;
use std::fs::File;
use toml;
use std::io::{Read, Write};
use gotham;
use url::Url;

use errors::*;

#[derive(Serialize, Deserialize, Debug, StateData)]
pub struct AppConfig {
  pub canonical_url: String,
  pub oauth_path: String,
  pub api_key: String,
  pub client_id: String,
  pub client_secret: String,

  #[serde(default)]
  pub access_token: String,
  #[serde(default)]
  pub refresh_token: String,
}

impl AppConfig {
  pub fn oauth_url(&self) -> Result<Url> {
    let url: Url = self.canonical_url.parse()?;
    Ok(url.join(&self.oauth_path)?)
  }
}

fn state_path() -> Result<String> {
  let mut path = env::home_dir().ok_or(format_err!("Can't determine $HOME!"))?;
  path.push(".config");
  path.push("d2tools.toml");
  Ok(path.to_str().ok_or(format_err!("Couldn't build state path!"))?.to_owned())
}

pub fn load() -> Result<AppConfig> {
  let path = state_path()?;
  let mut file = File::open(path)?;
  let mut contents = String::new();
  file.read_to_string(&mut contents)?;
  let loaded: AppConfig = toml::from_str(&contents)?;
  Ok(loaded)
}

pub fn save(state: AppConfig) -> Result<()> {
  let contents = toml::to_string(&state)?;
  let mut file = File::create(state_path()?)?;
  file.write_all(contents.as_bytes())?;
  Ok(())
}
