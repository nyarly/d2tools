use url;
use hyper;
use uritemplate::UriTemplate;
use errors::*;
use super::dtos::enums;

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

pub fn get_profile(m_type: super::dtos::enums::BungieMemberType,
                   dmid: i64,
                   components: &[enums::ComponentType])
                   -> Result<hyper::Uri> {
  let path =
    UriTemplate::new("./Destiny2/{membershipType}/Profile/{destinyMembershipId}/{?components}")
      .set("membershipType", m_type)
      .set("destinyMembershipId", dmid.to_string())
      .set("components", enums::component_list(components))
      .build();
  build_url(&path)
}

pub fn get_item(m_type: super::dtos::enums::BungieMemberType,
                dmid: &str,
                instance_id: &str,
                components: &[enums::ComponentType])
                -> Result<hyper::Uri> {
  let path =
    UriTemplate::new("./Destiny2/{membershipType}/Profile/{destinyMembershipId}/Item/{itemInstanceId}/{?components}")
      .set("membershipType", m_type)
      .set("destinyMembershipId", dmid)
      .set("itemInstanceId", instance_id)
      .set("components", enums::component_list(components))
      .build();
  build_url(&path)
}
