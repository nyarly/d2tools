use std::env;
use std::fs::File;
use toml;
use errors::*;
use std::io::{Read, Write};

#[derive(Serialize, Deserialize, Debug)]
pub struct State {
  pub api_key: String,
  pub client_id: String,
  pub client_secret: String,

  #[serde(default)]
  pub access_token: String,
  #[serde(default)]
  pub refresh_token: String,
}

fn state_path() -> Result<String> {
  let mut path = env::home_dir().ok_or("Can't determine $HOME!")?;
  path.push(".config");
  path.push("d2tools.toml");
  Ok(path.to_str().ok_or("Couldn't build state path!")?.to_owned())
}

pub fn load() -> Result<State> {
  let path = state_path()?;
  let mut file = File::open(path)?;
  let mut contents = String::new();
  file.read_to_string(&mut contents)?;
  let loaded: State = toml::from_str(&contents)?;
  Ok(loaded)
}

pub fn save(state: State) -> Result<()> {
  let contents = toml::to_string(&state)?;
  let mut file = File::create(state_path()?)?;
  file.write_all(contents.as_bytes())?;
  Ok(())
}
