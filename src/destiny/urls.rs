use url;
use hyper;
use uritemplate::UriTemplate;
use errors::*;

fn root() -> Result<url::Url> {
  Ok("https://www.bungie.net/Platform/".parse()?)
}

fn build_url(path: &str) -> Result<hyper::Uri> {
  let url = root()?.join(path)?;
  Ok(url.as_str().parse()?)
}

pub fn get_manifest() -> Result<hyper::Uri> {
  build_url("./Destiny2/Manifest/")
}

pub fn get_membership_data_for_current_user() -> Result<hyper::Uri> {
  build_url("./User/GetMembershipsForCurrentUser/")
}

pub fn get_profile(m_type: super::dtos::BungieMemberType, dmid: i64) -> Result<hyper::Uri> {
  let path =
    UriTemplate::new("./Destiny2/{membershipType}/Profile/{destinyMembershipId}/?components=100,\
                      101,102,103,200,201,202,204,205,300,301,302,304,305,306,307,308,400,401,\
                      402,500")
      .set("membershipType", m_type)
      .set("destinyMembershipId", format!("{}", dmid))
      .build();
  build_url(&path)
}
