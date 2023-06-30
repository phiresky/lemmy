use std::{
  fmt,
  fmt::{Debug, Display}, any::Any,
};
use tracing_error::SpanTrace;

#[derive(serde::Serialize)]
struct ApiError {
  error: String,
}

pub type LemmyResult<T> = Result<T, LemmyError>;

pub struct LemmyError {
  pub message: Option<String>,
  pub inner: anyhow::Error,
  pub context: SpanTrace,
}

impl LemmyError {
  /// Create LemmyError from a message, including stack trace
  pub fn from_message(message: &str) -> Self {
    let inner = anyhow::anyhow!("{}", message);
    LemmyError {
      message: Some(message.into()),
      inner,
      context: SpanTrace::capture(),
    }
  }

  /// Create a LemmyError from error and message, including stack trace
  pub fn from_error_message<E>(error: E, message: &str) -> Self
  where
    E: Into<anyhow::Error>,
  {
    LemmyError {
      message: Some(message.into()),
      inner: error.into(),
      context: SpanTrace::capture(),
    }
  }

  /// Add message to existing LemmyError (or overwrite existing error)
  pub fn with_message(self, message: &str) -> Self {
    LemmyError {
      message: Some(message.into()),
      ..self
    }
  }

  pub fn to_json(&self) -> Result<String, Self> {
    let api_error = match &self.message {
      Some(error) => ApiError {
        error: error.into(),
      },
      None => ApiError {
        error: "Unknown".into(),
      },
    };

    Ok(serde_json::to_string(&api_error)?)
  }
}

impl<T> From<T> for LemmyError
where
  T: Into<anyhow::Error>,
{
  fn from(t: T) -> Self {
    
    LemmyError {
      message: None,
      inner: t.into(),
      context: SpanTrace::capture(),
    }
  }
}

impl Debug for LemmyError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("LemmyError")
      .field("message", &self.message)
      .field("inner", &self.inner)
      .field("context", &self.context)
      .finish()
  }
}

impl Display for LemmyError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    if let Some(message) = &self.message {
      write!(f, "{message}: ")?;
    }
    // print anyhow including trace
    // https://docs.rs/anyhow/latest/anyhow/struct.Error.html#display-representations
    // this will print the anyhow trace (only if it exists)
    // and if RUST_BACKTRACE=1, also a full backtrace
    writeln!(f, "{:?}", self.inner)?;
    fmt::Display::fmt(&self.context, f)
  }
}

impl actix_web::error::ResponseError for LemmyError {
  fn status_code(&self) -> http::StatusCode {
    match self.inner.downcast_ref::<diesel::result::Error>() {
      Some(diesel::result::Error::NotFound) => http::StatusCode::NOT_FOUND,
      _ => http::StatusCode::BAD_REQUEST,
    }
  }

  fn error_response(&self) -> actix_web::HttpResponse {
    if let Some(message) = &self.message {
      actix_web::HttpResponse::build(self.status_code()).json(ApiError {
        error: format!("innerr: {:?}", self.inner),
      })
    } else {
      actix_web::HttpResponse::build(self.status_code())
        .content_type("text/plain")
        .body(format!("inen: {:?}", self.inner.to_string()))
    }
  }
}

/// this has the same function as anyhow::Context except for LemmyErrors (keeping the spantrace and message)
pub trait LemmyErrContext<T> {
  fn context<C>(self, context: C) -> LemmyResult<T>
  where
    C: Display + Send + Sync + 'static;

  fn with_context<C, F>(self, f: F) -> LemmyResult<T>
  where
    C: Display + Send + Sync + 'static,
    F: FnOnce() -> C;
}
/*impl<T, R> LemmyErrContext<R> for T where T: Into<LemmyError> {

}*/
impl<T> LemmyErrContext<T> for Result<T, LemmyError>
{
  fn context<C>(self, context: C) -> LemmyResult<T>
  where
    C: Display + Send + Sync + 'static,
  {
    self.map_err(|e| {
      LemmyError {
        inner: e.inner.context(context),
        context: e.context,
        message: e.message
      }
    })
  }

  fn with_context<C, F>(self, f: F) -> LemmyResult<T>
  where
    C: Display + Send + Sync + 'static,
    F: FnOnce() -> C,
  {
    self.map_err(|e| {
      LemmyError {
        inner: e.inner.context(f()),
        ..e
      }
    })
  }
}
