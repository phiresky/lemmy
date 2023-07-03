use activitypub_federation::{
  activity_queue::send_activity,
  config::{Data, FederationConfig},
  traits::{Actor, Object},
};
use actix_web::{get, http::header::ContentType, web, App, HttpResponse, HttpServer, Responder};
use anyhow::Result;
use async_trait::async_trait;
use clap::{arg, Parser};
use diesel::QueryableByName;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use integration_testing::{import_zstd_json_dump, jsonld_context, DontCareActixError, ToApub};
use serde::Serialize;
use serde_json::json;
use std::{fs::File, path::Path, pin, str::FromStr, time::Instant};
use tokio_stream::StreamExt;
use url::Url;
#[derive(Debug, Parser, Serialize)]
struct UploadOptions {
  /// at which host and port we listen
  #[arg(default_value_t=Url::parse("http://reddit.com.localhost:5313/").unwrap())]
  local_server: Url,
  /// at which host and port we send federation events to
  #[arg(long)]
  remote_server: Url,
  /// the jsonl file to upload
  #[arg(long)]
  input_file: String,
  /// how many outgoing federation workers to use
  #[arg(default_value_t = 100, long)]
  federation_workers: usize,
  /// where to output the info json
  #[arg(long)]
  output_json: String,
  /// name of this run
  #[arg(long)]
  runname: String,
}

#[derive(Debug)]
struct RedditActor {
  server_url: Url,
}
#[async_trait]
impl Object for RedditActor {
  type DataType = ();

  type Kind = ();

  type Error = anyhow::Error;

  async fn read_from_id(
    _object_id: Url,
    _data: &Data<Self::DataType>,
  ) -> Result<Option<Self>, Self::Error> {
    todo!();
  }

  async fn into_json(self, _data: &Data<Self::DataType>) -> Result<Self::Kind, Self::Error> {
    todo!();
  }

  async fn verify(
    _json: &Self::Kind,
    _expected_domain: &Url,
    _data: &Data<Self::DataType>,
  ) -> Result<(), Self::Error> {
    todo!()
  }
  async fn from_json(_json: Self::Kind, _data: &Data<Self::DataType>) -> Result<Self, Self::Error> {
    todo!()
  }
}
impl Actor for RedditActor {
  fn id(&self) -> Url {
    self.server_url.clone()
  }
  fn public_key_pem(&self) -> &str {
    "-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAuU/rSjt1HevYTcHaw4qI\nIzfpWEoJfE9j5iaftUrZKNW5D8Lh++IPbXKQmP8WxWL3jwWPYWWMkwzgYxTCXbl9\n7LredZ+uXdy4I4nr+HEp6nrt/5fU6qz2eOZlALLiaZnDXcB1UtbBNg5aH3uvM3Si\nl7AnwsbXvwx9yg32GoQ06xjMp152wcO3OcIghLbZM4pcGLhRltzuiu/h2u+t7pyi\nbmFW646veyHt+tuUZ6rxfoOyWx+rOekrAIHM6oMbp/D7QGovlejb68labStYckor\nXBcCk8ZlXX2hlRGvOe3RzMAqKDgqxDWZ/drxUEq7YkHU6Lw5L+cuuc+d5tjDdug2\nHwIDAQAB\n-----END PUBLIC KEY-----\n"
  }
  fn private_key_pem(&self) -> Option<String> {
    Some(String::from("-----BEGIN PRIVATE KEY-----\nMIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQC5T+tKO3Ud69hN\nwdrDiogjN+lYSgl8T2PmJp+1Stko1bkPwuH74g9tcpCY/xbFYvePBY9hZYyTDOBj\nFMJduX3sut51n65d3Lgjiev4cSnqeu3/l9TqrPZ45mUAsuJpmcNdwHVS1sE2Dlof\ne68zdKKXsCfCxte/DH3KDfYahDTrGMynXnbBw7c5wiCEttkzilwYuFGW3O6K7+Ha\n763unKJuYVbrjq97Ie3625RnqvF+g7JbH6s56SsAgczqgxun8PtAai+V6NvryVpt\nK1hySitcFwKTxmVdfaGVEa857dHMwCooOCrENZn92vFQSrtiQdTovDkv5y65z53m\n2MN26DYfAgMBAAECggEAHkbH7v9q4aIcW4vuJZ+XIYXrioDCLvy7mik6U8DwXQMa\nMtCI4oHrOlsK8+xNeJ90SfpDFEsmngnvCVEldnGteMWJPheCQhSjQy8wDg3TJtvB\n0c4pO9RZiqQ94VDYvB8is8kTgh7TP3U11Un8dIA8ZmMiA+k/65drX91LFcb+7F//\niMqGblqIV6PAzMp1Jr2fDspcFtVHOajt+bte5lMfndxeTswEI0kDBUQWTkvuzpYQ\nIcXTg2sZoV3btdK7Lxs/8Il69w4BxrTqhu/i+F0YOzpFbU9lW9leQ7EdjOlVuaXe\nx8ChvYU2qS5GG4Rrw6tnHImmlGdkQWXuS1NjJQ/LdQKBgQD6KVoGr3JyG0RtcO8Q\nhpWIufrSWn7xCOFqSjchswSucoY7gVS15tlmSa32Eun2CCxLftmLZ7LYJjpN472Z\n8CAwws/dgU7j0LLkhqSHLu99qEJeHHfCDKmR4Tggbkag6cpChXp7rlgtGH9ENQVc\nUTj9JfNWPz86t2KD22OVGIjp/QKBgQC9oxwjQatH3U1hxoOWGrzQG9KHA26qP40n\noqr6AP5LcP7uDn5tK2SsypovZgWiC0tYwiSZCADpM69vu46Ni25b0spmiIjIc3wX\nAuJXXyiBGv7ikP02roUvP5LnB3NwE7tIcC03llOHZqgd3Sf1CtB4zD8HBEL+7cC6\nnrGZOpIdSwKBgQC45Y1zuYN2US8PUNRxu3eUmhmIFnkSwESTog0DrGQ+Z8lM+/dX\nhyuSDc01Pp+MSFgs6LHz9o5ack7PuQ8vUysHv0WR63wap+tBOz8p54f9sTp0gsgF\nNgSzHOq2FavATWxAJJX2ClOD6UJPcHzo0eO0P7OOQKsEQ/zdhm8hCQRRJQKBgBNO\nV77/IIDgdtBNdXgCoNZO/s/f+ZQ7hBNU7DMnhrwHdOynbReQI1+0AJ5ytIAaxkDz\nAubRecZEDMhDP/AJEeMnQpPNsp81opx1HrXmaik6plhKinzWp5h30GzUxVvTpm1p\nfjD6jOZr/RGNQlQgFbk2kfQU6v0pF0XoggwnelihAoGBAOnc8Zs/tOu+Wz6CQk2z\nTo8Rce4Ur92cQVMT4xD2eVZS3owVGQw5JBxMeXl6XfmDPtJtOSzA4uePyhN6FW94\nLMaAZ/qOxQUbdh2/vadCjutAeQWL2IoGyOL/X31Ez6y9Fujt4F0jPgw2HAFD0pFK\no0TdM2sK20FaQDWzXZ3Pt4Ia\n-----END PRIVATE KEY-----\n"))
  }
  fn inbox(&self) -> Url {
    todo!();
  }
}

struct RData {
  server_url: Url,
}
impl RData {
  pub fn permalink(&self, p: impl AsRef<str>) -> Result<Url> {
    Ok(self.server_url.join(p.as_ref())?)
  }
}
#[get("/r/{name}")]
async fn http_get_community(
  rdata: web::Data<RData>,
  name: web::Path<String>,
) -> std::result::Result<impl Responder, DontCareActixError> {
  let url = rdata.permalink(format!("/r/{name}"))?;
  Ok(
      HttpResponse::Ok()
        .content_type(ContentType(
          mime::Mime::from_str("application/activity+json").unwrap(),
        ))
        .json(json!({
          "@context": jsonld_context(),
          "type": "Group",
          "id": url,
          "preferredUsername": name.to_string(),
          "inbox": rdata.permalink(format!("/r/{name}/inbox"))?,
          "followers":rdata.permalink(format!("/r/{name}/followers"))?,
          "publicKey": { // don't care
            "id": "https://lemmy.world/c/syncforlemmy#main-key",
            "owner": "https://lemmy.world/c/syncforlemmy",
            "publicKeyPem": "-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEApjSiWhzx6mY7s4qjfjU7\nRq72lFZ0Mcci4B9pv152/Ikihwt99lmwgy2bb1lPTBWhh4RLsa6qT8ilhgVMBE8f\n74pKo7MT+ZXmsQz4miFG9NoDIMFPhoUadUnISVxLtHWYx1bLvt8mpFinKJFT297T\npFgueBAcrgnu407mC/6XCaf/pKWwYzGOMqrOLxuviQ4s+vrPn73kaIRi177YRJ9Q\nnIjV5r2pFQavH0flQdQzLa/1O5paBOJPise8GzItiia6RJ2MSSN9R1R0efefN90E\ntkAJpVEOwaICST6gkMCImY09nqHrLkBlmEciGShBvqpk1vVIoXhY9P8qi0CYVzoB\n5wIDAQAB\n-----END PUBLIC KEY-----\n"
          },
          "name": name.to_string(),
          "summary": "loaded from redidt dump",
          "sensitive": false,
          // "moderators": "https://lemmy.world/c/syncforlemmy/moderators",
          // "attributedTo": "https://lemmy.world/c/syncforlemmy/moderators",
          "postingRestrictedToMods": false,
          "outbox": rdata.permalink(format!("/r/{name}/outbox"))?,
          /*"endpoints": {
            "sharedInbox": "https://lemmy.world/inbox"
          },*/
        })),
    )
}
#[get("/u/{name}")]
async fn http_get_user(
  rdata: web::Data<RData>,
  name: web::Path<String>,
) -> std::result::Result<impl Responder, DontCareActixError> {
  Ok(
    HttpResponse::Ok()
      .content_type(ContentType(
        mime::Mime::from_str("application/activity+json").unwrap(),
      ))
      .json(json!({
        "id": rdata.permalink(format!("/u/{name}"))?,
        "type": "Person",
        "preferredUsername": name.to_string(),
        "name": name.to_string(),
        // "summary": "<p>Captain of the starship <strong>Enterprise</strong>.</p>\n",
        /*"source": {
          "content": "Captain of the starship **Enterprise**.",
          "mediaType": "text/markdown"
        },*/
        /*"icon": {
          "type": "Image",
          "url": "https://enterprise.lemmy.ml/pictrs/image/ed9ej7.jpg"
        },
        "image": {
          "type": "Image",
          "url": "https://enterprise.lemmy.ml/pictrs/image/XenaYI5hTn.png"
        },*/
        // "matrixUserId": "@picard:matrix.org",
        "inbox": rdata.permalink(format!("/u/{name}/inbox"))?,
        "outbox": rdata.permalink(format!("/u/{name}/outbox"))?,

        /* "endpoints": {
          "sharedInbox": "https://enterprise.lemmy.ml/inbox"
        },*/
        // "published": "2020-01-17T01:38:22.348392+00:00",
        // "updated": "2021-08-13T00:11:15.941990+00:00",
        "publicKey": {
          "id": rdata.server_url,
          "owner": rdata.server_url,
          "publicKeyPem": RedditActor {server_url: rdata.server_url.clone() }.public_key_pem()
        }
      })),
  )
}

#[tokio::main]
async fn main() -> Result<()> {
  let opt = UploadOptions::parse();
  let rdata = RData {
    server_url: opt.local_server.clone(),
  };
  let reddit_actor = RedditActor {
    server_url: rdata.server_url,
  };
  let fed = FederationConfig::builder()
    .domain("reddit.com.local")
    .debug(false)
    .allow_http_urls(true)
    .app_data(())
    .worker_count(opt.federation_workers)
    .build()
    .await?;
  let server = HttpServer::new(|| {
    App::new()
      .service(http_get_user)
      .service(http_get_community)
  })
  .bind((
    "127.0.0.1",
    opt.local_server.port_or_known_default().unwrap(),
  ))?
  .disable_signals()
  .run();
  // server.await;

  tokio::task::spawn(server);
  let data = fed.to_request_data();
  let inboxes = vec![opt.remote_server.clone()];
  let mut stream = pin::pin!(import_zstd_json_dump::<ToApub>(Path::new(&opt.input_file)).await?);
  let start = Instant::now();
  let mut total_send_count = 0;
  while let Some(ele_apub) = stream.next().await {
    let ele_apub = ele_apub?;
    send_activity(ele_apub, &reddit_actor, inboxes.clone(), &data).await?;
    total_send_count += 1;
  }
  let unqueue_time = start.elapsed();
  tracing::warn!("sending {total_send_count} took {unqueue_time:.2?}");
  drop(data);
  let start = Instant::now();
  let activityqueue_stats = format!("{:?}", fed.shutdown(false).await?);
  let clear_time = start.elapsed();
  tracing::warn!("stats: {activityqueue_stats}");
  tracing::warn!("clearing queue took {clear_time:.2?}");

  let db_stats = {
    use diesel::dsl::sql_query;
    use lemmy_db_schema::utils::get_database_url;
    let settings = lemmy_utils::settings::SETTINGS.to_owned();

    // Run the DB migrations
    let db_url = get_database_url(Some(&settings));
    // run_migrations(&db_url);

    // Set up the connection pool
    //let pool = build_db_pool(&settings).await?;
    let mut conn = AsyncPgConnection::establish(&db_url).await?;
    sql_query("create extension if not exists pg_stat_statements")
      .execute(&mut conn)
      .await?;
    let st: DbStat = sql_query("select * from
    (select count(*) as comment_count from comment) a,
    (select count(*) as post_count from post) b,
    (select count(*) as activity_count from activity) d,
    (select count(*) as statement_count, sum(calls)::bigint as statement_call_count, 
        sum(total_plan_time)/1000 as total_plan_time_s, sum(total_exec_time)/1000 as total_exec_time_s from pg_stat_statements where query != 'SELECT $1') c,
    (select json_agg(row_to_json(top_queries)) as top_queries_by_call_count
      from (select query, calls, total_exec_time, mean_exec_time, rows  from pg_stat_statements order by calls desc limit 20) top_queries) t,
    (select json_agg(row_to_json(top_queries)) as top_queries_by_mean_time
      from (select query, calls, total_exec_time, mean_exec_time, rows  from pg_stat_statements order by mean_exec_time desc limit 20) top_queries) t2,
    (select json_agg(row_to_json(top_queries)) as top_queries_by_total_time
      from (select query, calls, total_exec_time, mean_exec_time, rows  from pg_stat_statements order by total_exec_time desc limit 20) top_queries) t3
      
      ;
    ").get_result(&mut conn).await?;
    st
  };

  serde_json::to_writer_pretty(
    File::create(&opt.output_json)?,
    &Output {
      unqueue_time_s: unqueue_time.as_secs_f64(),
      total_time_s: (unqueue_time + clear_time).as_secs_f64(),
      clear_time_s: clear_time.as_secs_f64(),
      activityqueue_stats,
      db_stats,
      config: opt,
    },
  )?;
  Ok(())
}

#[derive(Serialize, Debug, QueryableByName, PartialEq)]
struct DbStat {
  #[diesel(sql_type=diesel::sql_types::BigInt)]
  comment_count: i64,
  #[diesel(sql_type=diesel::sql_types::BigInt)]
  post_count: i64,
  #[diesel(sql_type=diesel::sql_types::BigInt)]
  activity_count: i64,
  #[diesel(sql_type=diesel::sql_types::BigInt)]
  statement_count: i64,
  #[diesel(sql_type=diesel::sql_types::BigInt)]
  statement_call_count: i64,
  #[diesel(sql_type=diesel::sql_types::Double)]
  total_plan_time_s: f64,
  #[diesel(sql_type=diesel::sql_types::Double)]
  total_exec_time_s: f64,
  #[diesel(sql_type=diesel::sql_types::Json)]
  top_queries_by_call_count: serde_json::Value,
  #[diesel(sql_type=diesel::sql_types::Json)]
  top_queries_by_mean_time: serde_json::Value,
  #[diesel(sql_type=diesel::sql_types::Json)]
  top_queries_by_total_time: serde_json::Value,
}

#[derive(Serialize)]
struct Output {
  total_time_s: f64,
  unqueue_time_s: f64,
  clear_time_s: f64,
  activityqueue_stats: String,
  db_stats: DbStat,
  config: UploadOptions,
}
