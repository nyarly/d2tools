use errors::*;
use std::collections::HashMap;
use std::cmp;

pub mod enums;

pub trait Deser
  where Self: ::std::marker::Sized
{
  fn deser(value: Download) -> Result<Self>;
}

use destiny::{Download, write_body};

macro_rules! body_wrapper{
  ($inner:ident, $outer:ident) => {
  #[derive(Deserialize, Debug)]
  #[serde(rename_all = "PascalCase")]
  pub struct $outer {
    pub response: $inner,
    pub error_code: i32,
    pub error_status: String,
    pub message: String,
  }

  impl Deser for $outer {
    fn deser(value: Download) -> Result<$outer> {
      let (outurl, json_out, body_chunk) = value;
      println!("{}", outurl);
      write_body(&json_out, &body_chunk);
      Ok(serde_json::from_slice(&body_chunk).chain_err(|| format!("deserializing JSON: recorded at {:?}", json_out))?)
    }
  }
  }
}

body_wrapper!(ItemResponse, ItemResponseBody);
body_wrapper!(UserMembershipData, UserResponseBody);
body_wrapper!(DestinyManifest, ManifestResponseBody);
body_wrapper!(DestinyProfileResponse, ProfileResponseBody);

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

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DestinyProfileResponse {
  pub profile_inventory: Option<InventoryComponentResponse>,
  pub character_equipment: Option<CharacterEquipmentComponentResponse>,
  pub character_inventories: Option<CharacterEquipmentComponentResponse>,
  pub item_components: ItemComponentSet,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ItemComponentSet {
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ItemResponse {
  // stats SingleItemStats
  // sockets SingleItemSockets
  pub character_id: Option<String>, // API says i64...
  pub item: Option<SingleItem>,
  pub instance: Option<SingleItemInstance>,
  pub sockets: Option<ItemSocketsComponent>,
  bucket: Option<InventoryBucketDefinition>,
  item_def: Option<InventoryItemDefinition>,

  #[serde(skip)]
  pub plug_defs: Vec<ItemSocketState>,
}

use rusqlite::Connection;
use serde_json;

fn fetch_plug_def(hash: i32, db: &Connection) -> Result<InventoryItemDefinition> {
  let mut stmt =
    db.prepare_cached("select json from DestinyInventoryItemDefinition where id = ?1")?;
  stmt.query_row(&[&hash], |row| {
      let json: String = row.get(0);
      serde_json::from_str(&json).chain_err(|| format!("deserializing JSON: {}", hash))
    })
    .chain_err(|| format!("{}", hash))
    .and_then(|res| res)
}

impl ItemResponse {
  pub fn fetch_component_defs<'f, 'g>(&'f mut self, db: &'g Connection) {
    match self.fetch_item_def(db)
      .and(self.fetch_bucket_def(db))
      .and(self.fetch_plug_defs(db)) {
      Ok(_) => (),
      Err(e) => println!("Problem getting defs for {:?}: {:?}", self, e),
    }
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
        let bucket: InventoryBucketDefinition =
          serde_json::from_str(&json).chain_err(|| format!("{}", json))?;
        self.bucket = Some(bucket);
        Ok(())
      }
      None => bail!("No bucket def for hash!"),
    }
  }

  fn fetch_plug_defs<'f, 'g>(&'f mut self, db: &'g Connection) -> Result<()> {
    self.plug_defs = self.plug_hashes()
      .iter()
      .map(|sock| {
        let mut sock = sock.clone();
        match sock.plug_hash {
          Some(hash) => {
            match fetch_plug_def(hash as i32, db) {
              Ok(v) => {
                sock.plug_def = Some(v);
              }
              Err(e) => {
                println!("{:?}", e);
              }
            }
          }
          None => (),
        };
        sock
      })
      .collect();
    Ok(())
  }

  fn plug_hashes(&self) -> Vec<ItemSocketState> {
    self.sockets
      .clone()
      .and_then(|sockc| sockc.data.map(|socks| socks.sockets))
      .unwrap_or_default()
  }

  pub fn bucket_name(&self) -> String {
    self.bucket.clone().map_or("".to_owned(),
                               |b| b.display_properties.name.unwrap_or_default())
  }

  pub fn item_hash(&self) -> Result<i32> {
    self.item.clone().ok_or("No item!".into()).map(|i| i.data.item_hash as i32)
  }

  pub fn bucket_hash(&self) -> Result<i32> {
    self.item.clone().ok_or("No item!".into()).map(|i| i.data.bucket_hash as i32)
  }

  pub fn item_name(&self) -> String {
    self.item_def.clone().map_or("".to_owned(),
                                 |i| i.display_properties.name.unwrap_or_default())
  }

  pub fn level(&self) -> String {
    self.instance.clone().map_or("".to_owned(), |inst| format!("{}", inst.data.item_level))
  }

  pub fn stat_value(&self) -> String {
    format!("{}", self.stat_num())
  }

  fn stat_num(&self) -> i32 {
    self.instance.clone().map_or(0,
                                 |inst| inst.data.primary_stat.map(|s| s.value).unwrap_or(0))
  }

  pub fn infusion_category(&self) -> String {
    self.item_def.clone().map_or("".to_owned(),
                                 |i| i.quality.map_or("".to_owned(), |q| q.infusion_category_name))
  }

  pub fn infusion_power(&self) -> String {
    if self.plug_defs.iter().any(|sock| sock.bumps_power()) {
      format!("{}", cmp::max(0, self.stat_num() - 5))
    } else {
      self.stat_value()
    }
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
pub struct ItemSocketsComponent {
  pub data: Option<ItemSockets>,
  pub privacy: i32,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ItemSockets {
  pub sockets: Vec<ItemSocketState>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ItemSocketState {
  pub plug_hash: Option<u32>,
  pub is_enabled: bool,
  pub enable_fail_indexes: Option<Vec<i32>>,
  pub reusable_plug_hashes: Option<Vec<u32>>,
  plug_def: Option<InventoryItemDefinition>,
}

impl ItemSocketState {
  pub fn plug_name(&self) -> String {
    self.plug_def.clone().map_or("".to_owned(),
                                 |plug| plug.display_properties.name.unwrap_or_default())
  }
  pub fn plug_type(&self) -> String {
    self.plug_def.clone().map_or("".to_owned(), |plug| plug.item_type_display_name)
  }
  pub fn plug_tier(&self) -> String {
    format!("{:?}", self.tier())
  }

  fn tier(&self) -> enums::TierType {
    self.plug_def.clone().map_or(enums::TierType::Unknown, |plug| plug.inventory.tier_type)
  }

  pub fn category_id(&self) -> String {
    self.plug_def.clone().map_or("".to_owned(), |plug| {
      plug.plug.map_or("".to_owned(), |plug| plug.plug_category_identifier)
    })
  }
  pub fn is_enabled(&self) -> String {
    format!("{}", self.is_enabled)
  }

  fn bumps_power(&self) -> bool {
    let cat = self.category_id();
    self.tier() == enums::TierType::Legendary &&
    (cat.contains("enhancements.") || cat.contains(".weapon.damage_type."))
  }
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
  pub primary_stat: Option<Stat>,
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
  pub category: i32,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InventoryItemDefinition {
  pub display_properties: DisplayProperties,

  pub item_type_display_name: String,
  pub item_type: i32,
  pub item_sub_type: i32,
  pub quality: Option<QualityBlockDefinition>,
  pub plug: Option<PlugDefinition>,
  pub investment_stats: Vec<InvestmentStatDefinition>,
  pub inventory: InventoryBlockDefinition,
}
// many fields omitted. See online docs

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InvestmentStatDefinition {
  pub stat_type_hash: u32,
  pub value: i32,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InventoryBlockDefinition {
  pub stack_unique_label: Option<String>,
  pub max_stack_size: i32,
  pub bucket_type_hash: u32,
  pub recovery_bucket_type_hash: Option<u32>,
  pub is_instance_item: bool,
  pub tier_type: enums::TierType,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlugDefinition {
  pub insertion_rules: Vec<PlugRuleDefinition>,
  pub plug_category_identifier: String,
  pub on_action_recreate_self: bool,
  pub insertion_material_requirement_hash: Option<u32>,
  pub enabled_material_requirement_hash: Option<u32>,
  pub enabled_rules: Vec<PlugRuleDefinition>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlugRuleDefinition {
  pub failure_message: String,
}

// ("{\"
// displayProperties\":{\"hasIcon\":false},\"scope\":0,\"category\":0,\"bucketOrder\":0,\"itemCount\":1,\"location\":1,\"hasTransferDestination\":false,\"enabled\":false,\"fifo\":t
// rue,\"hash\":2422292810,\"index\":36,\"redacted\":false}"), State { next_error: Some(ErrorImpl { code: Message("missing field `description`"), line: 1, column: 38 })
//
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DisplayProperties {
  pub description: Option<String>,
  pub name: Option<String>,
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

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CharacterEquipmentComponentResponse {
  pub data: HashMap<String, InventoryComponent>,
  pub privacy: i32,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InventoryComponentResponse {
  pub data: InventoryComponent,
  pub privacy: i32,
}

#[derive(Deserialize, Debug, Clone)]
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
