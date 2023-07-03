use activitypub_federation::{config::Data, traits::ActivityHandler};
use actix_web::ResponseError;
use async_stream::stream;
use async_trait::async_trait;
use futures_core::Stream;
use serde::{Deserialize, Serialize};
use serde_json::{Deserializer, Value, json};
use url::Url;
use std::{path::Path, fmt::Display};
use anyhow::{Result, Context};

pub async fn import_zstd_json_dump<T>(
  source_file: &Path,
) -> Result<impl Stream<Item = Result<T>>>
where
  T: for<'a> Deserialize<'a> + Send + 'static,
{
  let mut comments_stream = zstd::stream::Decoder::new(std::fs::File::open(source_file)?)?;
  comments_stream.window_log_max(31)?;
  let mut de = Deserializer::from_reader(comments_stream).into_iter::<T>();

  Ok(stream! {
    loop {
      let (next, den) = tokio::task::spawn_blocking(|| {
        (de.next(), de)
      }).await.context("spawning error")?;
      de = den;
      let Some(next) = next else {
        return;
      };
      yield next.context("decoding error");
    }
  })
}


/// hard coded "fake" context
pub fn jsonld_context() -> Value {
    json!([
      "https://www.w3.org/ns/activitystreams",
      "https://w3id.org/security/v1",
      {
        "lemmy": "https://join-lemmy.org/ns#",
        "litepub": "http://litepub.social/ns#",
        "pt": "https://joinpeertube.org/ns#",
        "sc": "http://schema.org/",
        "ChatMessage": "litepub:ChatMessage",
        "commentsEnabled": "pt:commentsEnabled",
        "sensitive": "as:sensitive",
        "matrixUserId": "lemmy:matrixUserId",
        "postingRestrictedToMods": "lemmy:postingRestrictedToMods",
        "removeData": "lemmy:removeData",
        "stickied": "lemmy:stickied",
        "moderators": {
          "@type": "@id",
          "@id": "lemmy:moderators"
        },
        "expires": "as:endTime",
        "distinguished": "lemmy:distinguished",
        "language": "sc:inLanguage",
        "identifier": "sc:identifier"
      }
    ])
  }
  




/// Necessary because of this issue: https://github.com/actix/actix-web/issues/1711
#[derive(Debug)]
pub struct DontCareActixError(pub(crate) anyhow::Error);

impl Display for DontCareActixError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    std::fmt::Display::fmt(&self.0, f)
  }
}

impl<T> From<T> for DontCareActixError
where
  T: Into<anyhow::Error>,
{
  fn from(t: T) -> Self {
    DontCareActixError(t.into())
  }
}
impl ResponseError for DontCareActixError {}

#[derive(Serialize, Deserialize, Debug)]
pub struct ToApub {
  #[serde(flatten)]
  pub raw: Value,
  pub id: Url,
  pub actor: Url,
}
#[async_trait]
impl ActivityHandler for ToApub {
  type DataType = ();

  type Error = anyhow::Error;

  fn id(&self) -> &Url {
    &self.id
  }

  fn actor(&self) -> &Url {
    &self.actor
  }

  async fn verify(&self, _data: &Data<Self::DataType>) -> Result<(), Self::Error> {
    todo!()
  }

  async fn receive(self, _data: &Data<Self::DataType>) -> Result<(), Self::Error> {
    todo!()
  }
}