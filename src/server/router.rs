use gotham::router::Router;
use gotham::router::builder::build_router;
use gotham::router::builder::DrawRoutes;
use gotham::router::builder::DefineSingleRoute;
use gotham::pipeline::set::{finalize_pipeline_set, new_pipeline_set};
use gotham::pipeline::new_pipeline;

use gotham::middleware::session::{NewSessionMiddleware, MemoryBackend};

pub fn new() -> Router {
  let ps_builder = new_pipeline_set();
  let (ps_builder, global) = ps_builder.add(new_pipeline()
    .add(session_middleware())
    .add(oauth_config_middleware())
    .build());
  let (ps_builder, req_authn) = ps_builder.add(new_pipeline()
    .add(require_auth_middleware())
    .build());
  let ps = finalize_pipeline_set(ps_builder);

  let bare_pipeline = (global, ());
  let normal_pipeline = (req_authn, (global, ()));

  build_router(normal_pipeline, ps, |route| {
    route.get_or_head("/").to(super::inventory::handler);
    route.with_pipeline_chain(bare_pipeline, |auth| {
      auth.get_or_head("/oauth").to(super::oauth_receiver::handler);
    });
  })
}

fn session_middleware() -> NewSessionMiddleware<MemoryBackend, super::D2Session> {
  NewSessionMiddleware::default().with_session_type::<super::D2Session>()
}

fn oauth_config_middleware() -> super::app_config::New {
  super::app_config::New {}
}

fn require_auth_middleware() -> super::require_authn::New {
  super::require_authn::New {}
}
