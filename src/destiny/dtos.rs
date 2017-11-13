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
  response: BodyResponse,
  error_code: i32,
  error_status: String,
  message: String,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum BodyResponse {
  UserMembershipData(UserMembershipData),
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct UserMembershipData {
  destiny_memberships: Vec<UserInfoCard>,
  bungie_net_user: GeneralUser,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GeneralUser {
  membership_id: String,
  unique_name: String,
  display_name: String,
  is_deleted: bool,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct UserInfoCard {
  display_name: String,
  #[serde(default)]
  supplemental_display_name: String,
  membership_type: i32,
  membership_id: String,
}
