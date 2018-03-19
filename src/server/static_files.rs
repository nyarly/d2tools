use tokio_core::reactor::Remote;
use tokio_service::Service;
use hyper_staticfile::Static;
use gotham::handler::{NewHandler,Handler,HandlerFuture};
use gotham::state::State;
use futures::Future;

struct NewStaticFile {
  dir: String,
  handle: Remote,
}

impl NewStaticFile {
  fn new(dir: String, handle: Remote) -> NewStaticFile {
    NewStaticFile{ dir, handle }
  }
}

// ???
impl ::std::panic::RefUnwindSafe for NewStaticFile{}

impl NewHandler for NewStaticFile {
  type Instance = StaticFile;

  fn new_handler(&self) -> Result<Self::Instance, ::std::io::Error> {
    Ok(StaticFile::new(self.handle, self.dir))
  }
}

struct StaticFile {
  dir: String,
  handle: Remote,
}

impl StaticFile {
  fn new(handle: Remote, root: String) -> StaticFile {
    StaticFile{
      dir: root,
      handle: handle
    }
  }
}


impl Handler for StaticFile {
  fn handle(self, state: State) -> Box<HandlerFuture> {
    let inner = Static::new(&self.handle.handle()?, self.dir);
    let request = state.borrow();
    inner.call(request)
      .map(|res| (state, res))
      .map_err(|e| (state, e))
  }
}
