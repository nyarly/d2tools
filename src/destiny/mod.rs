use std::env;
use std::io::{self,Read,Write};
use std::fs;
use std::path::{Path, PathBuf};
use futures::{Stream};
use futures::future::{self,Future};
use hyper::{self, header, Body, Chunk};
use hyper::client::{Client, Request, HttpConnector};
use hyper_tls::HttpsConnector;
use tokio_core::reactor::Core;
use zip::read::ZipArchive;
use rusqlite::Connection;

use errors::*;

use table;

mod urls;
mod headers;
mod dtos;

use self::dtos::enums;
use super::state;

fn build_client(core: &Core) -> Result<Client<HttpsConnector<HttpConnector>, Body>> {
  let handle = core.handle();
  Ok(Client::configure()
    .connector(HttpsConnector::new(4, &handle)?)
    .build(&handle))
}

fn cache_path(filename: &str) -> Result<PathBuf> {
  let mut path = env::home_dir().ok_or(format_err!("Can't determine $HOME!"))?;
  path.push(".local");
  path.push("cache");
  path.push("d2tools");
  path.push(filename);
  Ok(path)
}

fn database_name_from_path(path: &str) -> Result<String> {
  path.split('/').last().ok_or(format_err!("Couldn't split URL path")).map(|s| s.to_owned())
}

fn store_received_databases(chunk: Chunk) -> Result<()> {
  let body_cursor = io::Cursor::new(chunk);
  let mut zip_reader = ZipArchive::new(body_cursor)?;
  for i in 0..zip_reader.len() {
    let zipfile = zip_reader.by_index(i)?;
    let path = cache_path(zipfile.name())?;
    fs::create_dir_all(path.parent().expect("path has no parent?".into()))?;
    let mut file = fs::File::create(path)?;
    file.write_all(&(zipfile.bytes().collect::<::std::result::Result<Vec<u8>,_>>()?))?;
  }
  Ok(())
}

use self::dtos::Deser;
use failure::ResultExt;

fn unshare_result<T,U: ::std::ops::Deref,E>(res: ::std::result::Result<U, E>) -> Result<U::Target>
where U::Target: Sized + Clone,
      E: ::std::ops::Deref<Target=Error>,
{
  match res {
    Ok(it) => Ok((*it).clone()),
    Err(e) => bail!("{}", *e)
  }
}

pub fn api_exchange(token: String, app_auth: String) -> Result<table::Table<dtos::ItemResponse>> {
  let mut core = Core::new()?;

  let client = build_client(&core)?;
  let authd = AuthGetter::new(client, token, app_auth);

  let content_client = build_client(&core)?;

  let database_path = authd.get(urls::get_manifest()?)
    .and_then(|dl| dtos::ManifestResponseBody::deser(dl))
    .and_then(|mrb| mrb.response.mobile_world_content_paths.get("en")
              .ok_or(format_err!("No 'en' content!"))
              .map(|rurl| rurl.clone())
    )
    .shared();

  let database_stored = database_path.clone()
    .then(|res| unshare_result::<String,_,_>(res))
    .and_then(|urlpath| {
      let dbpath = cache_path(&database_name_from_path(&urlpath)?)?;
      let urlstr = format!("https://www.bungie.net{}", urlpath);
      println!("Expecting db at {:?}", dbpath);
      Ok(
        if !dbpath.is_file() {
          println!("DB not present - downloading...");
          Some(
            future::lazy(move || Ok(urlstr.parse()?))
              .and_then(|url| content_client.get(url).map_err(|e| Error::from(e)) )
              .and_then(|res| res.body().concat2().map_err(|e| Error::from(e)))
              .and_then(|body_chunk| Ok(store_received_databases(body_chunk).with_context(|_| "storing db")?))
          )
        } else {
          None
        }
      ) })
  .flatten()
  .and_then(|_| Ok(println!("DB available"))).shared();

  let database_name = database_path
    .then(|res| unshare_result::<String,_,_>(res))
    .and_then(|urlpath|
              Ok( cache_path(&database_name_from_path(&urlpath)?).context("getting content")?)
      ).shared();

  let card = authd.get(urls::get_membership_data_for_current_user()?)
    .and_then(|dl| dtos::UserResponseBody::deser(dl))
    .and_then(|urb| {
      let res: Result<_> = match urb.response.destiny_memberships.get(0) {
        Some(membership) => Ok(membership.clone()),
        None => bail!("No memberships!"),
      };
      res
    }).shared();

  let profile = card.clone()
    .then(|res| unshare_result::<dtos::UserInfoCard,_,_>(res))
    .and_then(|card| urls::get_profile(card.membership_type, card.id()?, &[
                                       enums::ComponentType::ProfileInventories,
                                       enums::ComponentType::Profiles,
                                       enums::ComponentType::Characters,
                                       enums::ComponentType::CharacterInventories,
                                       enums::ComponentType::CharacterEquipment,
                                       enums::ComponentType::Kiosks,
    ]))
    .and_then(|url|  authd.get(url))
    .and_then(|dl| dtos::ProfileResponseBody::deser(dl))
    .map(|body| body.response)
    .shared();

  let equipment_ids = profile.clone()
    .then(|res| unshare_result::<String,_,_>(res))
    .and_then(|profile| profile.character_equipment.ok_or(format_err!("No equipment!")))
    .and_then(|inv_comp| {
      Ok(inv_comp.data.values().flat_map(|inv| {
        inv.items.iter().filter_map(|it| {
          it.item_instance_id.clone().map(|id| id)
        })
      }).collect::<Vec<_>>())
    });

  let inventory_ids = profile.clone()
    .then(|res| unshare_result::<String,_,_>(res))
    .and_then(|profile| profile.character_inventories.ok_or(format_err!("No inventory!")))
    .and_then(|inv_comp| {
      Ok(inv_comp.data.values().flat_map(|inv| {
        inv.items.iter().filter_map(|it| {
          it.item_instance_id.clone().map(|id| id)
        })
      }).collect::<Vec<_>>())
    });


  let vault_ids = profile.clone()
    .then(|res| unshare_result::<String,_,_>(res))
    .and_then(|profile| profile.profile_inventory.ok_or(format_err!("No vault!")))
    .and_then(|vault| {
      Ok(vault.data.items.iter().filter_map(|it| {
        it.item_instance_id.clone().map(|id| id)
      }).collect::<Vec<_>>())
    });


  let work = card
    .then(|res| unshare_result::<dtos::UserInfoCard,_,_>(res))
    .join4(equipment_ids, vault_ids, inventory_ids)
    .and_then(|(card, ids, vids, iids)| {
      ids.iter().chain(vids.iter()).chain(iids.iter()).map(|id| {
        urls::get_item(card.membership_type, &card.membership_id, &id, &[
                                  enums::ComponentType::ItemCommonData,
                                  enums::ComponentType::ItemInstances,
                                  enums::ComponentType::ItemStats,
                                  enums::ComponentType::ItemSockets,
        ])
      }).collect::<Result<Vec<_>>>()
    })
    .and_then(|urls| {
      future::join_all(urls.iter().map(|url| {
        let database = database_name.clone()
          .then(|res| unshare_result::<String,_,_>(res))
          .join(database_stored.clone().then(|res| unshare_result::<String,_,_>(res)))
          .and_then(|(name, _)| Ok(Connection::open(name).context("opening DB connection")?));

        authd.get(url.clone())
          .and_then(|dl| dtos::ItemResponseBody::deser(dl))
          .map(|res| res.response)
        .join(database)
        .and_then(|(ref mut item, ref db)| {
          item.fetch_component_defs(db);
          let item: dtos::ItemResponse = item.clone();
          Ok(item)
        })
      }).collect::<Vec<_>>())
    })
    .map(|mut items| {
      items.sort_by(|left, right| {
        left.infusion_category().cmp(&right.infusion_category()).then(left.infusion_power().cmp(&right.infusion_power()).reverse())
      });
      items
    })
    .map(|populated_items| {
      table::printer()
        .field("", dtos::ItemResponse::holding_status)
        .field("Bucket Name", dtos::ItemResponse::bucket_name)
        .field("Item Name", dtos::ItemResponse::item_name)
        .field("Item Tier", dtos::ItemResponse::tier)
        .field("Item Kind", dtos::ItemResponse::item_kind)
        .field("Infusion Power", dtos::ItemResponse::infusion_power)
        .field("Effective Power", dtos::ItemResponse::stat_value)
        .field("Infusion Cat.", dtos::ItemResponse::infusion_category)
        .with_items(populated_items)
      });

  Ok(core.run(work)?)
}

struct AuthGetter {
  client: Client<HttpsConnector<HttpConnector>, Body>,
  token: String,
  app_auth: String,
  json_dir: PathBuf,
}

struct RequestAction {
  url: hyper::Uri,
  app_auth: String,
  token: String,
  client: Client<HttpsConnector<HttpConnector>, Body>,
}

use rand::{self,Rng};
use tokio_retry::{strategy,Retry,Action};

impl Action for RequestAction {
  type Future = Box<Future<Item = hyper::Response, Error = hyper::Error>>;
  type Item = hyper::Response;
  type Error = hyper::Error;

  fn run(&mut self) -> Self::Future {
    let mut req = Request::new(hyper::Method::Get, self.url.clone());
    req.headers_mut().set(headers::XApiKey::key(self.app_auth.clone()));
    req.headers_mut().set(header::Accept::json());
    req.headers_mut().set(header::Authorization(header::Bearer { token: self.token.to_owned() }));
    Box::new(self.client.request(req))
  }
}

type Download = ( String, OsString, hyper::Chunk);

impl AuthGetter {
  fn new( client: Client<HttpsConnector<HttpConnector>, Body>, token: String, app_auth: String,) -> AuthGetter {
    let mut json_dir = env::temp_dir();
    json_dir.push("d2tools");
    json_dir.push( &rand::thread_rng().gen_ascii_chars().take(8).collect::<String>());
    AuthGetter{ client, token, app_auth, json_dir }
  }

  // boxed until https://github.com/rust-lang/rust/issues/34511...
  fn get(&self, url: hyper::Uri) -> Box<Future<Item=Download, Error=Error>> {
    let backoff = strategy::ExponentialBackoff::from_millis(10)
      .map(strategy::jitter)
      .take(5);

    let outurl = url.to_string();
    let json_out = self.next_json_path();

    let retry = Retry::spawn( self.client.handle().clone(), backoff, RequestAction{
      url: url,
      app_auth: self.app_auth.clone(),
      token: self.token.clone(),
      client: self.client.clone(),
    });

    Box::new( retry
              .map_err(|e| Error::from(Error::from(e).context("network error")))
              .and_then(|result| {
                match result.status() {
                  hyper::StatusCode::Ok => Ok(result),
                  hyper::StatusCode::Unauthorized => {
                    let mut state = state::load().unwrap();
                    state.access_token = String::new();
                    state.refresh_token = String::new();
                    state::save(state)?;
                    bail!("unauthorized - old token scrubbed, rerun.")
                  }
                  _ => bail!("Other status..."),
                }
              })
              .and_then(|res| {
                res.body().concat2().map_err(|e| Error::from(e))
              })
              .and_then(move |body_chunk| {
                Ok((outurl, json_out, body_chunk))
              }))
  }

  fn next_json_path(&self) -> OsString {
    let mut path = self.json_dir.clone();
    let key: String = rand::thread_rng().gen_ascii_chars().take(8).collect();
    path.push(format!("debug-{}.json", key));
    path.into_os_string()
  }
}


use std::ffi::OsString;
use std::fmt::Debug;

fn write_body<T: AsRef<Path> + Debug + Clone>(path: T, chunk: &hyper::Chunk) {
  match maybe_write_body(path.clone(), chunk) {
    Ok(_) => println!("Wrote debug data to {:?}", path),
    Err(e) => println!("Error writing debug data: {:?}", e)
  }
}
fn maybe_write_body<T: AsRef<Path> + Debug + Clone>(path: T, chunk: &hyper::Chunk) -> Result<()> {
  let pc = path.clone();
  let dir = pc.as_ref().parent().ok_or(format_err!("JSON tempfile has no dir (?!)"))?;
  fs::create_dir_all(dir)?;
  let mut file = fs::File::create(path)?;
  Ok(write!(file, "{}", String::from_utf8_lossy(&(*chunk)))?)
}
