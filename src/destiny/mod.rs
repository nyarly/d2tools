use std::{env, fs, io::{self, Read, Write}, path::{Path, PathBuf}};
use futures::{Stream, future::{self, Future, Shared, SharedItem}};
use hyper::{self, header, Body, Chunk, client::{Client, HttpConnector, Request}};
use hyper_tls::HttpsConnector;
use tokio_core::reactor::{Core, Handle};
use zip::read::ZipArchive;
use rusqlite::Connection;

use failure::ResultExt;

use errors::*;

use table;

mod urls;
mod headers;
mod dtos;

use self::dtos::Deser;
use self::dtos::enums;

pub fn api_exchange(token: String, app_auth: String) -> Result<table::Table<dtos::ItemResponse>> {
  let mut core = Core::new()?;
  let content_client = build_client(&core)?;
  let authd = AuthGetter::new(&core, token, app_auth);

  let database_path = fetch_db_path(&authd)?.shared();
  let database_stored = store_db(clone_unshare(&database_path), content_client)?.shared();
  let database_name = get_db_name(unshare(database_path))?.shared();

  let user_card = fetch_card(&authd)?.shared();
  let profile = fetch_profile(clone_unshare(&user_card), &authd)?.shared();

  let equipment_ids = extract_equipment_ids(clone_unshare(&profile));
  let inventory_ids = extract_inventory_ids(clone_unshare(&profile));
  let vault_ids = extract_vault_ids(clone_unshare(&profile));

  let urls = map_urls(unshare(user_card), equipment_ids, vault_ids, inventory_ids);
  let items = fetch_all_items(&authd, urls, database_name, database_stored);
  let work = table_format(items);

  Ok(core.run(work)?)
}

fn unshare<T>(
  future: impl Future<Item = T, Error = impl Debug>,
) -> impl Future<Item = T, Error = Error> {
  future.map_err(|sherr| format_err!("{:?}", sherr))
}

fn clone_unshare<T>(
  future: &(impl Future<Item = T, Error = impl Debug> + Clone),
) -> impl Future<Item = T, Error = Error> {
  future.clone().map_err(|sherr| format_err!("{:?}", sherr))
}

fn cache_path(filename: &str) -> Result<PathBuf> {
  let mut path = env::home_dir().ok_or(format_err!("Can't determine $HOME!"))?;
  path.push(".local");
  path.push("cache");
  path.push("d2tools");
  path.push(filename);
  Ok(path)
}

fn build_client(core: &Core) -> Result<Client<HttpsConnector<HttpConnector>, Body>> {
  let handle = core.handle();
  Ok(
    Client::configure()
      .connector(HttpsConnector::new(4, &handle)?)
      .build(&handle),
  )
}

fn database_name_from_path(path: &str) -> Result<String> {
  path
    .split('/')
    .last()
    .ok_or(format_err!("Couldn't split URL path"))
    .map(|s| s.to_owned())
}

fn store_received_databases(chunk: Chunk) -> Result<()> {
  let body_cursor = io::Cursor::new(chunk);
  let mut zip_reader = ZipArchive::new(body_cursor)?;
  for i in 0..zip_reader.len() {
    let zipfile = zip_reader.by_index(i)?;
    let path = cache_path(zipfile.name())?;
    fs::create_dir_all(path.parent().expect("path has no parent?".into()))?;
    let mut file = fs::File::create(path)?;
    file.write_all(
      &(zipfile
        .bytes()
        .collect::<::std::result::Result<Vec<u8>, _>>()?),
    )?;
  }
  Ok(())
}

fn fetch_db_path(authd: &AuthGetter) -> Result<impl Future<Item = String, Error = Error>> {
  Ok(
    authd
      .get(urls::get_manifest()?)
      .and_then(|dl| dtos::ManifestResponseBody::deser(dl))
      .and_then(|mrb| {
        mrb
          .response
          .mobile_world_content_paths
          .get("en")
          .ok_or(format_err!("No 'en' content!"))
          .map(|rurl| rurl.clone())
      }),
  )
}

fn store_db(
  db_path: impl Future<Item = SharedItem<String>, Error = Error>,
  content_client: hyper::Client<HttpsConnector<HttpConnector>, Body>,
) -> Result<impl Future<Error = Error>> {
  Ok(
    db_path
      .and_then(move |urlpath| {
        let dbpath = cache_path(&database_name_from_path(&urlpath)?)?;
        let urlstr = format!("https://www.bungie.net{}", *urlpath);
        error!("Expecting db at {:?}", dbpath);
        Ok(if !dbpath.is_file() {
          info!("DB not present - downloading...");
          Some(
            future::lazy(move || Ok(urlstr.parse()?))
              .and_then(move |url| content_client.get(url).map_err(|e| Error::from(e)))
              .and_then(|res| res.body().concat2().map_err(|e| Error::from(e)))
              .and_then(
                |body_chunk| Ok(store_received_databases(body_chunk).with_context(|_| "storing db")?),
              ),
          )
        } else {
          None
        })
      })
      .flatten()
      .and_then(|_| Ok(println!("DB available"))),
  )
}

fn get_db_name(
  database_path: impl Future<Item = SharedItem<String>, Error = Error>,
) -> Result<impl Future<Item = PathBuf, Error = Error>> {
  Ok(database_path.and_then(|urlpath| {
    Ok(cache_path(&database_name_from_path(&urlpath)?).context("getting content")?)
  }))
}

fn fetch_card(authd: &AuthGetter) -> Result<impl Future<Item = dtos::UserInfoCard, Error = Error>> {
  Ok(
    authd
      .get(urls::get_membership_data_for_current_user()?)
      .and_then(|dl| dtos::UserResponseBody::deser(dl))
      .and_then(|urb| {
        let res: Result<_> = match urb.response.destiny_memberships.get(0) {
          Some(membership) => Ok(membership.clone()),
          None => bail!("No memberships!"),
        };
        res
      }),
  )
}

fn fetch_profile<'g>(
  card: impl Future<Item = SharedItem<dtos::UserInfoCard>, Error = Error> + 'g,
  authd: &'g AuthGetter,
) -> Result<impl Future<Item = dtos::DestinyProfileResponse, Error = Error> + 'g> {
  Ok(
    card
      .and_then(|card| {
        urls::get_profile(
          card.membership_type,
          card.id()?,
          &[
            enums::ComponentType::ProfileInventories,
            enums::ComponentType::Profiles,
            enums::ComponentType::Characters,
            enums::ComponentType::CharacterInventories,
            enums::ComponentType::CharacterEquipment,
            enums::ComponentType::Kiosks,
          ],
        )
      })
      .and_then(move |url| authd.get(url))
      .and_then(|dl| dtos::ProfileResponseBody::deser(dl))
      .map(|body| body.response),
  )
}

fn extract_equipment_ids(
  profile: impl Future<Item = SharedItem<dtos::DestinyProfileResponse>, Error = Error>,
) -> impl Future<Item = Vec<String>, Error = Error> {
  profile
    .and_then(|profile| {
      (*profile)
        .clone()
        .character_equipment
        .ok_or(format_err!("No equipment!"))
    })
    .and_then(|inv_comp| {
      Ok(
        inv_comp
          .data
          .values()
          .flat_map(|inv| {
            inv
              .items
              .iter()
              .filter_map(|it| it.item_instance_id.clone().map(|id| id))
          })
          .collect::<Vec<_>>(),
      )
    })
}

fn extract_inventory_ids(
  profile: impl Future<Item = SharedItem<dtos::DestinyProfileResponse>, Error = Error>,
) -> impl Future<Item = Vec<String>, Error = Error> {
  profile
    .and_then(|profile| {
      (*profile)
        .clone()
        .character_inventories
        .ok_or(format_err!("No inventory!"))
    })
    .and_then(|inv_comp| {
      Ok(
        inv_comp
          .data
          .values()
          .flat_map(|inv| {
            inv
              .items
              .iter()
              .filter_map(|it| it.item_instance_id.clone().map(|id| id))
          })
          .collect::<Vec<_>>(),
      )
    })
}

fn extract_vault_ids(
  profile: impl Future<Item = SharedItem<dtos::DestinyProfileResponse>, Error = Error>,
) -> impl Future<Item = Vec<String>, Error = Error> {
  profile
    .and_then(|profile| {
      (*profile)
        .clone()
        .profile_inventory
        .ok_or(format_err!("No vault!"))
    })
    .and_then(|vault| {
      Ok(
        vault
          .data
          .items
          .iter()
          .filter_map(|it| it.item_instance_id.clone().map(|id| id))
          .collect::<Vec<_>>(),
      )
    })
}

fn map_urls(
  card: impl Future<Item = SharedItem<dtos::UserInfoCard>, Error = Error>,
  equipment_ids: impl Future<Item = Vec<String>, Error = Error>,
  vault_ids: impl Future<Item = Vec<String>, Error = Error>,
  inventory_ids: impl Future<Item = Vec<String>, Error = Error>,
) -> impl Future<Item = Vec<hyper::Uri>, Error = Error> {
  card
    .join4(equipment_ids, vault_ids, inventory_ids)
    .and_then(|(card, ids, vids, iids)| {
      ids
        .iter()
        .chain(vids.iter())
        .chain(iids.iter())
        .map(|id| {
          urls::get_item(
            card.membership_type,
            &card.membership_id,
            id,
            &[
              enums::ComponentType::ItemCommonData,
              enums::ComponentType::ItemInstances,
              enums::ComponentType::ItemStats,
              enums::ComponentType::ItemSockets,
            ],
          )
        })
        .collect::<Result<Vec<_>>>()
    })
}

fn fetch_all_items<'g>(
  authd: &'g AuthGetter,
  urls: impl Future<Item = Vec<hyper::Uri>, Error = Error> + 'g,
  database_name: Shared<impl Future<Item = PathBuf, Error = Error> + 'g>,
  database_stored: Shared<impl Future<Error = Error> + 'g>,
) -> impl Future<Item = Vec<dtos::ItemResponse>, Error = Error> + 'g {
  urls.and_then(move |urls| {
    future::join_all(
      urls
        .iter()
        .map(|url| {
          let database = clone_unshare(&database_name)
            .join(clone_unshare(&database_stored))
            .and_then(
              |(name, _)| Ok(Connection::open((*name).clone()).context("opening DB connection")?),
            );

          authd
            .get(url.clone())
            .and_then(|dl| dtos::ItemResponseBody::deser(dl))
            .map(|res| res.response)
            .join(database)
            .and_then(|(ref mut item, ref db)| {
              item.fetch_component_defs(db);
              let item: dtos::ItemResponse = item.clone();
              Ok(item)
            })
        })
        .collect::<Vec<_>>(),
    )
  })
}

fn table_format(
  items: impl Future<Item = Vec<dtos::ItemResponse>, Error = Error>,
) -> impl Future<Item = table::Table<dtos::ItemResponse>, Error = Error> {
  items
    .map(|mut items| {
      items.sort_by(|left, right| {
        left
          .infusion_category()
          .cmp(&right.infusion_category())
          .then(left.infusion_power().cmp(&right.infusion_power()).reverse())
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
    })
}

struct RequestAction {
  url: hyper::Uri,
  app_auth: String,
  token: String,
  client: Client<HttpsConnector<HttpConnector>, Body>,
}

use rand::{self, Rng};
use tokio_retry::{strategy, Action, Retry};

impl Action for RequestAction {
  type Future = Box<Future<Item = hyper::Response, Error = hyper::Error>>;
  type Item = hyper::Response;
  type Error = hyper::Error;

  fn run(&mut self) -> Self::Future {
    let mut req = Request::new(hyper::Method::Get, self.url.clone());
    req
      .headers_mut()
      .set(headers::XApiKey::key(self.app_auth.clone()));
    req.headers_mut().set(header::Accept::json());
    req.headers_mut().set(header::Authorization(header::Bearer {
      token: self.token.to_owned(),
    }));
    Box::new(self.client.request(req))
  }
}

type Download = (String, OsString, hyper::Chunk);

struct AuthGetter {
  handle: Handle,
  client: Client<HttpsConnector<HttpConnector>, Body>,
  token: String,
  app_auth: String,
  json_dir: PathBuf,
}

impl AuthGetter {
  fn new(core: &Core, token: String, app_auth: String) -> AuthGetter {
    let mut json_dir = env::temp_dir();
    json_dir.push("d2tools");
    json_dir.push(&rand::thread_rng()
      .gen_ascii_chars()
      .take(8)
      .collect::<String>());
    let client = build_client(core).unwrap();
    let handle = core.handle();
    AuthGetter {
      handle,
      client,
      token,
      app_auth,
      json_dir,
    }
  }

  fn get(&self, url: hyper::Uri) -> impl Future<Item = Download, Error = Error> {
    let backoff = strategy::ExponentialBackoff::from_millis(10)
      .map(strategy::jitter)
      .take(5);

    let outurl = url.to_string();
    let json_out = self.next_json_path();

    let retry = Retry::spawn(
      self.handle.clone(),
      backoff,
      RequestAction {
        url: url,
        app_auth: self.app_auth.clone(),
        token: self.token.clone(),
        client: self.client.clone(),
      },
    );

    retry
      .map_err(|e| Error::from(Error::from(e).context("network error")))
      .and_then(|result| {
        match result.status() {
          hyper::StatusCode::Ok => Ok(result),
          hyper::StatusCode::Unauthorized => {
            // XXX need to actually scrub the old token.
            error!("Unauthorized!");
            bail!("unauthorized - old token scrubbed, rerun.")
          }
          _ => {
            info!("Other status: {}", result.status());
            bail!("Other status from API: {}", result.status())
          }
        }
      })
      .and_then(|res| res.body().concat2().map_err(|e| Error::from(e)))
      .and_then(move |body_chunk| Ok((outurl, json_out, body_chunk)))
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
    Ok(_) => debug!("Wrote debug data to {:?}", path),
    Err(e) => debug!("Error writing debug data: {:?}", e),
  }
}
fn maybe_write_body<T: AsRef<Path> + Debug + Clone>(path: T, chunk: &hyper::Chunk) -> Result<()> {
  let pc = path.clone();
  let dir = pc.as_ref()
    .parent()
    .ok_or(format_err!("JSON tempfile has no dir (?!)"))?;
  fs::create_dir_all(dir)?;
  let mut file = fs::File::create(path)?;
  Ok(write!(file, "{}", String::from_utf8_lossy(&(*chunk)))?)
}
