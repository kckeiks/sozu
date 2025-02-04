#[macro_export]
macro_rules! take_while_complete (
  ($input:expr, $submac:ident!( $($args:tt)* )) => ({
    use nom::InputTakeAtPosition;
    use nom::Err;

    let input = $input;
    match input.split_at_position(|c| !$submac!(c, $($args)*)) {
      Err(Err::Incomplete(_)) => Ok((&input[input.len()..], input)),
      res => res,
    }
  });
  ($input:expr, $f:expr) => (
    take_while_complete!($input, call!($f))
  );
);

#[macro_export]
macro_rules! take_while1_complete (
  ($input:expr, $submac:ident!( $($args:tt)* )) => ({
    use nom::InputTakeAtPosition;
    use nom::Err;
    use nom::error::ErrorKind;

    let input = $input;
    match input.split_at_position1(|c| !$submac!(c, $($args)*), ErrorKind::TakeWhile1) {
      Err(Err::Incomplete(_)) => Ok((&input[input.len()..], input)),
      res => res,
    }
  });
  ($input:expr, $f:expr) => (
    take_while1_complete!($input, call!($f));
  );
);

#[macro_export]
macro_rules! empty (
  ($i:expr,) => (
    {
      use std::result::Result::*;
      use nom::{Err,error::ErrorKind};
      use nom::InputLength;

      if ($i).input_len() == 0 {
        Ok(($i, $i))
      } else {
        Err(Err::Error(error_position!($i, ErrorKind::Eof)))
      }
    }
  );
);

pub mod h2;
pub mod http;
pub mod pipe;
pub mod proxy_protocol;
pub mod rustls;

use std::{cell::RefCell, rc::Rc};

use mio::Token;
use sozu_command::ready::Ready;

use crate::{
    L7Proxy, ProxySession, SessionIsToBeClosed, SessionMetrics, SessionResult, StateResult,
};

pub use self::{
    http::{Http, StickySession},
    pipe::Pipe,
    proxy_protocol::send::SendProxyProtocol,
    rustls::TlsHandshake,
};

/// All States should satisfy this trait in order to receive and handle Session events
pub trait SessionState {
    /// if a session received an event or can still execute, the event loop will
    /// call this method. Its result indicates if it can still execute or if the
    /// session can be closed
    fn ready(
        &mut self,
        session: Rc<RefCell<dyn ProxySession>>,
        proxy: Rc<RefCell<dyn L7Proxy>>,
        metrics: &mut SessionMetrics,
    ) -> SessionResult;
    /// if the event loop got an event for a token associated with the session,
    /// it will call this method
    fn update_readiness(&mut self, token: Token, events: Ready);
    /// closes the state
    fn close(&mut self, _proxy: Rc<RefCell<dyn L7Proxy>>, _metrics: &mut SessionMetrics) {}
    /// if a timeout associated with the session triggers, the event loop will
    /// call this method with the timeout's token
    fn timeout(&mut self, token: Token, metrics: &mut SessionMetrics) -> StateResult;
    /// cancel frontend timeout (and backend timeout if present)
    fn cancel_timeouts(&mut self);
    /// displays the session's internal state (for debugging purpose),
    /// ```plain
    /// <context> Session(<State name>):
    ///     Frontend:
    ///         - Token(...) Readiness(...)
    ///     Backends:
    ///         - Token(...) Readiness(...)
    ///         - Token(...) Readiness(...)
    /// ```
    fn print_state(&self, context: &str);
    /// tell the session it has to shut down if possible
    ///
    /// if the session handles HTTP requests, it will not close until the response
    /// is completely sent back to the client
    fn shutting_down(&mut self) -> SessionIsToBeClosed {
        true
    }
}
