use errors::*;
use std::collections::HashMap;

pub mod enums;

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
  Item(ItemResponse),
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
  pub character_equipment: Option<HashMap<String, InventoryComponent>>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ItemResponse {
  // stats SingleItemStats
  // sockets SingleItemSockets
  pub character_id: Option<String>, // API says i64...
  pub item: Option<SingleItem>,
  pub instance: Option<SingleItemInstance>,
  bucket: Option<InventoryBucketDefinition>,
  item_def: Option<InventoryItemDefinition>,
}

use rusqlite::Connection;
use serde_json;

impl ItemResponse {
  pub fn fetch_component_defs<'f, 'g>(&'f mut self, db: &'g Connection) {
    self.fetch_item_def(db)
      .and(self.fetch_bucket_def(db));
  }

  fn fetch_item_def<'f, 'g>(&'f mut self, db: &'g Connection) -> Result<()> {
    let mut stmt =
      db.prepare_cached("select json from DestinyInventoryItemDefinition where id = ?1")?;
    let mut rows = stmt.query(&[&(self.item_hash()?)])?;
    match rows.next() {
      Some(row) => {
        let json: String = row?.get(0);
        let item: InventoryItemDefinition = serde_json::from_str(&json)?;
        self.item_def = Some(item);
        Ok(())
      }
      None => bail!("No item def for hash!"),
    }
  }

  fn fetch_bucket_def<'f, 'g>(&'f mut self, db: &'g Connection) -> Result<()> {
    let mut stmt =
      db.prepare_cached("select json from DestinyInventoryBucketDefinition where id = ?1")?;
    let mut rows = stmt.query(&[&(self.bucket_hash()?)])?;
    match rows.next() {
      Some(row) => {
        let json: String = row?.get(0);
        let bucket: InventoryBucketDefinition = serde_json::from_str(&json)?;
        self.bucket = Some(bucket);
        Ok(())
      }
      None => bail!("No bucket def for hash!"),
    }
  }

  pub fn bucket_name(&self) -> String {
    self.bucket.clone().map_or("".to_owned(), |b| b.display_properties.name)
  }

  pub fn item_hash(&self) -> Result<i32> {
    self.item.clone().ok_or("No item!".into()).map(|i| i.data.item_hash as i32)
  }

  pub fn bucket_hash(&self) -> Result<i32> {
    self.item.clone().ok_or("No item!".into()).map(|i| i.data.bucket_hash as i32)
  }

  pub fn item_name(&self) -> String {
    self.item_def.clone().map_or("".to_owned(), |i| i.display_properties.name)
  }

  pub fn level(&self) -> String {
    self.instance.clone().map_or("".to_owned(), |inst| format!("{}", inst.data.item_level))
  }

  pub fn stat_value(&self) -> String {
    self.instance.clone().map_or("".to_owned(),
                                 |inst| format!("{}", inst.data.primary_stat.value))
  }

  pub fn infusion_category(&self) -> String {
    self.item_def.clone().map_or("".to_owned(),
                                 |i| i.quality.map_or("".to_owned(), |q| q.infusion_category_name))
  }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SingleItem {
  pub data: Item,
  pub privacy: i32,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SingleItemInstance {
  pub data: ItemInstance,
  pub privacy: i32,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Item {
  // omitted for the moment:
  // bindStatus
  // location
  // transferStatuses
  // state
  pub item_hash: u32,
  pub item_instance_id: Option<String>,
  pub quantity: i32,
  pub bucket_hash: u32,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ItemInstance {
  // damageType
  // damageTypeHash
  pub primary_stat: Stat,
  pub item_level: i32,
  pub quality: i32,
  pub is_equipped: bool,
  pub can_equip: bool,
  pub equip_required_level: i32,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InventoryBucketDefinition {
  pub display_properties: DisplayProperties,
  pub scope: i32,
  pub bucket_category: i32,
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

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Stat {
  // maximum_value //unreliable, per docs
  pub stat_hash: u32,
  pub value: i32,
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
  pub membership_type: enums::BungieMemberType,
  pub membership_id: String,
}

impl UserInfoCard {
  pub fn id(&self) -> Result<i64> {
    Ok(self.membership_id.parse()?)
  }
}
