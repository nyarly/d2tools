use errors::*;
use uritemplate::{TemplateVar, IntoTemplateVar};
use std::collections::HashMap;


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
    let int: i32 = self.into();
    TemplateVar::Scalar(format!("{}", int))
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
#[serde(rename_all = "PascalCase")]
pub struct DebugResponseBody {
  pub response: DestinyProfileResponse,
  pub error_code: i32,
  pub error_status: String,
  pub message: String,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum BodyResponse {
  User(UserMembershipData),
  Manifest(DestinyManifest),
  Profile(DestinyProfileResponse),
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DestinyManifest {
  pub version: String,
  pub mobile_asset_content_path: String,
  pub mobile_gear_asset_data_bases: Vec<GearAssetDataBaseDefinition>,
  pub mobile_clan_banner_database_path: String,
  pub mobile_world_content_paths: HashMap<String, String>,

  #[serde(rename = "mobileGearCDN")]
  pub mobile_gear_cdn: HashMap<String, String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DestinyProfileResponse {
  pub profile_inventory: Option<InventoryComponentResponse>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InventoryComponentResponse {
  pub data: InventoryComponent,
  pub privacy: i32,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InventoryComponent {
  pub items: Vec<Item>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Item {
  pub item_hash: u32,
  pub item_instance_id: Option<String>,
  pub quantity: i32,
}
// omitted for the moment
// bindStatus
// location
// bucketHash
// transferStatuses
// state

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GearAssetDataBaseDefinition {
  version: i32,
  path: String,
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


#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InventoryItemDefinition {
  pub display_properties: DisplayProperties,

  pub item_type_display_name: String,
  pub item_type: i32,
  pub item_sub_type: i32,
  pub quality: Option<QualityBlockDefinition>,
}
// many fields omitted. See online docs

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DisplayProperties {
  pub description: String,
  pub name: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QualityBlockDefinition {
  pub infusion_category_name: String,
}
