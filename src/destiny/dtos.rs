use errors::*;
use uritemplate::{TemplateVar, IntoTemplateVar};

#[derive(Deserialize, Debug)]
pub enum BungieMemberType {
  TigerXbox,
  TigerPsn,
  TigerBlizzard,
  TigerDemon,
  BungieNext,
  All,
  Invalid,
}

impl Into<i32> for BungieMemberType {
  fn into(self) -> i32 {
    match self {
      BungieMemberType::TigerXbox => 1,
      BungieMemberType::TigerPsn => 2,
      BungieMemberType::TigerBlizzard => 4,
      BungieMemberType::TigerDemon => 10,
      BungieMemberType::BungieNext => 254,
      BungieMemberType::All => -1,
      BungieMemberType::Invalid => -2,
    }
  }
}

impl From<i32> for BungieMemberType {
  fn from(val: i32) -> BungieMemberType {
    match val {
      1 => BungieMemberType::TigerXbox,
      2 => BungieMemberType::TigerPsn,
      4 => BungieMemberType::TigerBlizzard,
      10 => BungieMemberType::TigerDemon,
      254 => BungieMemberType::BungieNext,
      -1 => BungieMemberType::All,
      _ => BungieMemberType::Invalid,
    }
  }
}

impl IntoTemplateVar for BungieMemberType {
  fn into_template_var(self) -> TemplateVar {
    TemplateVar::Scalar(format!("{}", self as i32))
  }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct ResponseBody {
  pub response: BodyResponse,
  pub error_code: i32,
  pub error_status: String,
  pub message: String,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum BodyResponse {
  UserMembershipData(UserMembershipData),
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UserMembershipData {
  pub destiny_memberships: Vec<UserInfoCard>,
  pub bungie_net_user: GeneralUser,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GeneralUser {
  pub membership_id: String,
  pub unique_name: String,
  pub display_name: String,
  pub is_deleted: bool,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserInfoCard {
  pub display_name: String,
  #[serde(default)]
  pub supplemental_display_name: String,
  pub membership_type: i32,
  pub membership_id: String,
}

impl UserInfoCard {
  pub fn mtype(&self) -> BungieMemberType {
    self.membership_type.into()
  }

  pub fn id(&self) -> Result<i64> {
    Ok(self.membership_id.parse()?)
  }
}
