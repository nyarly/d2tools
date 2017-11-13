error_chain!{
  foreign_links {
    TLS(::native_tls::Error);
    AddrParseError(::std::net::AddrParseError);
    EnvVarError(::std::env::VarError);
    IOError(::std::io::Error);
    DeError(::toml::de::Error);
    SerError(::toml::ser::Error);
    Oauth2Error(::oauth2::TokenError);
    UrlParseError(::url::ParseError);
    HyperParseError(::hyper::error::UriError);
    HyperError(::hyper::Error);
    SerdeJSON(::serde_json::Error);
  }
}
