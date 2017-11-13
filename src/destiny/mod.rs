use errors::*;
use futures::Stream;
use futures::future::Future;
use hyper;
use hyper::Client;
use hyper_tls::HttpsConnector;
use tokio_core::reactor::Core;
use serde_json;

mod urls;
mod headers;
mod dtos;

use super::state;

pub fn api_exchange(token: String, app_auth: String) -> Result<()> {
  let mut core = Core::new()?;
  let handle = core.handle();
  let client = Client::configure()
    .connector(HttpsConnector::new(4, &handle)?)
    .build(&handle);
  let mut req = hyper::client::Request::new(hyper::Method::Get,
                                            urls::get_membership_data_for_current_user()?);
  req.headers_mut().set(headers::XApiKey::key(app_auth));
  req.headers_mut().set(hyper::header::Accept::json());
  req.headers_mut()
    .set(hyper::header::Authorization(hyper::header::Bearer { token: token.to_owned() }));


  // println!("Request: {:?}", req);
  let work = client.request(req)
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
              bail!("unauthorized")
            }
            _ => bail!("Other status..."),
          }
        }
        _ => result.chain_err(|| "network error"),
      }
    })
    .and_then(|res| {
      res.body()
        .concat2()
        .then(|result| {
          match result {
            Ok(body) => {
              println!("Body: {:?}", String::from_utf8_lossy(&body));
              let v: dtos::ResponseBody =
                serde_json::from_slice(&body).chain_err(|| "deserializing JSON")?;
              println!(" -> {:?}", v);
              Ok(())
            }
            _ => result.map(|_| ()).chain_err(|| "body streaming"),
          }
        })
    });
  Ok(core.run(work)?)
}
