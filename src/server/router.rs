use gotham::router::Router;
use gotham::router::route::dispatch::{new_pipeline_set, finalize_pipeline_set, PipelineSet,
                                      PipelineHandleChain, DispatcherImpl};
use gotham::middleware::pipeline::new_pipeline;
use gotham::router::tree::TreeBuilder;
use gotham::router::tree::node::{NodeBuilder, SegmentType};

use hyper::Method;
use gotham::router::request::path::NoopPathExtractor;
use gotham::router::request::query_string::NoopQueryStringExtractor;
use gotham::router::response::finalizer::ResponseFinalizerBuilder;
use gotham::handler::NewHandler;
use gotham::router::route::{Extractors, Route, RouteImpl, Delegation};
use gotham::router::route::matcher::MethodOnlyRouteMatcher;
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

  let mut builder = TreeBuilder::new();
  let mut oauth = NodeBuilder::new("oauth", SegmentType::Static);

  builder.add_route(static_route(vec![Method::Get, Method::Head],
                                 || Ok(super::inventory::handler),
                                 normal_pipeline,
                                 ps.clone()));

  oauth.add_route(static_route(vec![Method::Get, Method::Head],
                               || Ok(super::oauth_receiver::handler),
                               bare_pipeline,
                               ps.clone()));

  builder.add_child(oauth);
  let tree = builder.finalize();

  let response_finalizer_builder = ResponseFinalizerBuilder::new();
  let response_finalizer = response_finalizer_builder.finalize();
  Router::new(tree, response_finalizer)
}

fn static_route<NH, P, C>(methods: Vec<Method>,
                          new_handler: NH,
                          active_pipelines: C,
                          ps: PipelineSet<P>)
                          -> Box<Route + Send + Sync>
  where NH: NewHandler + 'static,
        C: PipelineHandleChain<P> + Send + Sync + 'static,
        P: Send + Sync + 'static
{
  let matcher = MethodOnlyRouteMatcher::new(methods);
  let dispatcher = DispatcherImpl::new(new_handler, active_pipelines, ps);
  let extractors: Extractors<NoopPathExtractor, NoopQueryStringExtractor> = Extractors::new();
  let route = RouteImpl::new(matcher,
                             Box::new(dispatcher),
                             extractors,
                             Delegation::Internal);
  Box::new(route)
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
