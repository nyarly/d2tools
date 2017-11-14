use errors::*;
use futures::Stream;
use futures::future::Future;
use hyper::{self, header, Body};
use hyper::client::{Client, Request, HttpConnector};
use hyper_tls::HttpsConnector;
use tokio_core::reactor::Core;
use serde_json;

mod urls;
mod headers;
mod dtos;

use super::state;

struct AuthGetter {
  client: Client<HttpsConnector<HttpConnector>, Body>,
  token: String,
  app_auth: String,
}

use std::borrow::Borrow;

pub fn api_exchange(token: String, app_auth: String) -> Result<()> {
  let mut core = Core::new()?;
  let handle = core.handle();
  let client = Client::configure()
    .connector(HttpsConnector::new(4, &handle)?)
    .build(&handle);

  let authd = AuthGetter::new(client, token, app_auth);

  let work = authd.get(urls::get_membership_data_for_current_user()?)
    .and_then(|rb| {
      let res: Result<_> = match rb.response {
        dtos::BodyResponse::UserMembershipData(data) => {
          match data.destiny_memberships.get(0) {
            Some(membership) => Ok(membership),
            None => bail!("No memberships!"),
          }
        }
        _ => bail!("Not a membership data!"),
      };
      res
    })
  .and_then(|card| {
    let url = urls::get_profile(card.mtype(), card.id()?)?;
    authd.get(url)
  })
  .and_then(|resp| Ok(println!("{:?}", resp)));
  Ok(core.run(work)?)
}

impl AuthGetter {
  fn new( client: Client<HttpsConnector<HttpConnector>, Body>, token: String, app_auth: String,) -> AuthGetter {
    AuthGetter{ client, token, app_auth }
  }

  fn get(&self,
         url: hyper::Uri)
    -> Box<Future<Item = dtos::ResponseBody, Error = Error>> {
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
        let v: dtos::ResponseBody =
          serde_json::from_slice(&body_chunk).chain_err(|| "deserializing JSON")?;
        Ok(v)
      });
      Box::new(future)
    }

}
