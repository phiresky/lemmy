set timezone ='UTC';
--  Allow ALTER TABLE ... SET DATA TYPE changing between timestamp and timestamptz to avoid a table rewrite when the session time zone is UTC (Noah Misch)
-- In the UTC time zone, these two data types are binary compatible.

alter table community_moderator alter column published type timestamptz using published;
alter table community_follower alter column published type timestamptz using published;
alter table person_ban alter column published type timestamptz using published;
alter table community_person_ban alter column published type timestamptz using published;
alter table community_person_ban alter column expires type timestamptz using expires;
alter table person alter column published type timestamptz using published;
alter table person alter column updated type timestamptz using updated;
alter table person alter column last_refreshed_at type timestamptz using last_refreshed_at;
alter table person alter column ban_expires type timestamptz using ban_expires;
alter table post_like alter column published type timestamptz using published;
alter table post_saved alter column published type timestamptz using published;
alter table post_read alter column published type timestamptz using published;
alter table comment_like alter column published type timestamptz using published;
alter table comment_saved alter column published type timestamptz using published;
alter table comment alter column published type timestamptz using published;
alter table comment alter column updated type timestamptz using updated;
alter table mod_remove_post alter column when_ type timestamptz using when_;
alter table mod_lock_post alter column when_ type timestamptz using when_;
alter table mod_remove_comment alter column when_ type timestamptz using when_;
alter table mod_remove_community alter column expires type timestamptz using expires;
alter table mod_remove_community alter column when_ type timestamptz using when_;
alter table mod_ban_from_community alter column expires type timestamptz using expires;
alter table mod_ban_from_community alter column when_ type timestamptz using when_;
alter table mod_ban alter column expires type timestamptz using expires;
alter table mod_ban alter column when_ type timestamptz using when_;
alter table mod_add_community alter column when_ type timestamptz using when_;
alter table mod_add alter column when_ type timestamptz using when_;
alter table person_mention alter column published type timestamptz using published;
alter table mod_feature_post alter column when_ type timestamptz using when_;
alter table password_reset_request alter column published type timestamptz using published;
alter table private_message alter column published type timestamptz using published;
alter table private_message alter column updated type timestamptz using updated;
alter table activity alter column published type timestamptz using published;
alter table activity alter column updated type timestamptz using updated;
alter table community alter column published type timestamptz using published;
alter table community alter column updated type timestamptz using updated;
alter table community alter column last_refreshed_at type timestamptz using last_refreshed_at;
alter table post alter column published type timestamptz using published;
alter table post alter column updated type timestamptz using updated;
alter table comment_report alter column published type timestamptz using published;
alter table comment_report alter column updated type timestamptz using updated;
alter table post_report alter column published type timestamptz using published;
alter table post_report alter column updated type timestamptz using updated;
alter table post_aggregates alter column published type timestamptz using published;
alter table post_aggregates alter column newest_comment_time_necro type timestamptz using newest_comment_time_necro;
alter table post_aggregates alter column newest_comment_time type timestamptz using newest_comment_time;
alter table comment_aggregates alter column published type timestamptz using published;
alter table community_block alter column published type timestamptz using published;
alter table community_aggregates alter column published type timestamptz using published;
alter table mod_transfer_community alter column when_ type timestamptz using when_;
alter table person_block alter column published type timestamptz using published;
alter table local_user alter column validator_time type timestamptz using validator_time;
alter table admin_purge_person alter column when_ type timestamptz using when_;
alter table email_verification alter column published type timestamptz using published;
alter table admin_purge_community alter column when_ type timestamptz using when_;
alter table admin_purge_post alter column when_ type timestamptz using when_;
alter table admin_purge_comment alter column when_ type timestamptz using when_;
alter table registration_application alter column published type timestamptz using published;
alter table mod_hide_community alter column when_ type timestamptz using when_;
alter table site alter column published type timestamptz using published;
alter table site alter column updated type timestamptz using updated;
alter table site alter column last_refreshed_at type timestamptz using last_refreshed_at;
alter table comment_reply alter column published type timestamptz using published;
alter table person_post_aggregates alter column published type timestamptz using published;
alter table private_message_report alter column published type timestamptz using published;
alter table private_message_report alter column updated type timestamptz using updated;
alter table local_site alter column published type timestamptz using published;
alter table local_site alter column updated type timestamptz using updated;
alter table federation_allowlist alter column published type timestamptz using published;
alter table federation_allowlist alter column updated type timestamptz using updated;
alter table federation_blocklist alter column published type timestamptz using published;
alter table federation_blocklist alter column updated type timestamptz using updated;
alter table local_site_rate_limit alter column published type timestamptz using published;
alter table local_site_rate_limit alter column updated type timestamptz using updated;
alter table person_follower alter column published type timestamptz using published;
alter table tagline alter column published type timestamptz using published;
alter table tagline alter column updated type timestamptz using updated;
alter table custom_emoji alter column published type timestamptz using published;
alter table custom_emoji alter column updated type timestamptz using updated;
alter table instance alter column published type timestamptz using published;
alter table instance alter column updated type timestamptz using updated;
alter table captcha_answer alter column published type timestamptz using published;