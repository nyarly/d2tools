use std::env;
use std::io::{self,Read,Write};
use std::fs;
use std::path::PathBuf;
use std::convert;
use errors::*;
use futures::Stream;
use futures::future::{self,Future};
use hyper::{self, header, Body, Chunk};
use hyper::client::{Client, Request, HttpConnector};
use hyper_tls::HttpsConnector;
use tokio_core::reactor::Core;
use serde_json;
use zip::read::ZipArchive;
use rusqlite::Connection;

mod urls;
mod headers;
mod dtos;

use super::state;

struct AuthGetter {
  client: Client<HttpsConnector<HttpConnector>, Body>,
  token: String,
  app_auth: String,
}

fn build_client(core: &Core) -> Result<Client<HttpsConnector<HttpConnector>, Body>> {
  let handle = core.handle();
  Ok(Client::configure()
    .connector(HttpsConnector::new(4, &handle)?)
    .build(&handle))
}

fn cache_path(filename: &str) -> Result<PathBuf> {
  let mut path = env::home_dir().ok_or("Can't determine $HOME!")?;
  path.push(".local");
  path.push("cache");
  path.push("d2tools");
  path.push(filename);
  Ok(path)
}

fn database_name_from_path(path: &str) -> Result<String> {
  path.split('/').last().ok_or("Couldn't split URL path".into()).map(|s| s.to_owned())
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

fn unshare_result<T,U: ::std::ops::Deref,E: ::std::ops::Deref>(res: ::std::result::Result<U, E>) -> Result<U::Target>
where U::Target: Sized + Clone,
      E::Target: Sized,
      Error: convert::From<E::Target>
{
  match res {
    Ok(it) => Ok((*it).clone()),
    Err(_) => bail!("just broken")
  }
}

pub fn api_exchange(token: String, app_auth: String) -> Result<()> {
  let mut core = Core::new()?;

  let client = build_client(&core)?;
  let authd = AuthGetter::new(client, token, app_auth);

  let content_client = build_client(&core)?;

  let database_path = authd.get(urls::get_manifest()?)
    .and_then(|rb| {
      let res: Result<_> = match rb.response {
        dtos::BodyResponse::Manifest(mani) => {
          mani.mobile_world_content_paths.get("en")
            .ok_or("No 'en' content!".into())
            .map(|rurl| rurl.clone())
        },
        _ => bail!("Response not a DestinyManifest!")
      };
      res
    }).shared();

  let database_stored = database_path.clone()
    .then(|res| unshare_result::<String,_,_>(res))
    .and_then(|urlpath| {
      let dbpath = cache_path(&database_name_from_path(&urlpath)?)?;
      let urlstr = format!("https://www.bungie.net{}", urlpath);
      Ok(
        if !fs::metadata(dbpath)?.is_file() {
          println!("DB not present - downloading...");
          Some(
          future::lazy(move || urlstr.parse()
                       .map_err(|err| Error::with_chain(err, "parsing content url")))
            .and_then(|url| {
              content_client.get(url)
                .map_err(|err| Error::with_chain(err, "getting content"))
            })
          .and_then(|res| res.body().concat2()
                    .map_err(|e| Error::with_chain(e, "assembling body from stream")))
          .and_then(|body_chunk| store_received_databases(body_chunk))
          )
        } else {
          None
        }
      )
      })
  .flatten()
  .and_then(|_| Ok(println!("DB available")));

  let database_name = database_path
    .then(|res| unshare_result::<String,_,_>(res))
    .and_then(|urlpath|
      cache_path(&database_name_from_path(&urlpath)?)
              .map_err(|err| Error::with_chain(err, "getting content"))
      );

  let database = database_name.join(database_stored)
    .and_then(|(name, _)| Connection::open(name)
              .map_err(|err| Error::with_chain(err, "opening DB connection")));

  let profile = authd.get(urls::get_membership_data_for_current_user()?)
    .and_then(|rb| {
      let res: Result<_> = match rb.response {
        dtos::BodyResponse::User(data) => {
          match data.destiny_memberships.get(0) {
            Some(membership) => Ok(membership.clone()),
            None => bail!("No memberships!"),
          }
        }
        _ => bail!("Not a membership data!"),
      };
      res
    })
  .and_then(|card| urls::get_profile(card.mtype(), card.id()?))
  .and_then(|url|  authd.get(url));

  let work = database.join(profile)
    .and_then(|(db, profile)| {
      match profile.response {
        dtos::BodyResponse::Profile(prof) => {
          match prof.profile_inventory {
            Some(inv) => {
              for item in inv.data.items {
                match print_item(item.item_hash, &db) {
                  Ok(_) => println!(""),
                  Err(e) => println!("{:?}", e)
                }
              };
              Ok(())
            },
            None => bail!("No inventory!")
          }

        },
        _ => bail!("Response not a Profile!")
      }
    });
  core.run(work)?;
  Ok(())
}

fn print_item(item_hash: u32, db: &Connection) -> Result<()> {
  let mut stmt = db.prepare_cached("select json from DestinyInventoryItemDefinition where id = ?1")?;
  println!("Fetching Item {}", item_hash);
  let mut rows =stmt.query(&[&(item_hash as i32)])?;
  while let Some(row) = rows.next() {
    let json: String = row?.get(0);
    let item: dtos::InventoryItemDefinition = serde_json::from_str(&json)?;
    println!("ID: {} Item: {:?}", item_hash, item);
  };
  Ok(())
}

impl AuthGetter {
  fn new( client: Client<HttpsConnector<HttpConnector>, Body>, token: String, app_auth: String,) -> AuthGetter {
    AuthGetter{ client, token, app_auth }
  }

  fn get(&self,
         url: hyper::Uri)
    -> Box<Future<Item = dtos::ResponseBody, Error = Error>> {
      println!("{}", url);
      let mut req = Request::new(hyper::Method::Get, url);
      req.headers_mut().set(headers::XApiKey::key(self.app_auth.clone()));
      req.headers_mut().set(header::Accept::json());
      req.headers_mut().set(header::Authorization(header::Bearer { token: self.token.to_owned() }));
      // println!("Request: {:?}", req);
      let future = self.client.request(req)
        .then(|result| {
          match result {
            Ok(res) => {
              // println!("Response: {:?}", res);
              match res.status() {
                hyper::StatusCode::Ok => Ok(res),
                hyper::StatusCode::Unauthorized => {
                  let mut state = state::load().unwrap();
                  state.access_token = String::new();
                  state.refresh_token = String::new();
                  state::save(state)?;
                  bail!("unauthorized - old token scrubbed, rerun.")
                }
                _ => bail!("Other status..."),
              }
            }
            _ => result.chain_err(|| "network error"),
          }
        })
      .and_then(|res| {
        res.body().concat2().map_err(|e| Error::with_chain(e, "assembling body from stream"))
      })
      .and_then(|body_chunk| {
        println!("{}", String::from_utf8_lossy(&(*body_chunk)));
        let v: dtos::ResponseBody =
          serde_json::from_slice(&body_chunk).chain_err(|| format!("deserializing JSON: {:?}", String::from_utf8_lossy(&(*body_chunk))))?;
        Ok(v)
      });
      Box::new(future)
    }

  fn get_debug(&self,
         url: hyper::Uri)
    -> Box<Future<Item = dtos::DebugResponseBody, Error = Error>> {
      println!("{}", url);
      let mut req = Request::new(hyper::Method::Get, url);
      req.headers_mut().set(headers::XApiKey::key(self.app_auth.clone()));
      req.headers_mut().set(header::Accept::json());
      req.headers_mut().set(header::Authorization(header::Bearer { token: self.token.to_owned() }));
      // println!("Request: {:?}", req);
      let future = self.client.request(req)
        .then(|result| {
          match result {
            Ok(res) => {
              // println!("Response: {:?}", res);
              match res.status() {
                hyper::StatusCode::Ok => Ok(res),
                hyper::StatusCode::Unauthorized => {
                  let mut state = state::load().unwrap();
                  state.access_token = String::new();
                  state.refresh_token = String::new();
                  state::save(state)?;
                  bail!("unauthorized - old token scrubbed, rerun.")
                }
                _ => bail!("Other status..."),
              }
            }
            _ => result.chain_err(|| "network error"),
          }
        })
      .and_then(|res| {
        res.body().concat2().map_err(|e| Error::with_chain(e, "assembling body from stream"))
      })
      .and_then(|body_chunk| {
        println!("{}", String::from_utf8_lossy(&(*body_chunk)));
        let v: dtos::DebugResponseBody =
          serde_json::from_slice(&body_chunk).chain_err(|| format!("deserializing JSON: {:?}", String::from_utf8_lossy(&(*body_chunk))))?;
        Ok(v)
      });
      Box::new(future)
    }

}
