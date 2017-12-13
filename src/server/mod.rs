use errors::*;

use ::state;
use ::destiny;
use mime;

use hyper::server::{Http, Request, Response};
use hyper::{Method, StatusCode};

use gotham::handler::NewHandlerService;
use gotham::router::Router;
use gotham::router::tree::TreeBuilder;
use gotham::router::route::{Extractors, Route, RouteImpl, Delegation};
use gotham::router::route::matcher::MethodOnlyRouteMatcher;
use gotham::router::route::dispatch::{new_pipeline_set, finalize_pipeline_set, PipelineSet,
                                      PipelineHandleChain, DispatcherImpl};
use gotham::router::request::path::NoopPathExtractor;
use gotham::router::request::query_string::NoopQueryStringExtractor;
use gotham::state::State;
use gotham::http::response::create_response;
use gotham::middleware::pipeline::new_pipeline;
use gotham::handler::NewHandler;
use gotham::router::response::finalizer::ResponseFinalizerBuilder;


pub fn start_http() -> Result<()> {
  let addr = "127.0.0.1:7878".parse()?;

  let server = Http::new().bind(&addr, NewHandlerService::new(router()))?;

  println!("Listening on http://{}", server.local_addr()?);
  Ok(server.run()?)
}

fn inventory(gstate: State, _req: Request) -> (State, Response) {
  let res = match inventory_body() {
    Ok(string) => {
      create_response(&gstate,
                      StatusCode::Ok,
                      Some((string.into_bytes(), mime::TEXT_PLAIN)))
    }
    Err(e) => {
      create_response(&gstate,
                      StatusCode::InternalServerError,
                      Some((format!("{}", e).into_bytes(), mime::TEXT_PLAIN)))
    }
  };

  (gstate, res)
}

fn inventory_body() -> Result<String> {
  let state = state::load()?;
  Ok(format!("{}",
             destiny::api_exchange(state.access_token, state.api_key)?))
}

fn router() -> Router {
  let ps_builder = new_pipeline_set();
  let (ps_builder, global) = ps_builder.add(new_pipeline().build());
  let ps = finalize_pipeline_set(ps_builder);

  let mut builder = TreeBuilder::new();
  builder.add_route(static_route(
      vec![Method::Get, Method::Head], // Use this Route for Get and Head Requests
      || Ok(inventory),
      (global, ()), // This signifies that the active Pipelines for this route consist only of the global pipeline
      ps.clone())); // All the pipelines we've created for this Router
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
