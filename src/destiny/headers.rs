use std::fmt::{self, Display};
use hyper::header;
use hyper;

#[derive(Clone)]
pub struct XApiKey {
  key: String,
}

impl XApiKey {
  pub fn key(k: String) -> XApiKey {
    XApiKey { key: k }
  }
}

impl Display for XApiKey {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.key)
  }
}


impl header::Header for XApiKey {
  // Self = XApiKey
  fn header_name() -> &'static str {
    "X-API-Key"
  }

  fn parse_header(_: &header::Raw) -> hyper::Result<Self> {
    Ok(XApiKey { key: String::new() })
  }

  fn fmt_header(&self, f: &mut header::Formatter) -> fmt::Result {
    f.fmt_line(self)
  }
}
