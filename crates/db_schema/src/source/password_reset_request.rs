use crate::newtypes::LocalUserId;
#[cfg(feature = "full")]
use crate::schema::password_reset_request;
use chrono::{DateTime, Utc};

#[derive(PartialEq, Eq, Debug)]
#[cfg_attr(feature = "full", derive(Queryable, Identifiable))]
#[cfg_attr(feature = "full", diesel(table_name = password_reset_request))]
pub struct PasswordResetRequest {
  pub id: i32,
  pub token_encrypted: String,
  pub published: DateTime<Utc>,
  pub local_user_id: LocalUserId,
}

#[cfg_attr(feature = "full", derive(Insertable, AsChangeset))]
#[cfg_attr(feature = "full", diesel(table_name = password_reset_request))]
pub struct PasswordResetRequestForm {
  pub local_user_id: LocalUserId,
  pub token_encrypted: String,
}
