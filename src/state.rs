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
