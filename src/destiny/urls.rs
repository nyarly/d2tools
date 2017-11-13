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

pub fn get_membership_data_for_current_user() -> Result<hyper::Uri> {
  build_url("./User/GetMembershipsForCurrentUser/")
}

fn get_profile(m_type: super::dtos::BungieMemberType, dmid: i64) -> Result<hyper::Uri> {
  let path = UriTemplate::new("./Destiny2/{membershipType}/Profile/{destinyMembershipId}/")
    .set("membershipType", m_type)
    .set("destinyMembershipId", format!("{}", dmid))
    .build();
  build_url(&path)
}
